use crate::{
    comp::phys::{Ori, Pos},
    sync::Uid,
};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shockwave {
    pub shockwave_origin: Pos,
    pub shockwave_direction: Ori,
    pub shockwave_angle: f32,
    pub shockwave_speed: f32,
    pub shockwave_duration: Duration,
    pub damage: u32,
    pub knockback: f32,
    pub requires_ground: bool,
    pub owner: Option<Uid>,
}

impl Component for Shockwave {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
