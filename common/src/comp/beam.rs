use crate::{combat::Attack, resources::Secs};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, Entity as EcsEntity};
use vek::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Beam {
    pub attack: Attack,
    pub end_radius: f32,
    pub range: f32,
    pub duration: Secs,
    pub tick_dur: Secs,
    pub specifier: FrontendSpecifier,
    pub bezier: QuadraticBezier3<f32>,
    #[serde(skip)]
    pub hit_entities: Vec<EcsEntity>,
}

impl Component for Beam {
    type Storage = DerefFlaggedStorage<Self, specs::DenseVecStorage<Self>>;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrontendSpecifier {
    Flamethrower,
    LifestealBeam,
    Cultist,
    ClayGolem,
    Bubbles,
    Steam,
    Frost,
    WebStrand,
    Poison,
    Ink,
    Lightning,
}
