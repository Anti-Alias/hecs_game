use std::any::{TypeId, Any};
use std::cell::{RefCell, Ref, RefMut};
use std::collections::HashMap;

/// Game structure, which acts as a simple container of [`Domain`]s.
/// Contains no logic on its own.
pub struct Game {
    domains: HashMap<TypeId, Box<dyn Any>>,
}

impl Game {

    pub fn new() -> Self {
        Self {
            domains: HashMap::new()
        }
    }

    /// Adds a domain to the game.
    pub fn add<D: Domain>(&mut self, domain: D) -> &mut Self {
        self.domains.insert(TypeId::of::<D>(), Box::new(RefCell::new(domain)));
        self
    }

    /// Adds a domain to the game unless one is already present.
    pub fn init<D: Domain>(&mut self, producer: impl FnOnce(&mut Game) -> D) -> &mut Self {
        let type_id = TypeId::of::<D>();
        if !self.domains.contains_key(&type_id) {
            let domain = producer(self);
            self.domains.insert(type_id, Box::new(RefCell::new(domain)));
        }
        self
    }

    pub fn get<'a, E0: DomainExtractor<'a>>(&'a self) -> E0::Data {
        E0::extract(self).unwrap()
    }

    pub fn remove<D: Domain>(&mut self) -> D {
        self.try_remove().unwrap()
    }

    pub fn take<D: Domain + Default>(&mut self) -> D {
        self.try_take().unwrap()
    }

    pub fn contains<D: Domain>(&mut self) -> bool {
        self.domains.contains_key(&TypeId::of::<D>())
    }

    pub fn try_get<'a, E0: DomainExtractor<'a>>(&'a self) -> Option<E0::Data> {
        E0::extract(self)
    }

    pub fn try_remove<D: Domain>(&mut self) -> Option<D> {
        let domain = self.domains
            .remove(&TypeId::of::<D>())?
            .downcast::<RefCell<D>>()
            .unwrap()
            .into_inner();
        Some(domain)
    }

    /// Replaces domain with default implementation, and returns domain.
    /// If not found, does nothing and returns None.
    pub fn try_take<D: Domain + Default>(&mut self) -> Option<D> {
        let mut domain = self.domains
            .get(&TypeId::of::<D>())?
            .downcast_ref::<RefCell<D>>()
            .unwrap()
            .borrow_mut();
        let domain = &mut *domain;
        let domain = std::mem::take(domain);
        Some(domain)
    }

    /// Fetches a domain by type.
    pub fn get_cell<D: Domain>(&self) -> &RefCell<D> {
        self.try_get_cell().unwrap()
    }

    /// Fetches a domain by type.
    pub fn try_get_cell<'a, D: Domain>(&self) -> Option<&RefCell<D>> {
        let domain_id = TypeId::of::<D>();
        let any = self.domains.get(&domain_id)?;
        any.downcast_ref::<RefCell<D>>()
    }
}

/**
 * A place where logic of a certain variety is performed.
 * IE: Physics, Graphics, logic etc.
 */
pub trait Domain: Any {}
impl<D: Any> Domain for D {}


pub trait DomainExtractor<'a> {
    type Data;
    fn extract(game: &'a Game) -> Option<Self::Data>;
}

impl<'a, D0> DomainExtractor<'a> for &'a D0
where D0: Domain {
    type Data = Ref<'a, D0>;
    fn extract(game: &'a Game) -> Option<Self::Data> {
        let d0 = game.try_get_cell::<D0>()?;
        Some(d0.borrow())
    }
}

impl<'a, D0> DomainExtractor<'a> for &'a mut D0
where D0: Domain {
    type Data = RefMut<'a, D0>;
    fn extract(game: &'a Game) -> Option<Self::Data> {
        let d0 = game.try_get_cell::<D0>()?;
        Some(d0.borrow_mut())
    }
}