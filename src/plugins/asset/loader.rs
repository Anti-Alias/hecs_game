use std::any::Any;
use crate::{Asset, AssetManager, Handle, LoadError};

/**
 * Responsible for loading an [`Asset`] in a background thread.
 */
pub trait Loader: Send + Sync + 'static {
    type AssetType: Asset;
    const EXTENSIONS: &'static [&'static str];
    fn load(&self, bytes: &[u8], extension: &str, dependencies: Dependencies) -> anyhow::Result<Self::AssetType>;
}

/**
 * Dynamic wrapper around [`Loader`].
 */
pub(crate) trait DynLoader: Send + Sync + 'static {
    fn load(&self, bytes: &[u8], extension: &str, dependencies: Dependencies) -> anyhow::Result<Box<dyn Any>>;
}

impl<F, A> DynLoader for F
where
    F: Fn(&[u8], &str, Dependencies) -> anyhow::Result<A> + Send + Sync + 'static,
    A: Asset,
{
    fn load(&self, bytes: &[u8], extension: &str, dependencies: Dependencies) -> anyhow::Result<Box<dyn Any>> {
        let asset = self(bytes, extension, dependencies)?;
        Ok(Box::new(asset))
    }
}

/// Allows for a [`Loader`] to load other assets that the resulting asset is dependent on.
pub struct Dependencies(pub(crate) AssetManager);
impl Dependencies {
    pub fn load<A: Asset>(&self, path: impl AsRef<str>) -> Result<Handle<A>, LoadError> {
        return self.0.try_load(path)
    }
}

// Define an event trait
trait Event {}

// Define the EventHandler trait
trait EventHandler<E: Event> {
    fn handle(&self, event: E);
}

// Implement EventHandler for functions that take the event as an argument
impl<F, E: Event> EventHandler<E> for F
where
    F: Fn(E),
{
    fn handle(&self, event: E) {
        self(event);
    }
}
