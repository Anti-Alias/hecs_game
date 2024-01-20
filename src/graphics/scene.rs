use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::mpsc::{Sender, Receiver, self};

use slotmap::{new_key_type, SlotMap};
use smallvec::SmallVec;
use derive_more::*;

/**
 * Collection of [`Node`] with parent/child relationships.
 */
pub struct SceneGraph<V> {
    root_ids: HashSet<NodeId>,
    nodes: SlotMap<NodeId, Node<V>>,
    sender: Sender<NodeMessage>,
    receiver: Receiver<NodeMessage>,
}

impl<V> SceneGraph<V> {

    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            root_ids: HashSet::new(),
            nodes: SlotMap::default(),
            sender,
            receiver,
        }
    }

    pub fn root_ids(&self) -> &HashSet<NodeId> {
        &self.root_ids
    }

    /**
     * Iterator over all objects in the scene.
     */
    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.nodes
            .values()
            .map(|node| &node.value)
    }
  
    /**
     * Inserts a root object and returns a tracker.
     */
    pub fn insert(&mut self, value: V) -> NodeTracker<V> {
        NodeTracker {
            node_id: self.insert_untracked(value),
            sender: self.sender.clone(),
            phantom: PhantomData,
        }
    }

    /**
     * Inserts a root object and returns its id.
     */
    pub fn insert_untracked(&mut self, value: V) -> NodeId {
        let node = Node {
            value,
            parent_id: None,
            children_ids: SmallVec::new(),
        };
        let node_id = self.nodes.insert(node);
        self.root_ids.insert(node_id);
        node_id
    }

    /**
     * Inserts an object as a child of another.
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
        Ok(node_id)
    }

    /**
     * Inserts an object as a child of another.
     */
    pub fn insert_child_tracked(&mut self, value: V, parent_id: NodeId) -> Result<NodeTracker<V>, SceneGraphError> {
        let node_id = self.insert_child(value, parent_id)?;
        Ok(NodeTracker {
            node_id,
            sender: self.sender.clone(),
            phantom: PhantomData,
        })
    }

    /**
     * Gets an object by id.
     */
    pub fn get(&self, node_id: NodeId) -> Option<&V> {
        self.nodes
            .get(node_id)
            .map(|node| &node.value)
    }

    /**
     * Gets an object by id.
     */
    pub fn get_mut(&mut self, node_id: NodeId) -> Option<&mut V> {
        self.nodes
            .get_mut(node_id)
            .map(|node| &mut node.value)
    }

    /**
     * True if object is stored.
     */
    pub fn contains(&mut self, node_id: NodeId) -> bool {
        self.nodes.contains_key(node_id)
    }

    /**
     * Removes an object recursively.
     */
    pub fn remove(&mut self, node_id: NodeId) -> Option<V> {
        if !self.root_ids.remove(&node_id) {
            return None;
        }
        remove(node_id, &mut self.nodes)
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
                self.root_ids.insert(node_id);
            }
        }

        node.parent_id = None;
        node.children_ids.clear();
        Some(node.value)
    }

    /**
     * Removes all objects.
     */
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    /**
     * Drops [`Node`]s that hand their handles destroyed.
    */
    pub fn prune_nodes(&mut self) {
        for message in self.receiver.try_iter() {
            match message {
                NodeMessage::DropNode(node_id) => {
                    if !self.root_ids.remove(&node_id) { continue };
                    remove(node_id, &mut self.nodes)
                },
            };
        }
    }

    /// Recursive fold-like operation starting at the root nodes.
    /// Value accumulates from parent to child.
    /// Useful for implementing transform propagation.
    pub fn propagate<'a, A, F>(&'a self, accum: A, mut function: F)
    where
        A: Clone,
        F: FnMut(A, &'a V) -> A
    {
        for root_id in &self.root_ids {
            propagate_at(&self.nodes, *root_id, accum.clone(), &mut function);
        }
    }
}

fn remove<V>(node_id: NodeId, nodes: &mut SlotMap<NodeId, Node<V>>) -> Option<V> {
    let mut node = nodes.remove(node_id)?;
    for child_id in &node.children_ids {
        remove(*child_id, nodes);
    }
    node.parent_id = None;
    node.children_ids.clear();
    Some(node.value)
}

fn propagate_at<'a, V, A, F>(nodes: &'a SlotMap<NodeId, Node<V>>, node_id: NodeId, accum: A, function: &mut F)
where
    A: Clone,
    F: FnMut(A, &'a V) -> A
{
    let node = nodes.get(node_id).unwrap();
    let current = function(accum, &node.value);
    for child_id in &node.children_ids {
        propagate_at(nodes, *child_id, current.clone(), function);
    }
}

/// Container of a scene graph value, and a reference to its parent and children.
struct Node<V> {
    value: V,
    parent_id: Option<NodeId>,
    children_ids: SmallVec<[NodeId; 8]>,
}

new_key_type! {
    /**
     * ID for a [`Node`].
     */
    pub struct NodeId;
}

/// A tracked handle to an object in a [`SceneGraph`].
/// When the handle drops, the object and its descendants get removed.
#[derive(Debug)]
pub struct NodeTracker<V> {
    node_id: NodeId,
    sender: Sender<NodeMessage>,
    phantom: PhantomData<V>,
}

impl<V> NodeTracker<V> {
    pub fn node_id(&self) -> NodeId { self.node_id }
}

impl<V> Drop for NodeTracker<V> {
    fn drop(&mut self) {
        let _ = self.sender.send(NodeMessage::DropNode(self.node_id));
    }
}

enum NodeMessage {
    DropNode(NodeId),
}

#[derive(Error, Display, Debug, From)]
pub enum SceneGraphError {
    #[display(fmt="No such node")]
    NoSuchNode,
}