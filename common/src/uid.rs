#[cfg(not(target_arch = "wasm32"))]
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use specs::{
    saveload::{Marker, MarkerAllocator},
    world::EntitiesRes,
    Component, Entity, FlaggedStorage, Join, ReadStorage, VecStorage,
};
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
impl Marker for Uid {
    type Allocator = UidAllocator;
    type Identifier = u64;

    fn id(&self) -> u64 { self.0 }

    fn update(&mut self, update: Self) {
        assert_eq!(self.0, update.0);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct UidAllocator {
    index: u64,
    mapping: HashMap<u64, Entity>,
}

#[cfg(not(target_arch = "wasm32"))]
impl UidAllocator {
    pub fn new() -> Self {
        Self {
            index: 0,
            mapping: HashMap::new(),
        }
    }

    // Useful for when a single entity is deleted because it doesn't reconstruct the
    // entire hashmap
    pub fn remove_entity(&mut self, id: u64) -> Option<Entity> { self.mapping.remove(&id) }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for UidAllocator {
    fn default() -> Self { Self::new() }
}

#[cfg(not(target_arch = "wasm32"))]
impl MarkerAllocator<Uid> for UidAllocator {
    fn allocate(&mut self, entity: Entity, id: Option<u64>) -> Uid {
        let id = id.unwrap_or_else(|| {
            let id = self.index;
            self.index += 1;
            id
        });
        self.mapping.insert(id, entity);
        Uid(id)
    }

    fn retrieve_entity_internal(&self, id: u64) -> Option<Entity> { self.mapping.get(&id).copied() }

    fn maintain(&mut self, entities: &EntitiesRes, storage: &ReadStorage<Uid>) {
        self.mapping = (entities, storage)
            .join()
            .map(|(e, m)| (m.id(), e))
            .collect();
    }
}
