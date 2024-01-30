use std::any::{Any, TypeId};
use anyhow::Ok;
use crate::{Asset, AssetPath};

/// Takes the contents of a file, and converts them into an asset.
pub trait AssetLoader: Send + Sync + 'static {
    type AssetType: Asset;
    fn load(&self, bytes: &[u8], path: &AssetPath) -> anyhow::Result<Self::AssetType>;
    fn extensions(&self) -> &[&str];
}

/// Dynamic trait variant of [`Loader`].
pub trait DynLoader: Send + Sync + 'static {
    fn dyn_load(&self, bytes: &[u8], path: &AssetPath) -> anyhow::Result<Box<dyn Any + Send + Sync + 'static>>;
    fn asset_type(&self) -> TypeId;
}

impl<L: AssetLoader> DynLoader for L {

    fn dyn_load(&self, bytes: &[u8], path: &AssetPath) -> anyhow::Result<Box<dyn Any + Send + Sync + 'static>> {
        let asset = self.load(bytes, path)?;
        Ok(Box::new(asset))
    }

    fn asset_type(&self) -> TypeId {
        TypeId::of::<L::AssetType>()
    }
}