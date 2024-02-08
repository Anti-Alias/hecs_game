use crate::{AssetState, DynAssetValue, HashMap, Readiness};
use derive_more::*;
use std::any::TypeId;
use std::cell::{Ref, RefMut, RefCell};
use std::collections::hash_map::Entry;
use std::marker::PhantomData;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use crate::{Asset, AssetId, AssetLoader, AssetPath, AssetStorage, DynLoader, DynStorage, Handle, InnerAssetStorage, PathHash, Protocol};

/// Responsible for loading assets in a background thread and storing them in relevant storages.
pub struct AssetManager {
    path_prefix: Option<String>,
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
            path_prefix: None,
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

    pub fn set_path_prefix<S: Into<String>>(&mut self, prefix: Option<S>) {
        self.path_prefix = prefix.map(|s| s.into());
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
    pub fn add_loader(&mut self, loader: impl AssetLoader) {
        self.try_add_loader(loader).unwrap();
    }

    /// Adds a loader for transforming file bytes into assets.
    pub fn try_add_loader(&mut self, loader: impl AssetLoader) -> Result<(), LoadError> {
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
    pub fn insert<A: Asset>(&self, asset: A) -> Handle<A> {
        self.storage::<A>().insert(asset)
    }

    /// Gets the readiness of a handle.
    pub fn readiness_of<A: Asset>(&self, handle: &Handle<A>) -> Readiness {
        let storage = self.storage::<A>();
        let state = storage.get(handle);
        match state {
            AssetState::Loading => return Readiness::NotReady,
            AssetState::Loaded(asset) => asset.readiness(self),
            AssetState::Failed => return Readiness::Failed,
        }
    }

    /// Gets the merged readiness of a set of handles.
    pub fn readiness_all<'a, A: Asset>(&self, handles: impl IntoIterator<Item = &'a Handle<A>>) -> Readiness {
        let storage = self.storage::<A>();
        let mut readiness = Readiness::Ready;
        for handle in handles {
            let state = storage.get(handle);
            let asset = match state {
                AssetState::Loaded(asset) => asset,
                AssetState::Loading => return Readiness::NotReady,
                AssetState::Failed => return Readiness::Failed,
            };
            readiness = readiness.merge(asset.readiness(self));
        }
        readiness
    }

    /// Gets asset using specified handle.
    /// Underlying storage is "read-locked".
    pub fn get<'a, A: Asset>(&'a self, handle: &Handle<A>) -> Ref<'a, AssetState<A>> {
        let asset_type = TypeId::of::<A>();
        let dyn_storage = self.asset_storages.get(&asset_type).unwrap();
        let storage_cell = dyn_storage
            .as_any()
            .downcast_ref::<RefCell<InnerAssetStorage<A>>>()
            .unwrap()
            .borrow();
        Ref::map(storage_cell, |storage| {
            storage.get(handle.id.index).unwrap()
        })
    }

    /// Gets asset using specified handle.
    /// Underlying storage is "write-locked".
    pub fn get_mut<'a, A: Asset>(&'a self, handle: &Handle<A>) -> RefMut<'a, AssetState<A>> {
        let asset_type = TypeId::of::<A>();
        let dyn_storage = self.asset_storages.get(&asset_type).unwrap();
        let storage_cell = dyn_storage
            .as_any()
            .downcast_ref::<RefCell<InnerAssetStorage<A>>>()
            .unwrap()
            .borrow_mut();
        RefMut::map(storage_cell, |storage| {
            storage.get_mut(handle.id.index).unwrap()
        })
    }

    /// Gets asset storage for specified asset type.
    /// Underlying storage is "read-locked".
    pub fn storage<A: Asset>(&self) -> AssetStorage<A> {
        self.try_storage().unwrap()
    }

    /// Gets asset storage for specified asset type.
    /// Underlying storage is "read-locked".
    pub fn try_storage<A: Asset>(&self) -> Option<AssetStorage<A>> {
        let asset_type = TypeId::of::<A>();
        let dyn_storage = self.asset_storages.get(&asset_type)?;
        let storage_cell = dyn_storage
            .as_any()
            .downcast_ref::<RefCell<InnerAssetStorage<A>>>()
            .unwrap();
        Some(AssetStorage {
            inner: storage_cell.borrow_mut(),
            sender: &self.sender,
        })
    }

    /// Loads an asset in the background, and returns a handle.
    /// Contents of handle can be fetched from underlying storage once loading finishes.
    pub fn load<A: Asset>(&self, path: impl AsRef<str>) -> Handle<A> {
        self.try_load(path).unwrap()
    }

    /// Loads an asset in the background, and returns a handle.
    /// Contents of handle can be fetched from underlying storage once loading finishes.
    /// Assumes that path_hash is the correct hash of path.
    pub fn fast_load<A: Asset>(&self, path: &str, path_hash: PathHash) -> Handle<A> {
        self.try_fast_load(path, path_hash).unwrap()
    }

    /// Loads an asset in the background, and returns a handle.
    /// Contents of handle can be fetched from underlying storage once loading finishes.
    pub fn try_load<A, P>(&self, path: P) -> Result<Handle<A>, LoadError>
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
    pub fn try_fast_load<A: Asset>(&self, path: &str, path_hash: PathHash) -> Result<Handle<A>, LoadError> {
        
        // Returns cloned handle if already stored.
        let asset_type = TypeId::of::<A>();
        if let Some(asset_id) = self.path_to_asset.get(&path_hash) {
            if asset_id.asset_type != asset_type {
                return Err(LoadError::IncorrectAssetType);
            }
            let _ = self.sender.send(AssetMessage::HandleCloned(*asset_id));
            return Ok(Handle::new(*asset_id, self.sender.clone()));
        }

        // Parses path, and uses it to fetch protocol and loader.
        let mut path = AssetPath::parse(path, self.default_protocol.as_deref())?;
        path.prefix = self.path_prefix.clone();
        let protocol = match self.protocols.get(&path.protocol) {
            Some(protocol) => protocol.clone(),
            None => return Err(LoadError::NoSuchProtocol),
        };
        let loader = match self.extension_to_loader.get(&path.extension) {
            Some(loader_idx) => self.loaders[*loader_idx].clone(),
            None => return Err(LoadError::NoSuchLoader),
        };
        
        // Inserts new handle in "loading" state.
        let dyn_storage = match self.asset_storages.get(&asset_type) {
            Some(dyn_storage) => dyn_storage,
            None => return Err(LoadError::NoSuchStorage),
        };
        let asset_id = AssetId { asset_type, index: dyn_storage.insert_loading() };
        let _ = self.sender.send(AssetMessage::HandleCreated { asset_id, path_hash: Some(path_hash) });

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
            let dyn_asset_value = match loader.dyn_load(&bytes, &path) {
                Ok(dyn_asset) => dyn_asset,
                Err(err) => {
                    log::error!("{err}");
                    let _ = sender.send(AssetMessage::AssetFailedLoading(asset_id));
                    return;
                },
            };
            let _ = sender.send(AssetMessage::AssetFinishedLoading { asset_id, dyn_asset_value });
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
                AssetMessage::HandleCreated { asset_id, path_hash } => {
                    self.asset_metas.insert(asset_id, AssetMeta {
                        path_hash,
                        ref_count: 1,
                    });
                    if let Some(path_hash) = path_hash {
                        self.path_to_asset.insert(path_hash, asset_id);
                    }
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
                        let storage = self.asset_storages.get(&asset_id.asset_type).unwrap();
                        storage.remove(asset_id.index);
                        if let Some(path_hash) = asset_meta.path_hash {
                            self.path_to_asset.remove(&path_hash);
                        }
                        asset_meta_entry.remove();
                    }
                },
                AssetMessage::AssetFinishedLoading { asset_id, mut dyn_asset_value } => {
                    let storage = self.asset_storages.get(&asset_id.asset_type).unwrap();
                    let dyn_asset = dyn_asset_value.produce(self);
                    storage.finish(asset_id.index, dyn_asset);
                },
                AssetMessage::AssetFailedLoading(asset_id) => {
                    let storage = self.asset_storages.get(&asset_id.asset_type).unwrap();
                    storage.fail(asset_id.index);
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
    HandleCreated {
        asset_id: AssetId,
        path_hash: Option<PathHash>,
    },
    HandleCloned(AssetId),
    HandleDropped(AssetId),
    AssetFailedLoading(AssetId),
    AssetFinishedLoading {
        asset_id: AssetId,
        dyn_asset_value: Box<dyn DynAssetValue>,
    },
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