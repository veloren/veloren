use super::*;
use crate::data::{Actor, ActorId, Actors};
use hashbrown::HashMap;

// ActorId

impl Latest<ActorId> for ActorIdV0 {
    fn to_unversioned(self) -> ActorId {
        ActorId {
            idx: self.idx,
            gen: self.gen,
        }
    }

    fn from_unversioned(id: &ActorId) -> Self {
        Self {
            idx: id.idx,
            gen: id.gen,
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ActorIdV0 {
    pub idx: u32,
    pub gen: u32,
}

impl Version for ActorIdV0 {
    type Prev = Bottom;

    fn migrate(x: Self::Prev) -> Self { match x {} }
}

// Actor

impl Latest<Actor> for ActorV0 {
    fn to_unversioned(self) -> Actor { Actor {} }

    fn from_unversioned(actor: &Actor) -> Self { Self {} }
}

#[derive(Serialize, Deserialize, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ActorV0 {}

impl Version for ActorV0 {
    type Prev = Bottom;

    fn migrate(x: Self::Prev) -> Self { match x {} }
}

// Actors

impl Latest<Actors> for ActorsV0 {
    fn to_unversioned(self) -> Actors {
        Actors {
            actors: self
                .actors
                .into_iter()
                .map(|(k, v)| (k.to_unversioned(), v.to_unversioned()))
                .collect(),
        }
    }

    fn from_unversioned(actors: &Actors) -> Self {
        Self {
            actors: actors
                .actors
                .iter()
                .map(|(k, v)| (Latest::from_unversioned(k), Latest::from_unversioned(v)))
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ActorsV0 {
    actors: HashMap<V<ActorIdV0>, V<ActorV0>>,
}

impl Version for ActorsV0 {
    type Prev = Bottom;

    fn migrate(x: Self::Prev) -> Self { match x {} }
}
