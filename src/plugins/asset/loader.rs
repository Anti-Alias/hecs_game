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
    fn produce(&mut self, manager: &AssetManager) -> Self::AssetType;
}

/// Producer that utilizes an underlying callback function.
/// Consumes it when finished.
pub enum FnAssetProducer<F, A>
where
    F: FnOnce(&AssetManager) -> A + Send + Sync + 'static,
    A: Asset,
{
    Producer(F),
    Consumed,
}

impl<F, A> AssetProducer for FnAssetProducer<F, A>
where
    F: FnOnce(&AssetManager) -> A + Send + Sync + 'static,
    A: Asset,
{
    type AssetType = A;
    fn produce(&mut self, manager: &AssetManager) -> Self::AssetType {
        let producer = std::mem::replace(self, FnAssetProducer::Consumed);
        let function = match producer {
            FnAssetProducer::Producer(function) => function,
            FnAssetProducer::Consumed => unreachable!(),
        };
        function(manager)
    }
}

impl<F, A> AssetProducer for F
where
    F: FnMut(&AssetManager) -> A + Send + Sync + 'static,
    A: Asset
{
    type AssetType = A;
    fn produce(&mut self, manager: &AssetManager) -> Self::AssetType {
        self(manager)
    }
    
}

impl<P: AssetProducer> DynAssetProducer for P {
    fn dyn_produce(&mut self, manager: &AssetManager) -> Box<dyn Any + Send + Sync + 'static> {
        let asset = self.produce(manager);
        Box::new(asset)
    }
}


/// Value returned by an [`AssetLoader`].
/// Either a plain [`Asset`], or a producer of an [`Asset`].
/// Producer runs on main thread and has access to the [`AssetManager`] for loading or inserting dependent assets.
pub struct AssetValue<A>(AssetValueInner<A>);

impl<A: Asset> AssetValue<A> {
    pub fn from_fn<F>(function: F) -> Self
    where
        F: FnOnce(&AssetManager) -> A + Send + Sync + 'static,
    {
        let dyn_producer: Box<dyn DynAssetProducer> = Box::new(FnAssetProducer::Producer(function));
        Self(AssetValueInner::Producer(dyn_producer))
    }
}

impl<A: Asset> From<A> for AssetValue<A> {
    fn from(asset: A) -> Self {
        Self(AssetValueInner::Asset(asset))
    }
}

enum AssetValueInner<A> {
    Consumed,
    Asset(A),
    Producer(Box<dyn DynAssetProducer>)
}

/// Dynamic trait variant of [`AssetLoader`].
pub(crate) trait DynLoader: Send + Sync + 'static {
    fn dyn_load(&self, bytes: &[u8], path: &AssetPath) -> anyhow::Result<Box<dyn DynAssetValue>>;
    fn asset_type(&self) -> TypeId;
}

/// Dynamic trait variant of [`AssetProducer`].
pub trait DynAssetProducer: Send + Sync + 'static {
    fn dyn_produce(&mut self, manager: &AssetManager) -> Box<dyn Any + Send + Sync + 'static>;
}

pub trait DynAssetValue: Send + Sync + 'static {
    fn produce(&mut self, manager: &AssetManager) -> Box<dyn Any + Send + Sync + 'static>;
}

impl<A: Asset> DynAssetValue for AssetValue<A> {
    fn produce(&mut self, manager: &AssetManager) -> Box<dyn Any + Send + Sync + 'static> {
        let inner = std::mem::replace(&mut self.0, AssetValueInner::Consumed);
        match inner {
            AssetValueInner::Asset(asset) => Box::new(asset),
            AssetValueInner::Producer(mut producer) => producer.dyn_produce(manager),
            _ => panic!("produce cannot be invoked multiple times")
        }
    }
}

pub type AssetResult<A> = anyhow::Result<AssetValue<A>>;