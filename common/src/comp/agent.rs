use specs::{Component, Entity as EcsEntity, VecStorage};
use vek::*;

#[derive(Copy, Clone, Debug)]
pub enum Agent {
    Wanderer(Vec2<f32>),
    Pet {
        target: EcsEntity,
        offset: Vec2<f32>,
    },
}

impl Component for Agent {
    type Storage = VecStorage<Self>;
}
