#[cfg(not(target_arch = "wasm32"))]
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use specs::{Component, Entity, FlaggedStorage, VecStorage};
use std::{fmt, u64};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Uid(pub u64);

impl From<Uid> for u64 {
    fn from(uid: Uid) -> u64 { uid.0 }
}

impl From<u64> for Uid {
    fn from(uid: u64) -> Self { Self(uid) }
}

impl fmt::Display for Uid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}

#[cfg(not(target_arch = "wasm32"))]
impl Component for Uid {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct UidAllocator {
    next_id: u64,
    mapping: HashMap<Uid, Entity>,
}

#[cfg(not(target_arch = "wasm32"))]
impl UidAllocator {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            mapping: HashMap::new(),
        }
    }

    // Useful for when a single entity is deleted because it doesn't reconstruct the
    // entire hashmap
    pub fn remove_entity(&mut self, id: Uid) -> Option<Entity> { self.mapping.remove(&id) }

    pub fn allocate(&mut self, entity: Entity, id: Option<Uid>) -> Uid {
        let id = id.unwrap_or_else(|| {
            let id = self.next_id;
            self.next_id += 1;
            Uid(id)
        });
        self.mapping.insert(id, entity);
        id
    }

    pub fn retrieve_entity_internal(&self, id: Uid) -> Option<Entity> {
        self.mapping.get(&id).copied()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for UidAllocator {
    fn default() -> Self { Self::new() }
}
