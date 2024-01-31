use crate::HashMap;
use derive_more::*;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::marker::PhantomData;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use crate::{Asset, AssetId, AssetLoader, AssetPath, AssetStorage, AssetStorageMut, DynLoader, DynStorage, Handle, InnerAssetStorage, PathHash, Protocol};

/// Responsible for loading assets in a background thread and storing them in relevant storages.
pub struct AssetManager {
    protocols: HashMap<String, Arc<dyn Protocol>>,
    default_protocol: Option<String>,
    loaders: Vec<Arc<dyn DynLoader>>,
    extension_to_loader: HashMap<String, usize>,
    asset_storages: HashMap<TypeId, Box<dyn DynStorage>>,
    asset_metas: HashMap<AssetId, AssetMeta>,
    path_to_asset: HashMap<PathHash, AssetId>,
    sender: Sender<AssetMessage>,
    receiver: Receiver<AssetMessage>,
}

impl AssetManager {

    pub fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            protocols: HashMap::default(),
            default_protocol: None,
            loaders: Vec::default(),
            extension_to_loader: HashMap::default(),
            asset_storages: HashMap::default(),
            asset_metas: HashMap::default(),
            path_to_asset: HashMap::default(),
            sender,
            receiver,
        }
    }

    /// Adds an asset storage for the specified asset type.
    pub fn add_storage<A: Asset>(&mut self) {
        let asset_type = TypeId::of::<A>();
        self.asset_storages
            .entry(asset_type)
            .or_insert_with(|| Box::new(RefCell::new(InnerAssetStorage::<A>::default())));
    }

    /// Adds a protocol for use in loading bytes for asset loaders.
    pub fn add_protocol(&mut self, protocol: impl Protocol, is_default: bool) {
        let name = String::from(protocol.name());
        self.protocols.insert(name.clone(), Arc::new(protocol));
        if is_default {
            self.default_protocol = Some(name);
        }
    }

    /// Adds a loader for transforming file bytes into assets.
    pub fn add_loader(&mut self, loader: impl AssetLoader) -> Result<(), LoadError> {
        for extension in loader.extensions() {
            if self.extension_to_loader.contains_key(*extension) {
                return Err(LoadError::ExtensionOverlaps);
            }
        }
        let loader_index = self.loaders.len();
        for extension in loader.extensions() {
            self.extension_to_loader.insert(String::from(*extension), loader_index);
        }
        self.loaders.push(Arc::new(loader));
        Ok(())
    }

    /// Inserts an asset manually, and returns a handle to it.
    pub fn insert<A: Asset>(&mut self, asset: A) -> Handle<A> {
        self.storage_mut::<A>().unwrap().insert(asset)
    }

    /// Gets asset storage
    pub fn storage<A: Asset>(&self) -> Option<AssetStorage<A>> {
        let asset_type = TypeId::of::<A>();
        let dyn_storage = self.asset_storages.get(&asset_type)?;
        let inner_cell = dyn_storage
            .as_any()
            .downcast_ref::<RefCell<InnerAssetStorage<A>>>()
            .unwrap();
        Some(AssetStorage {
            inner: inner_cell.borrow_mut(),
            sender: &self.sender,
        })
    }

    /// Gets asset storage
    pub fn storage_mut<A: Asset>(&mut self) -> Option<AssetStorageMut<'_, A>> {
        let asset_type = TypeId::of::<A>();
        let dyn_storage = self.asset_storages.get(&asset_type)?;
        let inner_cell = dyn_storage
            .as_any()
            .downcast_ref::<RefCell<InnerAssetStorage<A>>>()
            .unwrap();
        Some(AssetStorageMut {
            inner: inner_cell.borrow_mut(),
            metas: &mut self.asset_metas,
            sender: &mut self.sender,
        })
    }

    /// Loads an asset in the background, and returns a handle.
    /// Contents of handle can be fetched from underlying storage once loading finishes.
    pub fn load<A, P>(&mut self, path: P) -> Handle<A>
    where
        A: Asset,
        P: AsRef<str>,
    {
        self.try_load(path).unwrap()
    }

    /// Loads an asset in the background, and returns a handle.
    /// Contents of handle can be fetched from underlying storage once loading finishes.
    /// Assumes that path_hash is the correct hash of path.
    pub fn fast_load<A: Asset>(&mut self, path: &str, path_hash: PathHash) -> Handle<A> {
        self.try_fast_load(path, path_hash).unwrap()
    }

    /// Loads an asset in the background, and returns a handle.
    /// Contents of handle can be fetched from underlying storage once loading finishes.
    pub fn try_load<A, P>(&mut self, path: P) -> Result<Handle<A>, LoadError>
    where
        A: Asset,
        P: AsRef<str>,
    {
        let path = path.as_ref();
        let path_hash = PathHash::of(path);
        self.try_fast_load(path, path_hash)
    }

    /// Loads an asset in the background, and returns a handle.
    /// Contents of handle can be fetched from underlying storage once loading finishes.
    /// Assumes that path_hash is the hash of path.
    pub fn try_fast_load<A: Asset>(&mut self, path: &str, path_hash: PathHash) -> Result<Handle<A>, LoadError> {
        
        // Returns cloned handle if already stored.
        let asset_type = TypeId::of::<A>();
        if let Some(asset_id) = self.path_to_asset.get(&path_hash) {
            if asset_id.asset_type != asset_type {
                return Err(LoadError::IncorrectAssetType);
            }
            let asset_meta = self.asset_metas.get_mut(asset_id).unwrap();
            asset_meta.ref_count += 1;
            return Ok(Handle::new(*asset_id, self.sender.clone()));
        }

        // Parses path, and uses it to fetch protocol and loader.
        let path = AssetPath::parse(path, self.default_protocol.as_deref())?;
        let protocol = match self.protocols.get_mut(&path.protocol) {
            Some(protocol) => protocol.clone(),
            None => return Err(LoadError::NoSuchProtocol),
        };
        let loader = match self.extension_to_loader.get(&path.extension) {
            Some(loader_idx) => self.loaders[*loader_idx].clone(),
            None => return Err(LoadError::NoSuchLoader),
        };
        
        // Inserts new handle in "loading" state.
        let dyn_storage = match self.asset_storages.get_mut(&asset_type) {
            Some(dyn_storage) => dyn_storage,
            None => return Err(LoadError::NoSuchStorage),
        };
        let asset_id = AssetId { asset_type, index: dyn_storage.insert_loading() };
        self.path_to_asset.insert(path_hash, asset_id);
        self.asset_metas.insert(asset_id, AssetMeta {
            path_hash: Some(path_hash),
            ref_count: 1,
        });

        // Loads asset in background thread.
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let bytes = match protocol.read(&path) {
                Ok(asset_bytes) => asset_bytes,
                Err(err) => {
                    log::error!("{err}");
                    let _ = sender.send(AssetMessage::AssetFailedLoading(asset_id));
                    return;
                },
            };
            let dyn_asset = match loader.dyn_load(&bytes, &path) {
                Ok(dyn_asset) => dyn_asset,
                Err(err) => {
                    log::error!("{err}");
                    let _ = sender.send(AssetMessage::AssetFailedLoading(asset_id));
                    return;
                },
            };
            let _ = sender.send(AssetMessage::AssetFinishedLoading(asset_id, dyn_asset));
        });

        Ok(Handle {
            id: asset_id,
            sender: self.sender.clone(),
            phantom: PhantomData,
        })
    }

    /// Handles messages enqueued in storages.
    /// This finishes loading assets that were loading in the background.
    /// This discards assets that have no more references.
    /// Acts as a sort of "garbage-collection" phase where the the user specifies when it runs.
    pub fn try_handle_messages(&mut self) -> u32 {
        let mut count = 0;
        for message in self.receiver.try_iter() {
            count += 1;
            match message {
                AssetMessage::HandleCreated(asset_id) => {
                    self.asset_metas.insert(asset_id, AssetMeta {
                        path_hash: None,
                        ref_count: 1,
                    });
                }
                AssetMessage::HandleCloned(asset_id) => {
                    let asset_meta = self.asset_metas.get_mut(&asset_id).unwrap();
                    asset_meta.ref_count += 1;
                },
                AssetMessage::HandleDropped(asset_id) => {
                    let mut asset_meta_entry = match self.asset_metas.entry(asset_id) {
                        Entry::Occupied(asset_meta) => asset_meta,
                        Entry::Vacant(_) => panic!("Asset entry not found"),
                    };
                    let asset_meta = asset_meta_entry.get_mut();
                    asset_meta.ref_count -= 1;
                    if asset_meta.ref_count == 0 {
                        let storage = self.asset_storages.get_mut(&asset_id.asset_type).unwrap();
                        storage.remove(asset_id.index);
                        if let Some(path_hash) = asset_meta.path_hash {
                            self.path_to_asset.remove(&path_hash);
                        }
                        asset_meta_entry.remove();
                    }
                },
                AssetMessage::AssetFinishedLoading(asset_id, dyn_asset) => {
                    let storage = self.asset_storages.get_mut(&asset_id.asset_type).unwrap();
                    storage.finish_loading(asset_id.index, dyn_asset);
                },
                AssetMessage::AssetFailedLoading(asset_id) => {
                    let storage = self.asset_storages.get_mut(&asset_id.asset_type).unwrap();
                    storage.fail_loading(asset_id.index);
                },
            }
        }
        count
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}


pub(crate) enum AssetMessage {
    HandleCreated(AssetId),
    HandleCloned(AssetId),
    HandleDropped(AssetId),
    AssetFailedLoading(AssetId),
    AssetFinishedLoading(AssetId, Box<dyn Any + Send + Sync + 'static>),
}

#[derive(Error, Debug, Display, Clone, Eq, PartialEq)]
pub enum LoadError {
    #[display(fmt="Incorrect asset type")]
    IncorrectAssetType,
    #[display(fmt="Asset storage not found")]
    NoSuchStorage,
    #[display(fmt="No default protocol")]
    NoDefaultProtocol,
    #[display(fmt="No such protocol")]
    NoSuchProtocol,
    #[display(fmt="No loader matching extension")]
    NoSuchLoader,
    #[display(fmt="Path missing extension")]
    PathMissingExtension,
    #[display(fmt="Supported extension of one loader overlaps with another")]
    ExtensionOverlaps,
}

#[derive(Debug)]
pub(crate) struct AssetMeta {
    pub path_hash: Option<PathHash>,
    pub ref_count: u32,
}

// #[cfg(test)]
// mod test {
//     use std::time::Duration;
//     use game_macros::load;
//     use serde_yaml::Value;
//     use crate::plugins::asset::*;

//     struct YmlLoader;
//     impl AssetLoader for YmlLoader {
//         type AssetType = Value;
//         fn load(&self, bytes: &[u8], _path: &AssetPath) -> anyhow::Result<Self::AssetType> {
//             let string = std::str::from_utf8(bytes)?;
//             let result: Value = serde_yaml::from_str(string)?;
//             Ok(result)
//         }
//         fn extensions(&self) -> &[&str] {
//             &["yml"]
//         }
//     }


//     #[test]
//     fn test_manual() {

//         // Creates asset manager with String asset storage.
//         let mut manager = AssetManager::new();
//         manager.add_storage::<&'static str>();

//         // Inserts asset manually and assert's they're loaded
//         let mut storage = manager.storage_mut::<&'static str>().unwrap();
//         assert_eq!(0, storage.len());
//         let handle_original = storage.insert("string");
//         assert_eq!(1, storage.len());
//         assert_eq!(AssetState::Loaded(&"string"), storage.get(&handle_original));
//         let handle_clone = handle_original.clone();
//         manager.try_handle_messages();

//         // Asserts handle's clone is loaded.
//         let storage = manager.storage::<&'static str>().unwrap();
//         assert_eq!(AssetState::Loaded(&"string"), storage.get(&handle_clone));
//         manager.try_handle_messages();

//         // Drops clone. Asserts original is loaded.
//         drop(handle_clone);
//         let storage = manager.storage::<&'static str>().unwrap();
//         assert_eq!(AssetState::Loaded(&"string"), storage.get(&handle_original));
//         manager.try_handle_messages();

//         // Drops original. Asserts there are none loaded.
//         drop(handle_original);
//         let storage = manager.storage::<&'static str>().unwrap();
//         assert_eq!(1, storage.len());
//         manager.try_handle_messages();

//         // After final drop, storage should be empty
//         let storage = manager.storage::<&'static str>().unwrap();
//         assert_eq!(0, storage.len());
//     }

//     #[test]
//     fn test_load() {

//         // Creates asset manager that loads a yml "file".
//         let mut manager = AssetManager::new();
//         manager.add_protocol(RawProtocol::from("{ name: steve, age: 22 }"), false);
//         manager.add_storage::<Value>();
//         manager.add_loader(YmlLoader).unwrap();
//         let handle: Handle<Value> = load!(manager, "raw://not_real_file.yml");

//         // Loads asset and asserts that it's in a "loading" state.
//         let storage = manager.storage::<Value>().unwrap();
//         let state = storage.get(&handle);
//         assert_eq!(true, state.is_loading());
//         assert_eq!(false, state.is_loaded());
//         assert_eq!(false, state.is_failed());

//         // Waits for asset to leave "loading" state.
//         let mut attempts = 10;
//         loop {
//             manager.try_handle_messages();
//             let storage = manager.storage::<Value>().unwrap();
//             let state = storage.get(&handle);
//             if !state.is_loading() { break }
//             attempts -= 1;
//             if attempts == 0 {
//                 panic!("Failed to finish loading");
//             }
//             std::thread::sleep(Duration::from_millis(100));
//         }

//         // Verifies that asset is "loaded".
//         let storage = manager.storage::<Value>().unwrap();
//         let state = storage.get(&handle);
//         assert_eq!(false, state.is_loading());
//         assert_eq!(true, state.is_loaded());
//         assert_eq!(false, state.is_failed());

//         // Checks that panics don't occur from cloning and dropping handles.
//         let handle2 = handle.clone();
//         manager.try_handle_messages();
//         drop(handle);
//         manager.try_handle_messages();
//         drop(handle2);
//         manager.try_handle_messages();
//     }

//     #[test]
//     fn test_load_failure() {

//         // Creates asset manager that loads a yml "file".
//         let mut manager = AssetManager::new();
//         manager.add_protocol(RawProtocol::from("\"invalid yaml"), false);
//         manager.add_storage::<Value>();
//         manager.add_loader(YmlLoader).unwrap();
//         let handle: Handle<Value> = load!(manager, "raw://not_real_file.yml");

//         // Loads asset and asserts that it's in a "loading" state.
//         let storage = manager.storage::<Value>().unwrap();
//         let state = storage.get(&handle);
//         assert_eq!(true, state.is_loading());
//         assert_eq!(false, state.is_loaded());
//         assert_eq!(false, state.is_failed());

//         // Waits for asset to leave "loading" state.
//         let mut attempts = 10;
//         loop {
//             manager.try_handle_messages();
//             let storage = manager.storage::<Value>().unwrap();
//             let state = storage.get(&handle);
//             if !state.is_loading() { break }
//             attempts -= 1;
//             if attempts == 0 {
//                 panic!("Failed to finish loading");
//             }
//             std::thread::sleep(Duration::from_millis(100));
//         }

//         // Verifies that asset is "failed".
//         let storage = manager.storage::<Value>().unwrap();
//         let state = storage.get(&handle);
//         assert_eq!(false, state.is_loading());
//         assert_eq!(false, state.is_loaded());
//         assert_eq!(true, state.is_failed());
//     }

//     #[test]
//     fn test_load_default_protocol() {

//         // Creates asset manager that loads a yml "file".
//         let mut manager = AssetManager::new();
//         manager.add_protocol(RawProtocol::from("{ yaml: true, json: false }"), true);
//         manager.add_storage::<Value>();
//         manager.add_loader(YmlLoader).unwrap();
//         let handle: Handle<Value> = load!(manager, "not_real_file.yml");

//         // Loads asset and asserts that it's in a "loading" state.
//         let storage = manager.storage::<Value>().unwrap();
//         let state = storage.get(&handle);
//         assert_eq!(true, state.is_loading());
//         assert_eq!(false, state.is_loaded());
//         assert_eq!(false, state.is_failed());

//         // Waits for asset to leave "loading" state.
//         let mut attempts = 10;
//         loop {
//             manager.try_handle_messages();
//             let storage = manager.storage::<Value>().unwrap();
//             let state = storage.get(&handle);
//             if !state.is_loading() { break }
//             attempts -= 1;
//             if attempts == 0 {
//                 panic!("Failed to finish loading");
//             }
//             std::thread::sleep(Duration::from_millis(100));
//         }

//         // Verifies that asset is "loaded".
//         let storage = manager.storage::<Value>().unwrap();
//         let state = storage.get(&handle);
//         assert_eq!(false, state.is_loading());
//         assert_eq!(true, state.is_loaded());
//         assert_eq!(false, state.is_failed());
//     }
// }