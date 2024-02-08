use std::any::{Any, TypeId};
use crate::{AssetIndex, AssetManager};

/// A shareable resource that may be loaded from a file.
/// An asset with dependent assets will usually need to implement the readiness method.
pub trait Asset: Any + Send + Sync + 'static {
    fn readiness(&self, _assets: &AssetManager) -> Readiness { Readiness::Ready }
}

/// Value that can uniquely identify an asset within an asset manager.
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct AssetId {
    pub(crate) asset_type: TypeId,
    pub(crate) index: AssetIndex,
}

/**
 * If an asset is "ready", then it is loaded, and typically all of its children, if any, are "ready".
 * If at least one child is "not ready", it is typically "not ready".
 * If at least one child is "failed", it is typically "failed."
 * Typically, "not ready" has priority over "ready", and "failed" has priority over "not ready".
 */
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Readiness { Ready, NotReady, Failed }

impl Readiness {
    pub fn merge(self, other: Readiness) -> Readiness {
        match (self, other) {
            (Readiness::Ready, Readiness::Ready) => Readiness::Ready,
            (Readiness::Ready, Readiness::NotReady) => Readiness::NotReady,
            (Readiness::Ready, Readiness::Failed) => Readiness::Failed,
            (Readiness::NotReady, Readiness::Ready) => Readiness::NotReady,
            (Readiness::NotReady, Readiness::NotReady) => Readiness::NotReady,
            (Readiness::NotReady, Readiness::Failed) => Readiness::Failed,
            (Readiness::Failed, Readiness::Ready) => Readiness::Failed,
            (Readiness::Failed, Readiness::NotReady) => Readiness::Failed,
            (Readiness::Failed, Readiness::Failed) => Readiness::Failed,
        }
    }

    pub fn of_all<'a, A: Asset>(assets: impl IntoIterator<Item = &'a A>, manager: &AssetManager) -> Readiness {
        let mut readiness = Readiness::Ready;
        for asset in assets {
            readiness = readiness.merge(asset.readiness(manager));
        }
        readiness
    }
}