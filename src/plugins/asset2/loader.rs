use std::any::{Any, TypeId};
use anyhow::Ok;
use super::{Asset, AssetPath};

/// Takes the contents of a file, and converts them into an asset.
pub trait AssetLoader: Send + Sync + 'static {
    type AssetType: Asset;
    fn load(&self, path: &AssetPath, bytes: &[u8]) -> anyhow::Result<Self::AssetType>;
    fn extensions(&self) -> &[&str];
}

/// Dynamic trait variant of [`Loader`].
pub trait DynLoader: Send + Sync + 'static {
    fn dyn_load(&self, path: &AssetPath, bytes: &[u8]) -> anyhow::Result<Box<dyn Any + Send + Sync + 'static>>;
    fn asset_type(&self) -> TypeId;
}

impl<L: AssetLoader> DynLoader for L {

    fn dyn_load(&self, path: &AssetPath, bytes: &[u8]) -> anyhow::Result<Box<dyn Any + Send + Sync + 'static>> {
        let asset = self.load(path, bytes)?;
        Ok(Box::new(asset))
    }

    fn asset_type(&self) -> TypeId {
        TypeId::of::<L::AssetType>()
    }
}