use std::any::{Any, TypeId};
use std::collections::{VecDeque, HashMap};
use crate::Game;

/// Event that is fired the first frame the game starts.
#[derive(Clone)]
pub struct StartEvent;

/**
 * Represents someting that happened in the [`Game`] to be reacted to.
 */
pub trait Event: Any + Send + Sync + Clone {}
impl<E: Any + Send + Sync + Clone> Event for E {}


/// Callback that handles an event.
pub type EventHandler<E> = fn(&mut Game, &E);
 

pub(crate) trait DynEventHandler {
    fn handle_dyn(&self, game: &mut Game, event: &dyn Any);
}

impl<E: Event> DynEventHandler for EventHandler<E> {
    fn handle_dyn(&self, game: &mut Game, event: &dyn Any) {
        let event = event.downcast_ref::<E>().unwrap();
        self(game, event);
    }
}

struct EventExecutor {
    handler: Box<dyn DynEventHandler>,
    events: VecDeque<Box<dyn Any>>,
}


/// Collection of event handlers for a particular stage.
pub(crate) struct EventBus {
    executors: HashMap<TypeId, Vec<EventExecutor>>
}

impl EventBus {

    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
        }
    }
    
    /// Adds an event handler.
    pub fn add_handler<E: Event>(&mut self, handler: EventHandler<E>) {
        let event_type = TypeId::of::<E>();
        let executors = self.executors.entry(event_type).or_default();
        executors.push(EventExecutor {
            handler: Box::new(handler),
            events: VecDeque::new(),
        });
    }

    /// Enqueues an event to be handled later.
    pub fn queue_event<E: Event>(&mut self, event: E) {
        let event_type = TypeId::of::<E>();
        let Some(executors) = self.executors.get_mut(&event_type) else { return };
        for executor in executors {
            executor.events.push_back(Box::new(event.clone()));
        }
    }

    /// Handles enqueued events with registered handlers.
    pub fn run_events(&mut self, game: &mut Game) {
        for executor_vec in self.executors.values_mut() {
            for executor in executor_vec {
                while let Some(event) = executor.events.pop_back() {
                    executor.handler.handle_dyn(game, &*event);
                }
            }
        }
    }
}