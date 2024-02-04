use std::cell::UnsafeCell;

use slotmap::{new_key_type, SlotMap};
use smallvec::SmallVec;
use derive_more::*;

use crate::HasId;

/// A hierarchical collection of [`Node`]s with parent/child relationships.
/// Useful for representing the graphics of a game, where each [`Node`] contains a renderable object.
///
/// * `R` - Renderable type.
///
pub struct SceneGraph<R: HasId> {
    root_ids: Vec<R::Id>,
    nodes: SlotMap<R::Id, NodeWrapper<R>>,
}

unsafe impl<R: HasId> Sync for SceneGraph<R> {}

impl<R: HasId> SceneGraph<R> {

    pub fn new() -> Self {
        Self {
            root_ids: Vec::default(),
            nodes: SlotMap::default(),
        }
    }

    pub fn root_ids(&self) -> &[R::Id] {
        &self.root_ids
    }

    /**
     * Iterator over all root  nodes.
     */
    pub fn root_nodes(&self) -> impl Iterator<Item = &Node<R>> + '_ {
        self.root_ids
            .iter()
            .flat_map(|root_id| self.get_node(*root_id))
    }

    /**
     * Iterator over all objects in the scene in no particular order.
     */
    pub fn iter(&self) -> impl Iterator<Item = &R> {
        self.nodes
            .values()
            .map(|node| &node.get().value)
    }

    /**
     * Iterator over all objects in the scene in no particular order.
     */
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut R> {
        self.nodes
            .values_mut()
            .map(|node| &mut node.get_mut().value)
    }

    /**
     * Inserts a root object and returns its id.
     */
    pub fn insert(&mut self, value: R) -> R::Id {
        let node = NodeWrapper::new(Node {
            value,
            parent_id: None,
            children_ids: SmallVec::new(),
        });
        let node_id = self.nodes.insert(node);
        self.root_ids.push(node_id);
        node_id
    }

    /**
     * Inserts an object as a child of another.
     */
    pub fn insert_child(&mut self, value: R, parent_id: R::Id) -> Result<R::Id, SceneGraphError> {
        let node = NodeWrapper::new(Node {
            value,
            parent_id: Some(parent_id),
            children_ids: SmallVec::new(),
        });
        let node_id = self.nodes.insert(node);
        match self.nodes.get_mut(parent_id) {
            Some(parent) => parent.get_mut().children_ids.push(node_id),
            None => {
                self.nodes.remove(node_id);
                return Err(SceneGraphError::NoSuchNode);
            },
        };
        Ok(node_id)
    }

    /**
     * Gets an object by id.
     * Unwraps it from its node for convenience.
     */
    pub fn get(&self, node_id: R::Id) -> Option<&R> {
        self.nodes
            .get(node_id)
            .map(|node| &node.get().value)
    }

    /**
     * Gets an object by id, wrapped in its node.
     */
    pub fn get_node(&self, node_id: R::Id) -> Option<&Node<R>> {
        self.nodes
            .get(node_id)
            .map(|node| node.get())
    }

    /**
     * Gets an object by id.
     */
    pub fn get_mut(&mut self, node_id: R::Id) -> Option<&mut R> {
        self.nodes
            .get_mut(node_id)
            .map(|node| &mut node.get_mut().value)
    }

    /**
     * Gets an object by id, wrapped in its node.
     */
    pub fn get_node_mut(&mut self, node_id: R::Id) -> Option<&mut Node<R>> {
        self.nodes
            .get_mut(node_id)
            .map(|node| node.get_mut())
    }

    /**
     * Gets an object by id.
     */
    pub unsafe fn get_mut_unsafe(&self, node_id: R::Id) -> Option<&mut R> {
        self.nodes
            .get(node_id)
            .map(|node| unsafe {
                &mut node.get_mut_unsafe().value
            })
    }

    /**
     * True if object is stored.
     */
    pub fn contains(&mut self, node_id: R::Id) -> bool {
        self.nodes.contains_key(node_id)
    }

    /**
     * Removes an object recursively.
     */
    pub fn remove(&mut self, node_id: R::Id) {
        let idx = self.root_ids.iter().position(|id| *id == node_id);
        if let Some(idx) = idx {
            self.root_ids.remove(idx);
        }
        remove(node_id, &mut self.nodes)
    }

    /**
     * Removes all [`Node`]s.
     */
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    /**
     * The number of [`Node`]s stored.
     */
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Recursive fold-like operation starting at the root nodes.
    /// Value accumulates from parent to child.
    /// Useful for implementing transform propagation.
    pub fn propagate<'a, A, F>(&'a self, accum: A, mut function: F)
    where
        A: Clone,
        F: FnMut(A, &'a R) -> A
    {
        for root_id in self.root_ids() {
            propagate_at(&self.nodes, *root_id, accum.clone(), &mut function);
        };
    }
}

fn propagate_at<'a, R: HasId, A, F>(
    nodes: &'a SlotMap<R::Id, NodeWrapper<R>>,
    node_id: R::Id,
    accum: A,
    function: &mut F
)
where
    A: Clone,
    F: FnMut(A, &'a R) -> A
{
    let node = unsafe { nodes.get_unchecked(node_id) };
    let current = function(accum, &node.get().value);
    for child_id in &node.get().children_ids {
        propagate_at(nodes, *child_id, current.clone(), function);
    }
}

fn remove<R: HasId>(node_id: R::Id, nodes: &mut SlotMap<R::Id, NodeWrapper<R>>) {
    let Some(node) = nodes.remove(node_id) else { return };
    for child_id in &node.get().children_ids {
        remove(*child_id, nodes);
    }
}


struct NodeWrapper<R: HasId>(UnsafeCell<Node<R>>);
impl<R: HasId> NodeWrapper<R> {

    fn new(node: Node<R>) -> Self {
        Self(UnsafeCell::new(node))
    }

    fn get(&self) -> &Node<R> {
        let ptr = self.0.get();
        unsafe { &*ptr }
    }

    fn get_mut(&mut self) -> &mut Node<R> {
        self.0.get_mut()
    }

    unsafe fn get_mut_unsafe(&self) -> &mut Node<R> {
        let ptr = self.0.get();
        &mut *ptr
    }
}

/// Container of a scene graph value, and a reference to its parent and children.
pub struct Node<R: HasId> {
    value: R,
    parent_id: Option<R::Id>,
    children_ids: SmallVec<[R::Id; 8]>,
}

impl<R: HasId> Node<R> {
    pub fn value(&self) -> &R {
        &self.value
    }
    pub fn value_mut(&mut self) -> &mut R {
        &mut self.value
    }
    pub fn parent_id(&self) -> Option<&R::Id> {
        self.parent_id.as_ref()
    }
    pub fn children_ids(&self) -> &[R::Id] {
        &self.children_ids
    }
}

new_key_type! {
    /**
     * ID for a [`Node`].
     */
    pub struct NodeId;
}

#[derive(Error, Display, Debug, From)]
pub enum SceneGraphError {
    #[display(fmt="No such node")]
    NoSuchNode,
}