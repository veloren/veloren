use specs::{Component, Entity as EcsEntity};
use vek::*;
use specs_idvs::IDVStorage;

#[derive(Copy, Clone, Debug)]
pub enum Agent {
    Wanderer(Vec2<f32>),
    Pet {
        target: EcsEntity,
        offset: Vec2<f32>,
    },
    Enemy {
        target: Option<EcsEntity>,
    },
}

impl Component for Agent {
    type Storage = IDVStorage<Self>;
}
