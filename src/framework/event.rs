use std::any::{Any, TypeId};
use crate::{Game, HashMap, RunContext};

/// Event that is fired the first frame the game starts.
#[derive(Clone)]
pub struct StartEvent;

/**
 * Represents someting that happened in the [`Game`] to be reacted to.
 */
pub trait Event: Any + Send + Sync + Clone {}
impl<E: Any + Send + Sync + Clone> Event for E {}

pub(crate) struct DynEvent {
    pub event: Box<dyn Any>,
    pub type_id: TypeId,
}

impl DynEvent {
    pub fn new<E: Event>(event: E) -> Self {
        Self {
            event: Box::new(event),
            type_id: TypeId::of::<E>(),
        }
    }
}


/// Callback that handles an event.
pub type EventHandler<E> = fn(&mut Game, &E, &mut RunContext);
 

pub(crate) trait DynEventHandler {
    fn handle_dyn(&self, game: &mut Game, event: &DynEvent, ctx: &mut RunContext);
}

impl<E: Event> DynEventHandler for EventHandler<E> {
    fn handle_dyn(&self, game: &mut Game, event: &DynEvent, ctx: &mut RunContext) {
        let event = event.event.downcast_ref::<E>().unwrap();
        self(game, event, ctx);
    }
}

/// Collection of event handlers for a particular stage.
#[derive(Default)]
pub(crate) struct EventBus {
    handlers: HashMap<TypeId, Vec<Box<dyn DynEventHandler>>>
}

impl EventBus {
    
    /// Adds an event handler.
    pub fn add_handler<E: Event>(&mut self, handler: EventHandler<E>) {
        let event_type = TypeId::of::<E>();
        let handlers_for_event = self.handlers.entry(event_type).or_default();
        handlers_for_event.push(Box::new(handler));
    }

    pub fn handle_event(&self, game: &mut Game, event: DynEvent, ctx: &mut RunContext) {
        let Some(handlers_for_event) = self.handlers.get(&event.type_id) else { return };
        for handler in handlers_for_event {
            handler.handle_dyn(game, &event, ctx);
        }
    }
}