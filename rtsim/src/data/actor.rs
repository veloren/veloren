use hashbrown::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorId {
    pub idx: u32,
    pub gen: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Actor {}

#[derive(Clone, Serialize, Deserialize)]
pub struct Actors {
    pub actors: HashMap<ActorId, Actor>,
}
