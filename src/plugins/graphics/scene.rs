use std::collections::HashSet;
use slotmap::{new_key_type, SlotMap};
use smallvec::SmallVec;
use derive_more::*;
use crate::{TrackerSender, Trackee, TrackerReceiver, tracker_channel, Tracker};

/// A hierarchical collection of [`Node`]s with parent/child relationships.
/// Useful for representing the graphics of a game, where each [`Node`] contains a renderable object.
///
/// * `R` - Renderable type.
///
pub struct SceneGraph<R: Trackee> {
    root_ids: HashSet<R::Id>,
    nodes: SlotMap<R::Id, Node<R>>,
    sender: TrackerSender<R>,
    receiver: TrackerReceiver<R>,
}

impl<R: Trackee> SceneGraph<R> {

    pub fn new() -> Self {
        let (sender, receiver) = tracker_channel();
        Self {
            root_ids: HashSet::new(),
            nodes: SlotMap::default(),
            sender,
            receiver,
        }
    }

    pub fn root_ids(&self) -> &HashSet<R::Id> {
        &self.root_ids
    }

    /**
     * Iterator over all objects in the scene in no particular order.
     */
    pub fn iter(&self) -> impl Iterator<Item = &R> {
        self.nodes
            .values()
            .map(|node| &node.value)
    }

    /**
     * Iterator over all objects in the scene in no particular order.
     */
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut R> {
        self.nodes
            .values_mut()
            .map(|node| &mut node.value)
    }
  
    /**
     * Inserts a root object and returns a tracker.
     */
    pub fn insert(&mut self, value: R) -> Tracker<R> {
        let id = self.insert_untracked(value);
        Tracker::new(id, self.sender.clone())
    }

    /**
     * Inserts a root object and returns its id.
     */
    pub fn insert_untracked(&mut self, value: R) -> R::Id {
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
    pub fn insert_child(&mut self, value: R, parent_id: R::Id) -> Result<R::Id, SceneGraphError> {
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
    pub fn insert_child_tracked(&mut self, value: R, parent_id: R::Id) -> Result<Tracker<R>, SceneGraphError> {
        let node_id = self.insert_child(value, parent_id)?;
        Ok(Tracker::new(node_id, self.sender.clone()))
    }

    /**
     * Gets an object by id.
     */
    pub fn get(&self, node_id: R::Id) -> Option<&R> {
        self.nodes
            .get(node_id)
            .map(|node| &node.value)
    }

    /**
     * Gets an object by id.
     */
    pub fn get_mut(&mut self, node_id: R::Id) -> Option<&mut R> {
        self.nodes
            .get_mut(node_id)
            .map(|node| &mut node.value)
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
    pub fn remove(&mut self, node_id: R::Id) -> Option<R> {
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
    pub fn remove_reparent(&mut self, node_id: R::Id) -> Option<R> {
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
     * Removes all [`Node`]s.
     */
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    /**
     * Removes [`Node`]s that hand their trackers dropped.
     * Descendants are also removed.
    */
    pub fn prune_nodes(&mut self) {
        for node_id in self.receiver.iter() {
            if !self.root_ids.remove(&node_id) { continue };
            remove(node_id, &mut self.nodes);
        }
    }

    /// Recursive fold-like operation starting at the root nodes.
    /// Value accumulates from parent to child.
    /// Useful for implementing transform propagation.
    pub fn propagate<'a, A, F>(&'a self, accum: A, mut function: F)
    where
        A: Clone,
        F: FnMut(A, &'a R) -> A
    {
        for root_id in &self.root_ids {
            propagate_at(&self.nodes, *root_id, accum.clone(), &mut function);
        }
    }
}

fn remove<R: Trackee>(node_id: R::Id, nodes: &mut SlotMap<R::Id, Node<R>>) -> Option<R> {
    let mut node = nodes.remove(node_id)?;
    for child_id in &node.children_ids {
        remove(*child_id, nodes);
    }
    node.parent_id = None;
    node.children_ids.clear();
    Some(node.value)
}

fn propagate_at<'a, R: Trackee, A, F>(nodes: &'a SlotMap<R::Id, Node<R>>, node_id: R::Id, accum: A, function: &mut F)
where
    A: Clone,
    F: FnMut(A, &'a R) -> A
{
    let node = nodes.get(node_id).unwrap();
    let current = function(accum, &node.value);
    for child_id in &node.children_ids {
        propagate_at(nodes, *child_id, current.clone(), function);
    }
}

/// Container of a scene graph value, and a reference to its parent and children.
struct Node<R: Trackee> {
    value: R,
    parent_id: Option<R::Id>,
    children_ids: SmallVec<[R::Id; 8]>,
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