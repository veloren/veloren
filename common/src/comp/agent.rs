use crate::path::Chaser;
use specs::{Component, Entity as EcsEntity};
use specs_idvs::IDVStorage;
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Alignment {
    Wild,
    Enemy,
    Npc,
    Owned(EcsEntity),
}

impl Alignment {
    // Always attacks
    pub fn hostile_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Enemy, Alignment::Enemy) => false,
            (Alignment::Enemy, _) => true,
            (_, Alignment::Enemy) => true,
            _ => false,
        }
    }

    // Never attacks
    pub fn passive_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Owned(a), Alignment::Owned(b)) if a == b => true,
            _ => false,
        }
    }
}

impl Component for Alignment {
    type Storage = IDVStorage<Self>;
}

#[derive(Clone, Debug, Default)]
pub struct Agent {
    pub patrol_origin: Option<Vec3<f32>>,
    pub activity: Activity,
}

impl Agent {
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
    Attack {
        target: EcsEntity,
        chaser: Chaser,
        time: f64,
        been_close: bool,
    },
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
            Activity::Attack { .. } => true,
            _ => false,
        }
    }
}

impl Default for Activity {
    fn default() -> Self { Activity::Idle(Vec2::zero()) }
}
