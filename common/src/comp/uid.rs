use std::{
    collections::HashMap,
    ops::Range,
};
use specs::{
    saveload::{Marker, MarkerAllocator},
    world::EntitiesRes,
    Component,
    VecStorage,
    Entity,
    Join,
    ReadStorage,
};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Uid {
    id: u64,
    seq: u64,
}

impl Component for Uid {
    type Storage = VecStorage<Self>;
}

impl Marker for Uid {
    type Identifier = u64;
    type Allocator = UidNode;

    fn id(&self) -> u64 { self.id }

    fn update(&mut self, update: Self) {
        assert_eq!(self.id, update.id);
        self.seq = update.seq;
    }
}

pub struct UidNode {
    pub(crate) range: Range<u64>,
    pub(crate) mapping: HashMap<u64, Entity>,
}

impl MarkerAllocator<Uid> for UidNode {
    fn allocate(&mut self, entity: Entity, id: Option<u64>) -> Uid {
        let id = id.unwrap_or_else(|| {
            self.range.next().expect("
                Id range must be effectively endless.
                Somehow, you ran this program for longer than the lifetime of the universe.
                It's probably time to stop playing and prepare for your imminent extinction.
            ")
        });
        self.mapping.insert(id, entity);
        Uid { id, seq: 0 }
    }

    fn retrieve_entity_internal(&self, id: u64) -> Option<Entity> {
        self.mapping.get(&id).cloned()
    }

    fn maintain(&mut self, entities: &EntitiesRes, storage: &ReadStorage<Uid>) {
        self.mapping = (&*entities, storage)
            .join()
            .map(|(e, m)| (m.id(), e))
            .collect();
    }
}
