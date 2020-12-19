use crate::{comp::PoiseChange, uid::Uid, Damage, GroupTarget};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Properties {
    pub angle: f32,
    pub speed: f32,
    pub effects: Vec<(Option<GroupTarget>, Damage, PoiseChange)>,
    pub lifesteal_eff: f32,
    pub energy_regen: u32,
    pub energy_cost: u32,
    pub duration: Duration,
    pub owner: Option<Uid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BeamSegment {
    pub properties: Properties,
    #[serde(skip)]
    /// Time that the beam segment was created at
    /// Used to calculate beam propagation
    /// Deserialized from the network as `None`
    pub creation: Option<f64>,
}

impl Component for BeamSegment {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}

impl std::ops::Deref for BeamSegment {
    type Target = Properties;

    fn deref(&self) -> &Properties { &self.properties }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Beam {
    pub hit_entities: Vec<Uid>,
    pub tick_dur: Duration,
    pub timer: Duration,
}

impl Component for Beam {
    type Storage = IdvStorage<Self>;
}
