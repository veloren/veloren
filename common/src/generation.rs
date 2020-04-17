use vek::*;
use crate::comp::{Alignment};

pub struct EntityInfo {
    pub pos: Vec3<f32>,
    pub is_waypoint: bool, // Edge case, overrides everything else
    pub is_giant: bool,
    pub alignment: Alignment,
}

impl EntityInfo {
    pub fn at(pos: Vec3<f32>) -> Self {
        Self {
            pos,
            is_waypoint: false,
            is_giant: false,
            alignment: Alignment::Wild,
        }
    }

    pub fn do_if(mut self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond {
            self = f(self);
        }
        self
    }

    pub fn into_waypoint(mut self) -> Self {
        self.is_waypoint = true;
        self
    }

    pub fn into_giant(mut self) -> Self {
        self.is_giant = true;
        self
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }
}

#[derive(Default)]
pub struct ChunkSupplement {
    pub entities: Vec<EntityInfo>,
}

impl ChunkSupplement {
    pub fn add_entity(&mut self, entity: EntityInfo) {
        self.entities.push(entity);
    }
}
