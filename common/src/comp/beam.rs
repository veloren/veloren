use crate::sync::Uid;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Properties {
    pub angle: f32,
    pub speed: f32,
    pub damage: u32,
    pub heal: u32,
    pub lifesteal_eff: f32,
    pub energy_regen: u32,
    pub energy_drain: u32,
    pub duration: Duration,
    pub owner: Option<Uid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Beam {
    pub properties: Properties,
    #[serde(skip)]
    /// Time that the beam segment was created at
    /// Used to calculate beam propagation
    /// Deserialized from the network as `None`
    pub creation: Option<f64>,
}

impl Component for Beam {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

impl std::ops::Deref for Beam {
    type Target = Properties;

    fn deref(&self) -> &Properties { &self.properties }
}
