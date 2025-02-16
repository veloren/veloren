use crate::{RtState, Rule, ai::NpcSystemData};
use common::{
    mounting::VolumePos,
    resources::{Time, TimeOfDay},
    rtsim::{Actor, NpcId, SiteId},
    terrain::SpriteKind,
};
use vek::*;
use world::{IndexRef, World};

pub trait Event: Clone + 'static {
    type SystemData<'a>;
}

pub struct EventCtx<'a, 'd, R: Rule, E: Event> {
    pub state: &'a RtState,
    pub rule: &'a mut R,
    pub event: &'a E,
    pub world: &'a World,
    pub index: IndexRef<'a>,
    pub system_data: &'a mut E::SystemData<'d>,
}

#[derive(Clone)]
pub struct OnSetup;
impl Event for OnSetup {
    type SystemData<'a> = ();
}

#[derive(Clone)]
pub struct OnTick {
    pub time_of_day: TimeOfDay,
    pub time: Time,
    pub tick: u64,
    pub dt: f32,
}
impl Event for OnTick {
    type SystemData<'a> = NpcSystemData<'a>;
}

#[derive(Clone)]
pub struct OnDeath {
    pub actor: Actor,
    pub wpos: Option<Vec3<f32>>,
    pub killer: Option<Actor>,
}
impl Event for OnDeath {
    type SystemData<'a> = ();
}

#[derive(Clone)]
pub struct OnHealthChange {
    pub actor: Actor,
    pub cause: Option<Actor>,
    pub new_health_fraction: f32,
}
impl Event for OnHealthChange {
    type SystemData<'a> = ();
}

#[derive(Clone)]
pub struct OnTheft {
    pub actor: Actor,
    pub wpos: Vec3<i32>,
    pub sprite: SpriteKind,
    pub site: Option<SiteId>,
}

impl Event for OnTheft {
    type SystemData<'a> = ();
}

#[derive(Clone)]
pub struct OnMountVolume {
    pub actor: Actor,
    pub pos: VolumePos<NpcId>,
}
impl Event for OnMountVolume {
    type SystemData<'a> = ();
}
