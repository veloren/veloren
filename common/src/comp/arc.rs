use crate::{
    combat::Attack,
    resources::{Secs, Time},
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use specs::Component;
use vek::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Arc {
    pub properties: ArcProperties,
    pub last_arc_time: Time,
    #[serde(skip)]
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
}

impl Component for Arc {
    type Storage = specs::DenseVecStorage<Self>;
}
