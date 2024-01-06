use std::any::{TypeId, Any};
use std::cell::{RefCell, Ref, RefMut};
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

    /// Adds a domain to the game.
    pub fn add<D: Domain>(&mut self, domain: D) -> &mut Self {
        self.domains.insert(TypeId::of::<D>(), Box::new(RefCell::new(domain)));
        self
    }

    /// Adds a domain to the game unless one is already present.
    pub fn init<D: Domain>(&mut self, producer: impl Fn() -> D) -> &mut Self {
        let type_id = TypeId::of::<D>();
        if !self.domains.contains_key(&type_id) {
            let domain = producer();
            self.domains.insert(type_id, Box::new(RefCell::new(domain)));
        }
        self
    }

    /// Returns true if a domain with the type specified is present.
    pub fn contains<D: Domain>(&mut self) -> bool {
        self.domains.contains_key(&TypeId::of::<D>())
    }

    /// Fetches a domain by type.
    pub fn get<D: Domain>(&self) -> Ref<'_, D> {
        self.try_get().unwrap()
    }

    /// Fetches a domain by type.
    pub fn get_mut<D: Domain>(&mut self) -> RefMut<'_, D> {
        self.try_get_mut().unwrap()
    }

    /// Fetches a domain by type.
    pub fn try_get<D: Domain>(&self) -> Option<Ref<'_, D>> {
        let domain = self.domains.get(&TypeId::of::<D>())?;
        domain
            .downcast_ref::<RefCell<D>>()
            .map(|ref_cell| ref_cell.borrow())
    }

    /// Fetches a domain by type.
    pub fn try_get_mut<D: Domain>(&mut self) -> Option<RefMut<'_, D>> {
        let domain = self.domains.get_mut(&TypeId::of::<D>())?;
        domain
            .downcast_mut::<RefCell<D>>()
            .map(|ref_cell| ref_cell.borrow_mut())
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
