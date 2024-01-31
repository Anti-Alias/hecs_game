use std::any::{Any, TypeId};
use std::cell::{RefCell, RefMut};
use std::marker::PhantomData;
use std::sync::mpsc::Sender;
use slotmap::{new_key_type, SlotMap};
use crate::{Asset, AssetId, AssetMessage, AssetMeta, HashMap, Readiness};

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
/// Informs asset manager of insertions by passing messages.
pub struct AssetStorage<'a, A> {
    pub(crate) inner: RefMut<'a, InnerAssetStorage<A>>,
    pub(crate) sender: &'a Sender<AssetMessage>,
}

impl<'a, A: Asset> AssetStorage<'a, A> {

    pub fn insert(&mut self, asset: A) -> Handle<A> {
        let index = self.inner.insert(AssetState::Loaded(asset));
        let id = AssetId {
            asset_type: TypeId::of::<A>(),
            index,
        };
        let _ = self.sender.send(AssetMessage::HandleCreated(id));
        Handle {
            id,
            sender: self.sender.clone(),
            phantom: PhantomData,
        }
    }

    pub fn get(&self, handle: &Handle<A>) -> AssetState<&A> {
        self.inner.get(handle.id.index).unwrap().as_ref()
    }

    pub fn get_mut(&mut self, handle: &Handle<A>) -> AssetState<&A> {
        self.inner.get_mut(handle.id.index).unwrap().as_ref()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn values(&self) -> impl Iterator<Item = &AssetState<A>> {
        self.inner.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut AssetState<A>> {
        self.inner.values_mut()
    }

    pub fn iter(&mut self) -> impl Iterator<Item = (AssetIndex, &AssetState<A>)> {
        self.inner.iter()
    }
}

/// Simple contiguous storage of assets.
/// Informs asset manager of insertions without passing messages.
/// Has exclusive access to asset manager.
pub struct AssetStorageMut<'a, A> {
    pub(crate) inner: RefMut<'a, InnerAssetStorage<A>>,     // Internal storage of assets
    pub(crate) metas: &'a mut HashMap<AssetId, AssetMeta>,  // Metadata of assets
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

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn values(&self) -> impl Iterator<Item = &AssetState<A>> {
        self.inner.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut AssetState<A>> {
        self.inner.values_mut()
    }
}

/// Simple contiguous storage of assets.
pub(crate) type InnerAssetStorage<A> = SlotMap<AssetIndex, AssetState<A>>;
impl<A: Asset> DynStorage for RefCell<InnerAssetStorage<A>> {

    fn insert_loading(&mut self) -> AssetIndex {
        let slf = self.get_mut();
        slf.insert(AssetState::Loading)
    }

    fn finish_loading(&mut self, index: AssetIndex, asset: Box<dyn Any>) {
        let slf = self.get_mut();
        let state = slf.get_mut(index).unwrap();
        let asset = asset.downcast::<A>().unwrap();
        *state = AssetState::Loaded(*asset);
    }

    fn fail_loading(&mut self, index: AssetIndex) {
        let slf = self.get_mut();
        let state = slf.get_mut(index).unwrap();
        *state = AssetState::Failed;
    }
    
    fn remove(&mut self, index: AssetIndex) {
        let slf = self.get_mut();
        slf.remove(index);
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

    pub fn to_readiness(&self) -> Readiness {
        match self {
            AssetState::Loading => Readiness::NotReady,
            AssetState::Loaded(_) => Readiness::Ready,
            AssetState::Failed => Readiness::Failed,
        }
    }
}

impl<'a, A> AssetState<&'a A> {
    pub fn unwrap(&self) -> &'a A {
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