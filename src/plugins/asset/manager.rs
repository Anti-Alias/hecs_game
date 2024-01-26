use std::sync::{Arc, RwLock};
use derive_more::{Error, Display};
use crate::{Handle, Protocol, PathParts, Asset, Dependencies, DynLoader, Loader, DynHandle, HashMap};

/**
 * Central location for loading [`Asset`]s located in files.
 */
pub struct AssetManager(Arc<RwLock<Store>>);

impl Clone for AssetManager {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        let store = Store {
            base_path: "assets".to_string(),
            default_protocol: None,
            protocols: HashMap::default(),
            handles: HashMap::default(),
            extensions_to_loaders: HashMap::default(),
            loaders: Vec::new(),
        };
        Self(Arc::new(RwLock::new(store)))
    }
}

impl AssetManager {

    pub fn load<A: Asset>(&self, path: impl AsRef<str>) -> Handle<A> {
        self.try_load(path).unwrap()
    }

    /// Loads an asset from a file.
    /// Uses the protocol named in the path, or the default protocol if not specified.
    /// path/to/file.png            (uses default protocol. Fails if there is none).
    /// file://path/to/file.png     (uses file protocol).
    /// http://path/to/file.png     (uses http protocol)
    pub fn try_load<A: Asset>(&self, path: impl AsRef<str>) -> Result<Handle<A>, LoadError> {
        
        // Gets resources to load asset.
        // Returns early if cached handle was found.
        let (handle, protocol, path_parts, dyn_loader) = {
            let mut store = self.0.write().unwrap();
            let mut path_parts = PathParts::parse(path.as_ref(), store.default_protocol.as_deref())?;
            let handle_id = fxhash::hash64(&path_parts);
            if let Some(dyn_handle) = store.get_handle(handle_id) {
                let handle = Handle::<A>::from_dyn(handle_id, dyn_handle.clone(), self.clone());
                return Ok(handle);
            }
            path_parts.body = format!("{}/{}", store.base_path, path_parts.body);
            let protocol = store.get_protocol(&path_parts.protocol)?;
            let dyn_loader = store.get_loader(&path_parts.extension)?;
            let handle = Handle::<A>::loading(handle_id, self.clone());
            store.insert_handle(handle_id, handle.to_dyn());
            (handle, protocol, path_parts, dyn_loader)
        };

        // Handle handle in the background.
        let t_handle = handle.clone();
        let dependencies = Dependencies(self.clone());
        std::thread::spawn(move || {
            let bytes = match protocol.read(&path_parts.path()) {
                Ok(bytes) => bytes,
                Err(err) => {
                    log::error!("Failed to read bytes from {}: {}", path_parts, err);
                    t_handle.fail();
                    return;
                },
            };
            let dyn_asset = match dyn_loader.load(&bytes, &path_parts.extension, dependencies) {
                Ok(dyn_asset) => dyn_asset,
                Err(err) => {
                    log::error!("Loader failed on {}: {}", path_parts, err);
                    t_handle.fail();
                    return;
                },
            };
            let asset = match dyn_asset.downcast::<A>() {
                Ok(asset) => asset,
                Err(_) => {
                    log::error!("Incorrect asset type for {}", path_parts);
                    t_handle.fail();
                    return;
                },
            };
            t_handle.finish(*asset);
        });
        return Ok(handle);
    }

    /// Removes a handle stored internally.
    /// Called when the last [`Handle`] to an [`Asset`] gets dropped.
    pub(crate) fn remove_handle(&self, handle_id: u64) {
        let mut store = self.0.write().unwrap();
        store.handles.remove(&handle_id);
    }

    pub fn add_protocol(&self, protocol: impl Protocol, default: bool) {
        let mut store = self.0.write().unwrap();
        let protocol_name = String::from(protocol.name());
        store.protocols.insert(protocol_name.clone(), Arc::new(protocol));
        if default {
            store.default_protocol = Some(protocol_name);
        }
    }

    pub fn set_base_path(&self, base_path: impl Into<String>) {
        let mut store = self.0.write().unwrap();
        store.base_path = base_path.into();
    }

    pub fn add_loader<L: Loader>(&self, loader: L) {
        let mut store = self.0.write().unwrap();
        let inner_loader = move |bytes: &[u8], extension: &str, dependencies: Dependencies| -> anyhow::Result<Box<dyn Asset>> {
            let asset = loader.load(bytes, extension, dependencies)?;
            let b: Box<dyn Asset> = Box::new(asset);
            Ok(b)
        };
        let loader_idx = store.loaders.len();
        store.loaders.push(Arc::new(inner_loader));
        for extension in L::EXTENSIONS {
            store.extensions_to_loaders.insert(extension.to_lowercase(), loader_idx);
        }
    }
}

/**
 * Underlying storage of [`AssetManager`].
 */
struct Store {
    base_path:              String,
    default_protocol:       Option<String>,
    protocols:              HashMap<String, Arc<dyn Protocol>>,
    handles:                HashMap<u64, DynHandle>,
    extensions_to_loaders:  HashMap<String, usize>,
    loaders:                Vec<Arc<dyn DynLoader>>,
}

impl Store {
    
    /// Inserts a dynamic handle into the cache, overwriting the old one.
    fn insert_handle(&mut self, handle_id: u64, dyn_handle: DynHandle) {
        self.handles.insert(handle_id, dyn_handle);
    }

    /// Retrieves a weak handle from the cache if a match is found.
    /// Fails if cached handle is of a different type.
    fn get_handle(&self, handle_id: u64) -> Option<&DynHandle> {
        self.handles.get(&handle_id)
    }

    /// Gets a protocol by name, or the default protocol if name is not specified.
    /// Fails if not found.
    fn get_protocol(&self, name: &str) -> Result<Arc<dyn Protocol>, LoadError> {
        let Some(dyn_protocol) = self.protocols.get(name) else {
            return Err(LoadError::NoMatchingProtocol);
        };
        return Ok(dyn_protocol.clone());
    }

    /// Gets a [`Loader`] by asset type.
    fn get_loader(&self, extension: &str) -> Result<Arc<dyn DynLoader>, LoadError> {
        let idx = self.extensions_to_loaders
            .get(extension)
            .ok_or(LoadError::NoMatchingLoader)?;
        Ok(self.loaders[*idx].clone())
    }
}

/// Error that can occur when [`AssetManager`] fails during load().
#[derive(Error, Debug, Display)]
pub enum LoadError {
    #[display(fmt="Invalid path")]
    InvalidPath,
    #[display(fmt="No matching protocol")]
    NoMatchingProtocol,
    #[display(fmt="Path missing protocol, and no default protocol was available")]
    PathMissingProtocol,
    #[display(fmt="Path missing extension")]
    PathMissingExtension,
    #[display(fmt="No matching loader")]
    NoMatchingLoader,
}