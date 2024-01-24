use tracing::instrument;
use crate::{tracker_channel, HasId, SceneGraph, SceneGraphError, Tracker, TrackerReceiver, TrackerSender};

/// Wrapper for a [`SceneGraph`] which adds tracking.
pub struct Scene<R: HasId> {
    pub graph: SceneGraph<R>,
    sender: TrackerSender<R>,
    receiver: TrackerReceiver<R>,
}

impl<R: HasId> Scene<R> {

    pub fn new() -> Self {
        let (sender, receiver) = tracker_channel();
        Self {
            graph: SceneGraph::new(),
            sender,
            receiver,
        }
    }

    pub fn root_ids(&self) -> &[R::Id] {
        self.graph.root_ids()
    }

    /**
     * Iterator over all objects in the scene in no particular order.
     */
    pub fn iter(&self) -> impl Iterator<Item = &R> {
        self.graph.iter()
    }

    /**
     * Iterator over all objects in the scene in no particular order.
     */
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut R> {
        self.graph.iter_mut()

    }
  
    /**
     * Inserts a root object and returns a tracker.
     */
    pub fn insert(&mut self, value: R) -> Tracker<R> {
        let id = self.graph.insert(value);
        Tracker::new(id, self.sender.clone())
    }

    pub fn insert_child(&mut self, value: R, parent_id: R::Id) -> Result<Tracker<R>, SceneGraphError> {
        let node_id = self.graph.insert_child(value, parent_id)?;
        Ok(Tracker::new(node_id, self.sender.clone()))
    }

    pub fn get(&self, node_id: R::Id) -> Option<&R> {
        self.graph.get(node_id)
    }

    pub fn get_mut(&mut self, node_id: R::Id) -> Option<&mut R> {
        self.graph.get_mut(node_id)
    }

    pub fn contains(&mut self, node_id: R::Id) -> bool {
        self.graph.contains(node_id)
    }

    /**
     * Removes an object recursively.
     */
    pub fn remove(&mut self, node_id: R::Id) {
        self.graph.remove(node_id);
    }

     /**
     * Removes a node by id.
     * Its children, if any, are reparented to the removed node's parent.
     * If the removed node did not have a parent, they become root nodes.
     */
    pub fn remove_reparent(&mut self, node_id: R::Id) {
        self.graph.remove_reparent(node_id);
    }

    /**
     * Removes all [`Node`]s.
     */
    pub fn clear(&mut self) {
        self.graph.clear();
    }

    /**
     * The number of [`Node`]s stored.
     */
    pub fn len(&self) -> usize {
        self.graph.len()
    }

    #[instrument(skip_all)]
    pub fn prune_nodes(&mut self) {
        for node_id in self.receiver.iter() {
            self.graph.remove(node_id);
        }
    }
}
