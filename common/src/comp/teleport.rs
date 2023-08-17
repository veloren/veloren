use specs::{Component, DerefFlaggedStorage, Entity};

use crate::resources::Time;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Teleporting {
    pub portal: Entity,
    pub end_time: Time,
}

impl Component for Teleporting {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}
