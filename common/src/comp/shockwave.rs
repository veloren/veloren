use crate::{
    combat::{Attack, AttackSource},
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::time::Duration;

use super::ability::Dodgeable;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Properties {
    pub angle: f32,
    pub vertical_angle: f32,
    pub speed: f32,
    pub attack: Attack,
    pub dodgeable: Dodgeable,
    pub duration: Duration,
    pub owner: Option<Uid>,
    pub specifier: FrontendSpecifier,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shockwave {
    pub properties: Properties,
    #[serde(skip)]
    /// Time that the shockwave was created at
    /// Used to calculate shockwave propagation
    /// Deserialized from the network as `None`
    pub creation: Option<f64>,
}

impl Component for Shockwave {
    type Storage = DerefFlaggedStorage<Self, specs::DenseVecStorage<Self>>;
}

impl std::ops::Deref for Shockwave {
    type Target = Properties;

    fn deref(&self) -> &Properties { &self.properties }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShockwaveHitEntities {
    pub hit_entities: Vec<Uid>,
}

impl Component for ShockwaveHitEntities {
    type Storage = specs::DenseVecStorage<Self>;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrontendSpecifier {
    Ground,
    Fire,
    FireLow,
    Water,
    Ice,
    IceSpikes,
    Steam,
    Poison,
    AcidCloud,
    Ink,
    Lightning,
}

impl Dodgeable {
    pub fn shockwave_attack_source(&self) -> AttackSource {
        match self {
            Self::Roll => AttackSource::AirShockwave,
            Self::Jump => AttackSource::GroundShockwave,
            Self::No => AttackSource::UndodgeableShockwave,
        }
    }

    pub fn shockwave_attack_source_slice(&self) -> &[AttackSource] {
        match self {
            Self::Roll => &[AttackSource::AirShockwave],
            Self::Jump => &[AttackSource::GroundShockwave],
            Self::No => &[AttackSource::UndodgeableShockwave],
        }
    }

    pub fn explosion_shockwave_attack_source_slice(&self) -> &[AttackSource] {
        match self {
            Self::Roll => &[AttackSource::Explosion, AttackSource::AirShockwave],
            Self::Jump => &[AttackSource::Explosion, AttackSource::GroundShockwave],
            Self::No => &[AttackSource::Explosion, AttackSource::UndodgeableShockwave],
        }
    }
}
