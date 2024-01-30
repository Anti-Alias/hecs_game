use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::sync::mpsc::Sender;
use slotmap::{new_key_type, SlotMap};
use crate::{HashMap, Asset, AssetId, AssetMessage, AssetMeta, PathHash};

/// Trait that [`AssetStorage`] must implement to be used dynamically by the [`AssetServer`].
pub(crate) trait DynStorage {
    fn insert_loading(&mut self) -> AssetIndex;
    fn finish_loading(&mut self, index: AssetIndex, asset: Box<dyn Any>);
    fn fail_loading(&mut self, index: AssetIndex);
    fn remove(&mut self, index: AssetIndex);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Simple contiguous storage of assets.
pub struct AssetStorage<'a, A> {
    pub(crate) inner: &'a InnerAssetStorage<A>,
}

impl<'a, A: Asset> AssetStorage<'a, A> {

    pub fn get(&self, handle: &Handle<A>) -> AssetState<&A> {
        self.inner.get(handle.id.index).unwrap().as_ref()
    }

    pub unsafe fn get_unchecked(&self, handle: &Handle<A>) -> AssetState<&A> {
        self.inner.get_unchecked(handle.id.index).as_ref()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

/// Simple contiguous storage of assets.
/// Informs asset manager of changes.
pub struct AssetStorageMut<'a, A> {
    pub(crate) inner: &'a mut InnerAssetStorage<A>,         // Internal storage of assets
    pub(crate) metas: &'a mut HashMap<AssetId, AssetMeta>,  // Metadata of assets
    pub(crate) paths: &'a mut HashMap<PathHash, AssetId>,   // Mapping of file paths to assets
    pub(crate) sender: &'a mut Sender<AssetMessage>,
}

impl<'a, A: Asset> AssetStorageMut<'a, A> {

    pub fn insert(&mut self, asset: A) -> Handle<A> {
        let index = self.inner.insert(AssetState::Loaded(asset));
        let id = AssetId {
            asset_type: TypeId::of::<A>(),
            index,
        };
        self.metas.insert(id, AssetMeta {
            path_hash: None,
            ref_count: 1,
        });
        Handle {
            id,
            sender: self.sender.clone(),
            phantom: PhantomData,
        }
    }

    pub fn get(&self, handle: &Handle<A>) -> AssetState<&A> {
        self.inner.get(handle.id.index).unwrap().as_ref()
    }

    pub fn get_mut(&mut self, handle: &Handle<A>) -> AssetState<&mut A> {
        self.inner.get_mut(handle.id.index).unwrap().as_mut()
    }

    pub fn get_weak(&self, handle: &WeakHandle<A>) -> Option<AssetState<&A>> {
        self.inner.get(handle.id.index).map(|state| state.as_ref())
    }

    pub fn get_weak_mut(&mut self, handle: &WeakHandle<A>) -> Option<AssetState<&mut A>> {
        self.inner.get_mut(handle.id.index).map(|state| state.as_mut())
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

/// Simple contiguous storage of assets.
pub(crate) type InnerAssetStorage<A> = SlotMap<AssetIndex, AssetState<A>>;
impl<A: Asset> DynStorage for InnerAssetStorage<A> {

    fn insert_loading(&mut self) -> AssetIndex {
        self.insert(AssetState::Loading)
    }

    fn finish_loading(&mut self, index: AssetIndex, asset: Box<dyn Any>) {
        let state = self.get_mut(index).unwrap();
        let asset = asset.downcast::<A>().unwrap();
        *state = AssetState::Loaded(*asset);
    }

    fn fail_loading(&mut self, index: AssetIndex) {
        let state = self.get_mut(index).unwrap();
        *state = AssetState::Failed;
    }
    
    fn remove(&mut self, index: AssetIndex) {
        self.remove(index);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}



#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum AssetState<A> {
    Loading,
    Loaded(A),
    Failed,
}

impl<A> AssetState<A> {

    pub fn is_loading(&self) -> bool {
        match self {
            AssetState::Loading => true,
            _ => false,
        }
    }

    pub fn is_loaded(&self) -> bool {
        match self {
            AssetState::Loaded(_) => true,
            _ => false,
        }
    }

    pub fn is_failed(&self) -> bool {
        match self {
            AssetState::Failed => true,
            _ => false,
        }
    }

    pub fn as_loaded(&self) -> Option<&A> {
        match self {
            AssetState::Loaded(asset) => Some(asset),
            _ => None,
        }
    }

    pub fn as_loaded_mut(&mut self) -> Option<&mut A> {
        match self {
            AssetState::Loaded(asset) => Some(asset),
            _ => None,
        }
    }

    pub fn as_ref(&self) -> AssetState<&A> {
        match self {
            AssetState::Loading => AssetState::<&A>::Loading,
            AssetState::Loaded(asset) => AssetState::Loaded(asset),
            AssetState::Failed => AssetState::<&A>::Failed,
        }
    }
    
    pub fn as_mut(&mut self) -> AssetState<&mut A> {
        match self {
            AssetState::Loading => AssetState::<&mut A>::Loading,
            AssetState::Loaded(asset) => AssetState::Loaded(asset),
            AssetState::Failed => AssetState::<&mut A>::Failed,
        }
    }
}

impl<A> AssetState<&A> {
    pub fn unwrap(&self) -> &A {
        match self {
            AssetState::Loading => panic!("Asset is in a loading state"),
            AssetState::Loaded(asset) => asset,
            AssetState::Failed => panic!("Asset is in a failed state"),
        }
    }
}

impl<A> AssetState<&mut A> {
    pub fn unwrap(&mut self) -> &mut A {
        match self {
            AssetState::Loading => panic!("Asset was loading"),
            AssetState::Loaded(asset) => asset,
            AssetState::Failed => panic!("Asset failed to load"),
        }
    }
}

/// Smart index into an [`AssetStorage`].
/// Used to fetch underlying asset.
pub struct Handle<A> {
    pub(crate) id: AssetId,
    pub(crate) sender: Sender<AssetMessage>,
    pub(crate) phantom: PhantomData<A>,
}

impl<A: Asset> Handle<A> {
    pub(crate) fn new(id: AssetId, sender: Sender<AssetMessage>) -> Self {
        Self {
            id,
            sender,
            phantom: PhantomData,
        }
    }

    pub fn weak(&self) -> WeakHandle<A> {
        WeakHandle {
            id: self.id,
            phantom: PhantomData,
        }
    }
}

impl<A> Handle<A> {
    pub fn id(&self) -> AssetId { self.id }
}

impl<A> Clone for Handle<A> {
    fn clone(&self) -> Self {
        let _ = self.sender.send(AssetMessage::HandleCloned(self.id));
        Self {
            id: self.id,
            sender: self.sender.clone(),
            phantom: PhantomData,
        }
    }
}

impl<A> Drop for Handle<A> {
    fn drop(&mut self) {
        let _ = self.sender.send(AssetMessage::HandleDropped(self.id));
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Debug)]
pub struct WeakHandle<A> {
    id: AssetId,
    phantom: PhantomData<A>,
}

new_key_type! {
    /**
     * ID for a [`Node`].
     */
    pub struct AssetIndex;
}