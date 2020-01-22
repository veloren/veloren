use crate::{path::Chaser, pathfinding::WorldPath};
use specs::{Component, Entity as EcsEntity};
use specs_idvs::IDVStorage;
use vek::*;

#[derive(Clone, Debug)]
pub enum Agent {
    Wanderer(Vec2<f32>),
    Pet {
        target: EcsEntity,
        chaser: Chaser,
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

    pub fn pet(target: EcsEntity) -> Self {
        Agent::Pet {
            target,
            chaser: Chaser::default(),
        }
    }
}

impl Component for Agent {
    type Storage = IDVStorage<Self>;
}
