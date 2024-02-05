use std::any::{Any, TypeId};
use anyhow::Ok;
use crate::{Asset, AssetManager, AssetPath};


/// Takes the contents of a file, and converts them into an asset.
pub trait AssetLoader: Send + Sync + 'static {
    type AssetType: Asset;
    fn load(&self, bytes: &[u8], path: &AssetPath) -> anyhow::Result<AssetValue<Self::AssetType>>;
    fn extensions(&self) -> &[&str];
}

impl<L: AssetLoader> DynLoader for L {

    fn dyn_load(&self, bytes: &[u8], path: &AssetPath) -> anyhow::Result<Box<dyn DynAssetValue>> {
        let asset_value = self.load(bytes, path)?;
        Ok(Box::new(asset_value))
    }

    fn asset_type(&self) -> TypeId {
        TypeId::of::<L::AssetType>()
    }
}

/// Produces an asset using an asset manager.
pub trait AssetProducer: Send + Sync + 'static {
    type AssetType: Asset;
    fn produce(&self, manager: &AssetManager) -> Self::AssetType;
}

impl<P: AssetProducer> DynAssetProducer for P {
    fn dyn_produce(&self, manager: &AssetManager) -> Box<dyn Any + Send + Sync + 'static> {
        let asset = self.produce(manager);
        Box::new(asset)
    }
}

/// Either an asset, or a callback function that produces the asset.
pub enum AssetValue<A> {
    Consumed,
    Asset(A),
    AssetProducer(Box<dyn DynAssetProducer>)
}

impl<A> AssetValue<A> {
    pub fn from_asset(asset: A) -> Self {
        Self::Asset(asset)
    }
    pub fn from_producer<P: AssetProducer<AssetType=A>>(producer: P) -> Self {
        Self::AssetProducer(Box::new(producer))
    }
}

impl<A: Asset> From<A> for AssetValue<A> {
    fn from(asset: A) -> Self {
        Self::Asset(asset)
    }
}

impl<A: Asset> AssetValue<A> {
    pub fn set(&mut self, asset: A) {
        *self = Self::Asset(asset);
    }
    pub fn set_with(&mut self, producer: impl AssetProducer) {
        *self = Self::AssetProducer(Box::new(producer));
    }
}

/// Dynamic trait variant of [`AssetLoader`].
pub(crate) trait DynLoader: Send + Sync + 'static {
    fn dyn_load(&self, bytes: &[u8], path: &AssetPath) -> anyhow::Result<Box<dyn DynAssetValue>>;
    fn asset_type(&self) -> TypeId;
}

/// Dynamic trait variant of [`AssetProducer`].
pub trait DynAssetProducer: Send + Sync + 'static {
    fn dyn_produce(&self, manager: &AssetManager) -> Box<dyn Any + Send + Sync + 'static>;
}

pub trait DynAssetValue: Send + Sync + 'static {
    fn produce(&mut self, manager: &AssetManager) -> Box<dyn Any + Send + Sync + 'static>;
}

impl<A: Asset> DynAssetValue for AssetValue<A> {
    fn produce(&mut self, manager: &AssetManager) -> Box<dyn Any + Send + Sync + 'static> {
        let asset_value = std::mem::replace(self, AssetValue::Consumed);
        match asset_value {
            AssetValue::Asset(asset) => Box::new(asset),
            AssetValue::AssetProducer(producer) => Box::new(producer.dyn_produce(manager)),
            _ => unreachable!()
        }
    }
}

pub type AssetResult<A> = anyhow::Result<AssetValue<A>>;