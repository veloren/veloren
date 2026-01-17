use crate::{
    combat::Attack,
    resources::{Secs, Time},
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use specs::Component;
use vek::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Arcing {
    pub properties: ArcProperties,
    pub last_arc_time: Time,
    pub hit_entities: Vec<Uid>,
    pub owner: Option<Uid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArcProperties {
    pub attack: Attack,
    pub distance: f32,
    pub arcs: u32,
    pub min_delay: Secs,
    pub max_delay: Secs,
    pub targets_owner: bool,
}

impl Component for Arcing {
    type Storage = specs::DerefFlaggedStorage<Self, specs::DenseVecStorage<Self>>;
}
