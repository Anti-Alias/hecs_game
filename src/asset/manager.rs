use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use derive_more::{Error, Display};
use crate::{Handle, Protocol, PathParts, Asset, Dependencies, DynLoader, Loader, HandleId, DynHandle};

/**
 * Central location for loading [`Asset`]s located in files.
 */
pub struct AssetManager(Arc<RwLock<Store>>);

impl Clone for AssetManager {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl AssetManager {

    pub fn builder() -> AssetManagerBuilder {
        AssetManagerBuilder(Store {
            base_path: "assets".to_string(),
            default_protocol: None,
            protocols: HashMap::new(),
            handles: HashMap::new(),
            extensions_to_loaders: HashMap::new(),
            loaders: Vec::new(),
        })
    }

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
            let path_parts = PathParts::parse(path.as_ref(), store.default_protocol.as_deref())?;
            let handle_id = fxhash::hash64(&path_parts);
            if let Some(dyn_handle) = store.get_handle(handle_id) {
                let handle = Handle::<A>::from_dyn(handle_id, dyn_handle.clone(), self.clone());
                return Ok(handle);
            }
            let protocol = store.get_protocol(&path_parts.protocol)?;
            let dyn_loader = store.get_loader(&path_parts.extension)?;
            let handle = Handle::<A>::new(handle_id, self.clone());
            store.insert_handle(handle_id, handle.to_dyn());
            (handle, protocol, path_parts, dyn_loader)
        };

        // Handle handle in the background.
        let t_handle = handle.clone();
        let dependencies = Dependencies(self.clone());
        std::thread::spawn(move || {
            let bytes = match protocol.read(&path_parts.body) {
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
    pub(crate) fn remove_handle(&self, handle_id: HandleId) {
        let mut store = self.0.write().unwrap();
        store.handles.remove(&handle_id);
    }
}

pub struct AssetManagerBuilder(Store);
impl AssetManagerBuilder {

    pub fn base_path(mut self, base_path: impl Into<String>) -> Self {
        self.0.base_path = base_path.into();
        self
    }

    pub fn protocol(mut self, protocol: impl Protocol) -> Self {
        self.0.protocols.insert(protocol.name().into(), Arc::new(protocol));
        self
    }

    pub fn default_protocol(mut self, protocol: impl Protocol) -> Self {
        let name = protocol.name().to_string();
        self.0.protocols.insert(name.clone(), Arc::new(protocol));
        self.0.default_protocol = Some(name);
        self
    }

    pub fn loader<L: Loader>(mut self, loader: L) -> Self {
        let inner_loader = move |bytes: &[u8], extension: &str, dependencies: Dependencies| -> anyhow::Result<Box<dyn Asset>> {
            let asset = loader.load(bytes, extension, dependencies)?;
            let b: Box<dyn Asset> = Box::new(asset);
            Ok(b)
        };
        self.0.loaders.push(Arc::new(inner_loader));
        let loader_idx = self.0.loaders.len();
        for extension in L::EXTENSIONS {
            self.0.extensions_to_loaders.insert(extension.to_lowercase(), loader_idx);
        }
        self
    }

    pub fn build(self) -> AssetManager {
        AssetManager(Arc::new(RwLock::new(self.0)))
    }
}

/**
 * Underlying storage of [`AssetManager`].
 */
struct Store {
    base_path:              String,
    default_protocol:       Option<String>,
    protocols:              HashMap<String, Arc<dyn Protocol>>,
    handles:                HashMap<HandleId, DynHandle>,
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