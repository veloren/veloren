use crate::path::Chaser;
use specs::{Component, Entity as EcsEntity};
use specs_idvs::IDVStorage;
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Alignment {
    Wild,
    Enemy,
    Npc,
}

impl Alignment {
    pub fn hostile_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Wild, Alignment::Npc) => true,
            _ => self != other,
        }
    }
}

impl Component for Alignment {
    type Storage = IDVStorage<Self>;
}

#[derive(Clone, Debug, Default)]
pub struct Agent {
    pub chaser: Chaser,
    pub target: Option<EcsEntity>,
    pub owner: Option<EcsEntity>,
    pub patrol_origin: Option<Vec3<f32>>,
}

impl Agent {
    pub fn with_pet(mut self, owner: EcsEntity) -> Self {
        self.owner = Some(owner);
        self
    }
}

impl Component for Agent {
    type Storage = IDVStorage<Self>;
}
