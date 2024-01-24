use std::sync::mpsc::{Sender, Receiver};
use slotmap::Key;

/// Object to be tracked.
pub trait HasId: Send + Sync + 'static {
    type Id: Key + Send + Sync + 'static;
}

/// A "tracking" handle to an object in some domain (Trackee).
/// When the tracker drops, the object it references will be scheduled for removal.
/// Commonly stored in an ECS Entity to keep some external object alive until the entity is despawned.
pub struct Tracker<T: HasId> {
    id: T::Id,
    sender: TrackerSender<T>,
}

impl<T: HasId> Tracker<T> {
    pub fn new(id: T::Id, sender: TrackerSender<T>) -> Self {
        Self {
            id,
            sender,
        }
    }

    pub fn id(&self) -> T::Id {
        self.id
    }
}

impl<T: HasId> Drop for Tracker<T> {
    fn drop(&mut self) {
        let _ = self.sender.0.send(self.id);
    }
}

/// Produces a channel for a tracker system.
pub fn tracker_channel<T: HasId>() -> (TrackerSender<T>, TrackerReceiver<T>) {
    let (tx, rx) = std::sync::mpsc::channel();
    (TrackerSender(tx), TrackerReceiver(rx))    
}

/// Sends drop messages to a receiver when dropped.
pub struct TrackerSender<T: HasId>(Sender<T::Id>);
impl<T: HasId> Clone for TrackerSender<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Queue of trackee ids to be removed.
pub struct TrackerReceiver<T: HasId>(Receiver<T::Id>);
impl <T: HasId> TrackerReceiver<T> {
    pub fn iter(&self) -> impl Iterator<Item = T::Id> + '_ {
        self.0.try_iter()
    }
}