use std::any::Any;
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::sync::{RwLock, Arc, RwLockReadGuard, RwLockWriteGuard};
use std::hash::{Hash, Hasher};
use derive_more::*;
use uuid::Uuid;
use crate::{Asset, AssetManager};


/**
 * Dynamic variant of [`Handle`] that is stored within an [`AssetManager`].
 */
#[derive(Clone)]
pub(crate) struct DynHandle(pub(crate) Arc<RwLock<DynSlot>>);

/**
 * Dynamic variant of [`Slot`].
 */
pub(crate) enum DynSlot {
    Loading,
    Loaded(Box<dyn Any + Send + Sync>),
    Failed,
}

impl DynSlot {
    pub fn status(&self) -> HandleStatus {
        match self {
            DynSlot::Loading => HandleStatus::Loaded,
            DynSlot::Loaded(_) => HandleStatus::Loaded,
            DynSlot::Failed => HandleStatus::Failed,
        }
    }
}

/**
 * Shareable container of a [`Slot`], which itself may contain an [`Asset`].
 */
pub struct Handle<A: Asset> {
    variant: HandleVariant,     // Managed/unmanaged specific data
    dyn_handle: DynHandle,      // Underlying dynamic handle
    phantom: PhantomData<A>,    // ZST marker
}
impl<A: Asset> Handle<A> {

    /// Creates an "unmanaged" handle with contents created programmatically.
    pub fn new(asset: A) -> Self {
        Self {
            variant: HandleVariant::Unmanaged { id: Uuid::new_v4() },
            dyn_handle: DynHandle(Arc::new(RwLock::new(DynSlot::Loaded(Box::new(asset))))),
            phantom: PhantomData,
        }
    }

    /// Creates a "managed" handle in its initial loading state.
    /// To be filled out later by a [`Loader`](crate::Loader).
    pub(crate) fn loading(id: u64, manager: AssetManager) -> Self {
        Self {
            variant: HandleVariant::Managed {
                id,
                manager,
            },
            dyn_handle: DynHandle(Arc::new(RwLock::new(DynSlot::Loading))),
            phantom: PhantomData,
        }
    }

    /// Gets underlying slot with read access.
    pub fn slot(&self) -> Slot<'_, A> {
        Slot {
            dyn_slot: self.dyn_handle.0.read().unwrap(),
            phantom: PhantomData,
        }
    }
    
    /// Gets underlying slot with write access.
    pub fn slot_mut(&self) -> SlotMut<'_, A> {
        SlotMut {
            dyn_slot: self.dyn_handle.0.write().unwrap(),
            phantom: PhantomData,
        }
    }

    /**
     * Gets the status of the handle without providing a value.
    */
    pub fn status(&self) -> HandleStatus {
        self.slot().status()
    }

    pub(crate) fn fail(&self) {
        let mut dyn_slot = self.dyn_handle.0.write().unwrap();
        *dyn_slot = DynSlot::Failed;
    }

    pub(crate) fn finish(&self, asset: A) {
        let mut dyn_slot = self.dyn_handle.0.write().unwrap();
        *dyn_slot = DynSlot::Loaded(Box::new(asset));
    }

    pub(crate) fn from_dyn(id: u64, dyn_handle: DynHandle, manager: AssetManager) -> Self {
        Self {
            variant: HandleVariant::Managed { id, manager },
            dyn_handle,
            phantom: PhantomData,
        }
    }

    pub(crate) fn to_dyn(&self) -> DynHandle {
        self.dyn_handle.clone()
    }

    fn strong_count(&self) -> usize {
        Arc::strong_count(&self.dyn_handle.0)
    }
}

impl<A: Asset> Drop for Handle<A> {
    fn drop(&mut self) {
        // If count is 2, only this Handle and the AssetManager are keeping track of the asset.
        // AssetManager should remove the handle in this case.
        if self.strong_count() == 2 {
            if let HandleVariant::Managed { id, manager } = &mut self.variant {
                manager.remove_handle(*id);
            }
        }
    }
}

impl<A: Asset> Clone for Handle<A> {
    fn clone(&self) -> Self {
        Self {
            variant: self.variant.clone(),
            dyn_handle: self.dyn_handle.clone(),
            phantom: PhantomData,
        }
    }
}

impl <A: Asset> PartialEq for Handle<A> {
    fn eq(&self, other: &Self) -> bool {
        match (&self.variant, &other.variant) {
            (HandleVariant::Managed { id, .. }, HandleVariant::Managed { id: other_id, .. })    => id == other_id,
            (HandleVariant::Managed { .. }, HandleVariant::Unmanaged { .. })                    => false,
            (HandleVariant::Unmanaged { .. }, HandleVariant::Managed { .. })                    => false,
            (HandleVariant::Unmanaged { id }, HandleVariant::Unmanaged { id: other_id })        => id == other_id,
        }
    }
}

impl<A: Asset> Eq for Handle<A> {}

impl<A: Asset> PartialOrd for Handle<A> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (&self.variant, &other.variant) {
            (HandleVariant::Managed { id, .. }, HandleVariant::Managed { id: other_id, .. })    => id.partial_cmp(other_id),
            (HandleVariant::Managed { .. }, HandleVariant::Unmanaged { .. })                    => Some(Ordering::Less),
            (HandleVariant::Unmanaged { .. }, HandleVariant::Managed { .. })                    => Some(Ordering::Greater),
            (HandleVariant::Unmanaged { id }, HandleVariant::Unmanaged { id: other_id })        => id.partial_cmp(other_id),
        }
    }
}

impl<A: Asset> Ord for Handle<A> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.variant, &other.variant) {
            (HandleVariant::Managed { id, .. }, HandleVariant::Managed { id: other_id, .. })    => id.cmp(other_id),
            (HandleVariant::Managed { .. }, HandleVariant::Unmanaged { .. })                    => Ordering::Less,
            (HandleVariant::Unmanaged { .. }, HandleVariant::Managed { .. })                    => Ordering::Greater,
            (HandleVariant::Unmanaged { id }, HandleVariant::Unmanaged { id: other_id })        => id.cmp(other_id),
        }
    }
}

impl <A: Asset> Hash for Handle<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.variant {
            HandleVariant::Managed { id, .. } => id.hash(state),
            HandleVariant::Unmanaged { id } => id.hash(state),
        }
    }
}

/**
 * Data specific to managed or unmanaged handles.
 */
#[derive(Clone)]
pub enum HandleVariant {
    /// Handle stored in an [`AssetManager`].
    Managed {
        id: u64,
        manager: AssetManager,
    },
    /// Handle created programmatically.
    Unmanaged {
        id: Uuid,
    }
}

/**
 * Read-only slot.
 */
pub struct Slot<'a, A: Asset> {
    dyn_slot: RwLockReadGuard<'a, DynSlot>,
    phantom: PhantomData<A>,
}

impl<'a, A: Asset> Slot<'a, A> {
    
    pub fn status(&self) -> HandleStatus {
        let dyn_slot = &*self.dyn_slot;
        dyn_slot.status()
    }

    pub fn value(&self) -> SlotValue<&A> {
        self.try_value().unwrap()
    }

    pub fn try_value(&self) -> Result<SlotValue<&A>, SlotError> {
        let dyn_slot = &*self.dyn_slot;
        let dyn_asset: &dyn Any = match dyn_slot {
            DynSlot::Loaded(dyn_asset) => dyn_asset,
            DynSlot::Loading => return Ok(SlotValue::Loading),
            DynSlot::Failed => return Ok(SlotValue::Failed),
        };
        let asset = dyn_asset
            .downcast_ref::<A>()
            .ok_or(SlotError::IncorrectAssetType)?;
        if asset.status() == HandleStatus::Loading {
            return Ok(SlotValue::Loading);
        }
        Ok(SlotValue::Loaded(asset))
    }
}


#[derive(Error, Debug, Display)]
pub enum SlotError {
    #[display(fmt="Incorrect asset type")]
    IncorrectAssetType,
}

/**
 * Read / write slot.
 */
pub struct SlotMut<'a, A: Asset> {
    dyn_slot: RwLockWriteGuard<'a, DynSlot>,
    phantom: PhantomData<A>,
}

impl<'a, A: Asset> SlotMut<'a, A> {
    
    pub fn status(&self) -> HandleStatus {
        let dyn_slot = &*self.dyn_slot;
        match dyn_slot {
            DynSlot::Loading => HandleStatus::Loaded,
            DynSlot::Loaded(_) => HandleStatus::Loaded,
            DynSlot::Failed => HandleStatus::Failed,
        }
    }

    pub fn value(&mut self) -> SlotValue<&mut A> {
        self.try_value().unwrap()
    }

    pub fn try_value(&mut self) -> Result<SlotValue<&mut A>, SlotError> {
        let dyn_slot = &mut *self.dyn_slot;
        let dyn_asset: &mut dyn Any = match dyn_slot {
            DynSlot::Loaded(dyn_asset) => dyn_asset,
            DynSlot::Loading => return Ok(SlotValue::Loading),
            DynSlot::Failed => return Ok(SlotValue::Failed),
        };
        let asset = dyn_asset
            .downcast_mut::<A>()
            .ok_or(SlotError::IncorrectAssetType)?;
        Ok(SlotValue::Loaded(asset))
    }
}


/**
 * The value of a [`Slot`] or [`SlotMut`].
 */
pub enum SlotValue<V> {
    Loading,
    Loaded(V),
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