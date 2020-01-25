use vek::*;

pub enum EntityKind {
    Enemy,
    Boss,
    Waypoint,
}

pub struct EntityInfo {
    pub pos: Vec3<f32>,
    pub kind: EntityKind,
}

#[derive(Default)]
pub struct ChunkSupplement {
    pub entities: Vec<EntityInfo>,
}

impl ChunkSupplement {
    pub fn with_entity(mut self, entity: EntityInfo) -> Self {
        self.entities.push(entity);
        self
    }
}
