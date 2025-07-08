use crate::{
    combat::CombatEffect,
    comp::{PidController, ability::Dodgeable, beam},
    resources::{Secs, Time},
    states::basic_summon::BeamPillarIndicatorSpecifier,
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage, VecStorage};
use std::time::Duration;
use vek::Vec3;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Object {
    DeleteAfter {
        spawned_at: Time,
        timeout: Duration,
    },
    Portal {
        target: Vec3<f32>,
        requires_no_aggro: bool,
        buildup_time: Secs,
    },
    BeamPillar {
        spawned_at: Time,
        buildup_duration: Duration,
        attack_duration: Duration,
        beam_duration: Duration,
        radius: f32,
        height: f32,
        damage: f32,
        damage_effect: Option<CombatEffect>,
        dodgeable: Dodgeable,
        tick_rate: f32,
        specifier: beam::FrontendSpecifier,
        indicator_specifier: BeamPillarIndicatorSpecifier,
    },
    Crux {
        owner: Uid,
        scale: f32,
        range: f32,
        strength: f32,
        duration: Secs,
        #[serde(skip)]
        pid_controller: Option<PidController<fn(f32, f32) -> f32, 8>>,
    },
}

impl Component for Object {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

#[derive(Clone, Debug)]
pub struct PortalData {
    pub target: Vec3<f32>,
    pub requires_no_aggro: bool,
    pub buildup_time: Secs,
}

impl From<PortalData> for Object {
    fn from(
        PortalData {
            target,
            requires_no_aggro,
            buildup_time,
        }: PortalData,
    ) -> Self {
        Self::Portal {
            target,
            requires_no_aggro,
            buildup_time,
        }
    }
}
