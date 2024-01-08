use slotmap::{new_key_type, SlotMap};
use smallvec::SmallVec;
use derive_more::*;

/**
 * Collection of [`Node`] with parent/child relationships.
 */
pub struct SceneGraph<V> {
    root_ids: Vec<NodeId>,
    nodes: SlotMap<NodeId, Node<V>>,
}

impl<V> SceneGraph<V> {

    pub fn new() -> Self {
        Self {
            root_ids: Vec::new(),
            nodes: SlotMap::default()
        }
    }

    pub fn root_ids(&self) -> &[NodeId] {
        &self.root_ids
    }
  
    /**
     * Inserts a root [`Node`].
     */
    pub fn insert(&mut self, value: V) -> NodeId {
        let node = Node {
            value,
            parent_id: None,
            children_ids: SmallVec::new(),
        };
        let node_id = self.nodes.insert(node);
        self.root_ids.push(node_id);
        node_id
    }

    /**
     * Inserts a [`Node`] as a child of another.
     */
    pub fn insert_child(&mut self, value: V, parent_id: NodeId) -> Result<NodeId, SceneGraphError> {
        let node = Node {
            value,
            parent_id: Some(parent_id),
            children_ids: SmallVec::new(),
        };
        let node_id = self.nodes.insert(node);
        match self.nodes.get_mut(parent_id) {
            Some(parent) => parent.children_ids.push(node_id),
            None => {
                self.nodes.remove(node_id);
                return Err(SceneGraphError::NoSuchNode);
            },
        };
        todo!()
    }

    pub fn get(&self, node_id: NodeId) -> Option<&Node<V>> {
        self.nodes.get(node_id)
    }

    pub fn get_mut(&mut self, node_id: NodeId) -> Option<&mut Node<V>> {
        self.nodes.get_mut(node_id)
    }

    pub fn contains(&mut self, node_id: NodeId) -> bool {
        self.nodes.contains_key(node_id)
    }

    /**
     * Removes a node by id recursively.
     */
    pub fn remove(&mut self, node_id: NodeId) -> Option<V> {
        let mut node = self.nodes.remove(node_id)?;
        for child_id in &node.children_ids {
            self.remove(*child_id);
        }
        node.parent_id = None;
        node.children_ids.clear();
        Some(node.value)
    }

     /**
     * Removes a node by id.
     * Its children, if any, are reparented to the removed node's parent.
     * If the removed node did not have a parent, they become root nodes.
     */
    pub fn remove_reparent(&mut self, node_id: NodeId) -> Option<V> {
        let mut node = self.nodes.remove(node_id)?;

        // Children are reparented
        if let Some(parent_id) = node.parent_id {
            let parent = self.nodes.get_mut(parent_id).unwrap();
            parent.children_ids.extend_from_slice(&node.children_ids);
            for child_id in &node.children_ids {
                let child = self.nodes.get_mut(*child_id).unwrap();
                child.parent_id = Some(parent_id);
            }
        }

        // Children become roots
        else {
            for child_id in &node.children_ids {
                let child = self.nodes.get_mut(*child_id).unwrap();
                child.parent_id = None;
                self.root_ids.push(node_id);
            }
        }

        node.parent_id = None;
        node.children_ids.clear();
        Some(node.value)
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn for_each<F>(&self, mut function: F)
    where F: FnMut(&Node<V>)
    {
        for root_id in &self.root_ids {
            self.for_each_at(*root_id, &mut function);
        }
    }

    fn for_each_at<F>(&self, node_id: NodeId, function: &mut F)
    where F: FnMut(&Node<V>)
    {
        let node = self.get(node_id).unwrap();
        function(node);
        for child_id in &node.children_ids {
            self.for_each_at(*child_id, function);
        }
    }

    /// Recursive fold-like operation starting at the root nodes that manipulates nodes as it traverses.
    /// Useful for implementing transform propagation.
    pub fn propagate<B, F>(&mut self, init: B, mut function: F)
    where
        B: Clone,
        F: FnMut(B, &mut Node<V>) -> B
    {
        let root_ids: &'static [NodeId] = unsafe {
            let slice: &[NodeId] = &self.root_ids;
            std::mem::transmute(slice)
        };
        for root_id in root_ids {
            self.propagate_at(*root_id, init.clone(), &mut function);
        }
        todo!()
    }

    fn propagate_at<B, F>(&mut self, node_id: NodeId, init: B, function: &mut F)
    where
        B: Clone,
        F: FnMut(B, &mut Node<V>) -> B
    {
        let node = self.nodes.get_mut(node_id).unwrap();
        let current = function(init, node);
        let children_ids: &'static [NodeId] = unsafe {
            let slice: &[NodeId] = &node.children_ids;
            std::mem::transmute(slice)
        };
        for child_id in children_ids {
            self.propagate_at(*child_id, current.clone(), function);
        }
    }
}

pub struct Node<V> {
    pub value: V,
    parent_id: Option<NodeId>,
    children_ids: SmallVec<[NodeId; 8]>,
}

impl<V> Node<V> {
    pub fn new(value: V) -> Self {
        Self {
            value,
            parent_id: None,
            children_ids: SmallVec::new(),
        }
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