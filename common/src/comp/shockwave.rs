use crate::{sync::Uid, Damages};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Properties {
    pub angle: f32,
    pub vertical_angle: f32,
    pub speed: f32,
    pub damages: Damages,
    pub knockback: f32,
    pub requires_ground: bool,
    pub duration: Duration,
    pub owner: Option<Uid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shockwave {
    pub properties: Properties,
    #[serde(skip)]
    /// Time that the shockwave was created at
    /// Used to calculate shockwave propagation
    /// Deserialized from the network as `None`
    pub creation: Option<f64>,
}

impl Component for Shockwave {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

impl std::ops::Deref for Shockwave {
    type Target = Properties;

    fn deref(&self) -> &Properties { &self.properties }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShockwaveHitEntities {
    pub hit_entities: Vec<Uid>,
}

impl Component for ShockwaveHitEntities {
    type Storage = IdvStorage<Self>;
}
