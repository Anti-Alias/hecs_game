use std::any::{TypeId, Any};
use std::cell::RefCell;
use std::collections::HashMap;

/// Game structure, which acts as a simple container of [`Domain`]s.
/// Contains no logic on its own.
pub struct Game {
    domains: HashMap<TypeId, Box<dyn Any>>,
}

impl Game {
    pub fn builder() -> GameBuilder {
        GameBuilder(Self {
            domains: HashMap::new(),
        })
    }
    pub fn domain<D: Domain>(&self) -> &RefCell<D> {
        self.try_domain().unwrap()
    }
    pub fn try_domain<D: Domain>(&self) -> Option<&RefCell<D>> {
        let domain = self.domains.get(&TypeId::of::<D>())?;
        domain.downcast_ref::<RefCell<D>>()
    }
}

pub struct GameBuilder(Game);
impl GameBuilder {
    pub fn domain<D: Domain>(mut self, domain: D) -> Self {
        self.0.domains.insert(TypeId::of::<D>(), Box::new(domain));
        self
    }
    pub fn build(self) -> Game {
        self.0
    }
}

/**
 * A place where logic of a certain variety is performed.
 * IE: Physics, Graphics, logic etc.
 */
pub trait Domain: Any + Send + Sync {}
impl<D: Any + Send + Sync> Domain for D {}
