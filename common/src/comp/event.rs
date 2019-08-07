use specs::Component;
use specs_idvs::IDVStorage;
use std::ops::{Deref, DerefMut};
use vek::*;

#[derive(Clone, Debug, Default)]
pub struct Events(pub Vec<EntityEvent>);

impl Deref for Events {
    type Target = Vec<EntityEvent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Events {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Component for Events {
    type Storage = IDVStorage<Self>;
}

#[derive(Clone, Debug)]
pub enum EntityEvent {
    HitGround { vel: Vec3<f32> },
}
