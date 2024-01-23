use std::any::{TypeId, Any};
use std::cell::{RefCell, Ref, RefMut};
use crate::HashMap;

/// Game structure, which acts as a simple container of [`Domain`]s.
/// Contains no logic on its own.
pub struct Game {
    domains: HashMap<TypeId, Box<dyn Any>>,
}

impl Game {

    pub fn new() -> Self {
        Self {
            domains: HashMap::default()
        }
    }

    /// Adds a domain to the game.
    pub fn add<D: Domain>(&mut self, domain: D) -> &mut Self {
        self.domains.insert(TypeId::of::<D>(), Box::new(RefCell::new(domain)));
        self
    }

    /// Adds a domain to the game unless one is already present.
    pub fn init<D: Domain>(&mut self, producer: impl Fn(&mut Game) -> D) -> &mut Self {
        let type_id = TypeId::of::<D>();
        if !self.domains.contains_key(&type_id) {
            let domain = producer(self);
            self.domains.insert(type_id, Box::new(RefCell::new(domain)));
        }
        self
    }

    /// Returns true if a domain with the type specified is present.
    pub fn contains<D: Domain>(&mut self) -> bool {
        self.domains.contains_key(&TypeId::of::<D>())
    }

    pub fn get<'a, E0: DomainExtractor<'a>>(&'a self) -> E0::Data {
        E0::extract(self)
    }

    pub fn all<'a, S: DomainSet<'a>>(&'a self) -> S::Data {
        S::extract(self)
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


pub trait DomainSet<'a> {
    type Data;
    fn extract(game: &'a Game) -> Self::Data;
}

impl<'a, E0> DomainSet<'a> for (E0,)
where
    E0: DomainExtractor<'a>,
{
    type Data = (E0::Data,);
    fn extract(game: &'a Game) -> Self::Data {
        (E0::extract(game),)
    }
}

impl<'a, E0, E1> DomainSet<'a> for (E0, E1)
where
    E0: DomainExtractor<'a>,
    E1: DomainExtractor<'a>
{
    type Data = (
        E0::Data,
        E1::Data,
    );
    fn extract(game: &'a Game) -> Self::Data {
        (
            E0::extract(game),
            E1::extract(game),
        )
    }
}

impl<'a, E0, E1, E2> DomainSet<'a> for (E0, E1, E2)
where
    E0: DomainExtractor<'a>,
    E1: DomainExtractor<'a>,
    E2: DomainExtractor<'a>,
{
    type Data = (
        E0::Data,
        E1::Data,
        E2::Data,
    );
    fn extract(game: &'a Game) -> Self::Data {
        (
            E0::extract(game),
            E1::extract(game),
            E2::extract(game),
        )
    }
}

impl<'a, E0, E1, E2, E3> DomainSet<'a> for (E0, E1, E2, E3)
where
    E0: DomainExtractor<'a>,
    E1: DomainExtractor<'a>,
    E2: DomainExtractor<'a>,
    E3: DomainExtractor<'a>,
{
    type Data = (
        E0::Data,
        E1::Data,
        E2::Data,
        E3::Data,
    );
    fn extract(game: &'a Game) -> Self::Data {
        (
            E0::extract(game),
            E1::extract(game),
            E2::extract(game),
            E3::extract(game),
        )
    }
}

impl<'a, E0, E1, E2, E3, E4> DomainSet<'a> for (E0, E1, E2, E3, E4)
where
    E0: DomainExtractor<'a>,
    E1: DomainExtractor<'a>,
    E2: DomainExtractor<'a>,
    E3: DomainExtractor<'a>,
    E4: DomainExtractor<'a>,
{
    type Data = (
        E0::Data,
        E1::Data,
        E2::Data,
        E3::Data,
        E4::Data,
    );
    fn extract(game: &'a Game) -> Self::Data {
        (
            E0::extract(game),
            E1::extract(game),
            E2::extract(game),
            E3::extract(game),
            E4::extract(game),
        )
    }
}

impl<'a, E0, E1, E2, E3, E4, E5> DomainSet<'a> for (E0, E1, E2, E3, E4, E5)
where
    E0: DomainExtractor<'a>,
    E1: DomainExtractor<'a>,
    E2: DomainExtractor<'a>,
    E3: DomainExtractor<'a>,
    E4: DomainExtractor<'a>,
    E5: DomainExtractor<'a>,
{
    type Data = (
        E0::Data,
        E1::Data,
        E2::Data,
        E3::Data,
        E4::Data,
        E5::Data,
    );
    fn extract(game: &'a Game) -> Self::Data {
        (
            E0::extract(game),
            E1::extract(game),
            E2::extract(game),
            E3::extract(game),
            E4::extract(game),
            E5::extract(game),
        )
    }
}

impl<'a, E0, E1, E2, E3, E4, E5, E6> DomainSet<'a> for (E0, E1, E2, E3, E4, E5, E6)
where
    E0: DomainExtractor<'a>,
    E1: DomainExtractor<'a>,
    E2: DomainExtractor<'a>,
    E3: DomainExtractor<'a>,
    E4: DomainExtractor<'a>,
    E5: DomainExtractor<'a>,
    E5: DomainExtractor<'a>,
    E6: DomainExtractor<'a>,
{
    type Data = (
        E0::Data,
        E1::Data,
        E2::Data,
        E3::Data,
        E4::Data,
        E5::Data,
        E6::Data,
    );
    fn extract(game: &'a Game) -> Self::Data {
        (
            E0::extract(game),
            E1::extract(game),
            E2::extract(game),
            E3::extract(game),
            E4::extract(game),
            E5::extract(game),
            E6::extract(game),
        )
    }
}

impl<'a, E0, E1, E2, E3, E4, E5, E6, E7> DomainSet<'a> for (E0, E1, E2, E3, E4, E5, E6, E7)
where
    E0: DomainExtractor<'a>,
    E1: DomainExtractor<'a>,
    E2: DomainExtractor<'a>,
    E3: DomainExtractor<'a>,
    E4: DomainExtractor<'a>,
    E5: DomainExtractor<'a>,
    E5: DomainExtractor<'a>,
    E6: DomainExtractor<'a>,
    E7: DomainExtractor<'a>,
{
    type Data = (
        E0::Data,
        E1::Data,
        E2::Data,
        E3::Data,
        E4::Data,
        E5::Data,
        E6::Data,
        E7::Data,
    );
    fn extract(game: &'a Game) -> Self::Data {
        (
            E0::extract(game),
            E1::extract(game),
            E2::extract(game),
            E3::extract(game),
            E4::extract(game),
            E5::extract(game),
            E6::extract(game),
            E7::extract(game),
        )
    }
}


pub trait DomainExtractor<'a> {
    type Data;
    fn extract(game: &'a Game) -> Self::Data;
}

impl<'a, D0> DomainExtractor<'a> for &'a D0
where D0: Domain {
    type Data = Ref<'a, D0>;
    fn extract(game: &'a Game) -> Self::Data {
        let d0 = game.get_cell::<D0>();
        d0.borrow()
    }
}

impl<'a, D0> DomainExtractor<'a> for &'a mut D0
where D0: Domain {
    type Data = RefMut<'a, D0>;
    fn extract(game: &'a Game) -> Self::Data {
        let d0 = game.get_cell::<D0>();
        d0.borrow_mut()
    }
}