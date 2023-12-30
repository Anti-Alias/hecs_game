use std::any::Any;
use std::sync::{RwLock, Arc, RwLockReadGuard, RwLockWriteGuard};
use crate::{Asset, AssetManager};

pub(crate) type HandleId = u64;

/**
 * Shareable container of a [`Slot`], which itself may contain an [`Asset`].
 */
pub struct Handle<A: Asset> {
    pub(crate) id: HandleId,
    pub(crate) slot: Arc<RwLock<Slot<A>>>,
    pub(crate) manager: AssetManager,
}
impl<A: Asset> Handle<A> {

    /// Creates a handle in its initial loading state.
    /// To be filled out later by a [`Loader`](crate::Loader).
    pub(crate) fn loading(id: HandleId, manager: AssetManager) -> Self {
        Self {
            id,
            slot: Arc::new(RwLock::new(Slot::Loading)),
            manager
        }
    }

    /// Gets underlying slot with read-only access.
    pub fn read(&self) -> RwLockReadGuard<Slot<A>> {
        self.slot.read().unwrap()
    }

    /// Gets underlying slot with read and write access.
    pub fn write(&self) -> RwLockWriteGuard<Slot<A>> {
        self.slot.write().unwrap()
    }

    /**
     * Gets the status of the handle without providing a value.
    */
    pub fn status(&self) -> HandleStatus {
        let guard = self.slot.read().unwrap();
        let slot = &*guard;
        match slot {
            Slot::Loading   => HandleStatus::Loading,
            Slot::Loaded(_) => HandleStatus::Loaded,
            Slot::Failed    => HandleStatus::Failed,
        }
    }
}

impl<A: Asset> Drop for Handle<A> {
    fn drop(&mut self) {
        if Arc::strong_count(&self.slot) == 1 {
            self.manager.unload(self.id);
        }
    }
}

impl<A: Asset> Clone for Handle<A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            slot: self.slot.clone(),
            manager: self.manager.clone(),
        }
    }
}

impl <A: Asset> PartialEq for Handle<A> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<A: Asset> Eq for Handle<A> {}

/// A boxed dynamic wrapper around a [`Handle`].
/// Useful for homogenous collections of.
pub(crate) struct DynHandle(Box<dyn Any + Send + Sync>);
impl DynHandle {
    pub fn from_typed<A: Asset>(handle: Handle<A>) -> Self {
        Self(Box::new(handle))
    }
    pub fn to_typed<A: Asset>(&self) -> Option<&Handle<A>> {
        self.0.downcast_ref()
    }
}

/**
 * Container that stores an asset that is either loading, loaded or failed.
 */
pub enum Slot<A: Asset> {
    Loading,
    Loaded(A),
    Failed,
}

/**
 * Status of a [`Handle`].
 */
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum HandleStatus {
    Loading,
    Loaded,
    Failed,
}

impl HandleStatus {
    pub fn merge(self, other: HandleStatus) -> Self {
        match (self, other) {
            (HandleStatus::Loading, HandleStatus::Loading)  => HandleStatus::Loading,
            (HandleStatus::Loading, HandleStatus::Loaded)   => HandleStatus::Loading,
            (HandleStatus::Loading, HandleStatus::Failed)   => HandleStatus::Failed,
            (HandleStatus::Loaded, HandleStatus::Loading)   => HandleStatus::Loading,
            (HandleStatus::Loaded, HandleStatus::Loaded)    => HandleStatus::Loaded,
            (HandleStatus::Loaded, HandleStatus::Failed)    => HandleStatus::Failed,
            (HandleStatus::Failed, HandleStatus::Loading)   => HandleStatus::Failed,
            (HandleStatus::Failed, HandleStatus::Loaded)    => HandleStatus::Failed,
            (HandleStatus::Failed, HandleStatus::Failed)    => HandleStatus::Failed,
        }
    }
}