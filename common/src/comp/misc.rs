use super::item::Reagent;
use crate::{resources::Time, uid::Uid};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::time::Duration;
use vek::Vec3;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Object {
    Bomb {
        owner: Option<Uid>,
    },
    Firework {
        owner: Option<Uid>,
        reagent: Reagent,
    },
    DeleteAfter {
        spawned_at: Time,
        timeout: Duration,
    },
}

impl Component for Object {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Teleporter {
    pub target: Vec3<f32>,
    pub requires_no_aggro: bool,
}

impl Component for Teleporter {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}
