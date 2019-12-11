use crate::pathfinding::WorldPath;
use specs::{Component, Entity as EcsEntity};
use specs_idvs::IDVStorage;
use vek::*;

#[derive(Clone, Debug)]
pub enum Agent {
    Wanderer(Vec2<f32>),
    Pet {
        target: EcsEntity,
        offset: Vec2<f32>,
    },
    Enemy {
        bearing: Vec2<f32>,
        target: Option<EcsEntity>,
    },
    Traveler {
        path: WorldPath,
    },
}

impl Agent {
    pub fn enemy() -> Self {
        Agent::Enemy {
            bearing: Vec2::zero(),
            target: None,
        }
    }
}

impl Component for Agent {
    type Storage = IDVStorage<Self>;
}
