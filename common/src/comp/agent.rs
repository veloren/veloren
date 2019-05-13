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

#[derive(Copy, Clone, Debug)]
pub struct Control {
    pub move_dir: Vec2<f32>,
    pub jumping: bool,
}

impl Default for Control {
    fn default() -> Self {
        Self {
            move_dir: Vec2::zero(),
            jumping: false,
        }
    }
}

impl Component for Control {
    type Storage = VecStorage<Self>;
}
