use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, Entity};
use vek::Vec3;

use crate::resources::{Secs, Time};

pub enum TeleporterEvent {
    PortalTeleport { entity: Entity, target: Vec3<f32> },
    SetPortalActive { portal: Entity, active: bool },
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Teleporter {
    pub target: Vec3<f32>,
    pub requires_no_aggro: bool,
    pub buildup_time: Secs,
}

impl Component for Teleporter {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Teleporting {
    pub teleport_start: Time,
    pub portal: Entity,
    pub end_time: Time,
}

impl Component for Teleporting {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}
