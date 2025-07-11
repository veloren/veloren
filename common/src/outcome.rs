use crate::{
    DamageSource,
    combat::DamageContributor,
    comp::{self, item::ToolKind},
    terrain::SpriteKind,
    uid::Uid,
};
use comp::{UtteranceKind, beam, item::Reagent, poise::PoiseState, skillset::SkillGroupKind};
use hashbrown::HashSet;
use serde::{Deserialize, Serialize};
use vek::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HealthChangeInfo {
    pub amount: f32,
    pub precise: bool,
    pub target: Uid,
    pub by: Option<DamageContributor>,
    pub cause: Option<DamageSource>,
    pub instance: u64,
}

/// An outcome represents the final result of an instantaneous event. It implies
/// that said event has already occurred. It is not a request for that event to
/// occur, nor is it something that may be cancelled or otherwise altered. Its
/// primary purpose is to act as something for frontends (both server and
/// client) to listen to in order to receive feedback about events in the world.
#[derive(Clone, Debug, Serialize, Deserialize, strum::VariantNames)]
pub enum Outcome {
    Explosion {
        pos: Vec3<f32>,
        power: f32,
        radius: f32,
        is_attack: bool,
        reagent: Option<Reagent>, // How can we better define this?
    },
    Lightning {
        pos: Vec3<f32>,
    },
    ProjectileShot {
        pos: Vec3<f32>,
        body: comp::Body,
        vel: Vec3<f32>,
    },
    ProjectileHit {
        pos: Vec3<f32>,
        body: comp::Body,
        vel: Vec3<f32>,
        source: Option<Uid>,
        target: Option<Uid>,
    },
    Beam {
        pos: Vec3<f32>,
        specifier: beam::FrontendSpecifier,
    },
    ExpChange {
        uid: Uid,
        exp: u32,
        xp_pools: HashSet<SkillGroupKind>,
    },
    SkillPointGain {
        uid: Uid,
        skill_tree: SkillGroupKind,
        total_points: u16,
    },
    ComboChange {
        uid: Uid,
        combo: u32,
    },
    BreakBlock {
        pos: Vec3<i32>,
        tool: Option<ToolKind>,
        color: Option<Rgb<u8>>,
    },
    DamagedBlock {
        pos: Vec3<i32>,
        tool: Option<ToolKind>,
        stage_changed: bool,
    },
    SummonedCreature {
        pos: Vec3<f32>,
        body: comp::Body,
    },
    HealthChange {
        pos: Vec3<f32>,
        info: HealthChangeInfo,
    },
    Death {
        pos: Vec3<f32>,
    },
    Block {
        pos: Vec3<f32>,
        parry: bool,
        uid: Uid,
    },
    PoiseChange {
        pos: Vec3<f32>,
        state: PoiseState,
    },
    GroundSlam {
        pos: Vec3<f32>,
    },
    IceSpikes {
        pos: Vec3<f32>,
    },
    IceCrack {
        pos: Vec3<f32>,
    },
    FlashFreeze {
        pos: Vec3<f32>,
    },
    Steam {
        pos: Vec3<f32>,
    },
    LaserBeam {
        pos: Vec3<f32>,
    },
    CyclopsCharge {
        pos: Vec3<f32>,
    },
    FlamethrowerCharge {
        pos: Vec3<f32>,
    },
    FuseCharge {
        pos: Vec3<f32>,
    },
    TerracottaStatueCharge {
        pos: Vec3<f32>,
    },
    SurpriseEgg {
        pos: Vec3<f32>,
    },
    Utterance {
        pos: Vec3<f32>,
        body: comp::Body,
        kind: UtteranceKind,
    },
    Glider {
        pos: Vec3<f32>,
        wielded: bool,
    },
    SpriteDelete {
        pos: Vec3<i32>,
        sprite: SpriteKind,
    },
    SpriteUnlocked {
        pos: Vec3<i32>,
    },
    FailedSpriteUnlock {
        pos: Vec3<i32>,
    },
    Whoosh {
        pos: Vec3<f32>,
    },
    Swoosh {
        pos: Vec3<f32>,
    },
    Slash {
        pos: Vec3<f32>,
    },
    FireShockwave {
        pos: Vec3<f32>,
    },
    FireLowShockwave {
        pos: Vec3<f32>,
    },
    GroundDig {
        pos: Vec3<f32>,
    },
    PortalActivated {
        pos: Vec3<f32>,
    },
    TeleportedByPortal {
        pos: Vec3<f32>,
    },
    FromTheAshes {
        pos: Vec3<f32>,
    },
    ClayGolemDash {
        pos: Vec3<f32>,
    },
    Bleep {
        pos: Vec3<f32>,
    },
    Charge {
        pos: Vec3<f32>,
    },
    HeadLost {
        uid: Uid,
        head: usize,
    },
    Splash {
        vel: Vec3<f32>,
        pos: Vec3<f32>,
        mass: f32,
        kind: comp::fluid_dynamics::LiquidKind,
    },
    Transformation {
        pos: Vec3<f32>,
    },
    FirePillarIndicator {
        pos: Vec3<f32>,
        radius: f32,
    },
}

impl Outcome {
    pub fn get_pos(&self) -> Option<Vec3<f32>> {
        match self {
            Outcome::Explosion { pos, .. }
            // TODO: Include this, but allow it to be sent to clients when outside of the VD
            // | Outcome::Lightning { pos }
            | Outcome::ProjectileShot { pos, .. }
            | Outcome::ProjectileHit { pos, .. }
            | Outcome::Beam { pos, .. }
            | Outcome::SummonedCreature { pos, .. }
            | Outcome::HealthChange { pos, .. }
            | Outcome::Death { pos, .. }
            | Outcome::Block { pos, .. }
            | Outcome::PoiseChange { pos, .. }
            | Outcome::GroundSlam { pos }
            | Outcome::FlashFreeze { pos }
            | Outcome::Whoosh { pos }
            | Outcome::Swoosh { pos }
            | Outcome::Slash { pos }
            | Outcome::Bleep { pos }
            | Outcome::Charge { pos }
            | Outcome::IceSpikes { pos }
            | Outcome::Steam { pos }
            | Outcome::FireShockwave { pos }
            | Outcome::FireLowShockwave { pos }
            | Outcome::IceCrack { pos }
            | Outcome::Utterance { pos, .. }
            | Outcome::CyclopsCharge { pos }
            | Outcome::FlamethrowerCharge { pos }
            | Outcome::FuseCharge { pos }
            | Outcome::TerracottaStatueCharge { pos }
            | Outcome::SurpriseEgg { pos }
            | Outcome::LaserBeam { pos }
            | Outcome::GroundDig { pos }
            | Outcome::PortalActivated { pos }
            | Outcome::TeleportedByPortal { pos}
            | Outcome::FromTheAshes { pos }
            | Outcome::ClayGolemDash { pos }
            | Outcome::Glider { pos, .. }
            | Outcome::Splash { pos, .. }
            | Outcome::Transformation { pos }
            | Outcome::FirePillarIndicator { pos, .. } => Some(*pos),
            Outcome::BreakBlock { pos, .. }
            | Outcome::DamagedBlock { pos, .. }
            | Outcome::SpriteUnlocked { pos }
            | Outcome::SpriteDelete { pos, .. }
            | Outcome::FailedSpriteUnlock { pos } => Some(pos.map(|e| e as f32 + 0.5)),
            Outcome::ExpChange { .. }
            | Outcome::ComboChange { .. }
            | Outcome::Lightning { .. }
            | Outcome::SkillPointGain { .. }
            | Outcome::HeadLost { .. } => None,
        }
    }
}
