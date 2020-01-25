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
    pub owner: Option<EcsEntity>,
    pub patrol_origin: Option<Vec3<f32>>,
    pub activity: Activity,
}

impl Agent {
    pub fn with_pet(mut self, owner: EcsEntity) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn with_patrol_origin(mut self, origin: Vec3<f32>) -> Self {
        self.patrol_origin = Some(origin);
        self
    }
}

impl Component for Agent {
    type Storage = IDVStorage<Self>;
}

#[derive(Clone, Debug)]
pub enum Activity {
    Idle(Vec2<f32>),
    Follow(EcsEntity, Chaser),
    Attack(EcsEntity, Chaser, f64),
}

impl Activity {
    pub fn is_follow(&self) -> bool {
        match self {
            Activity::Follow(_, _) => true,
            _ => false,
        }
    }

    pub fn is_attack(&self) -> bool {
        match self {
            Activity::Attack(_, _, _) => true,
            _ => false,
        }
    }
}

impl Default for Activity {
    fn default() -> Self {
        Activity::Idle(Vec2::zero())
    }
}
