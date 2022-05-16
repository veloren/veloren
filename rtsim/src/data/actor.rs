use hashbrown::HashMap;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct ActorId {
    pub idx: u32,
    pub gen: u32,
}

pub struct Actor {}

pub struct Actors {
    pub actors: HashMap<ActorId, Actor>,
}
