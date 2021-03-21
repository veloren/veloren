use crate::{
    comp::{humanoid, quadruped_low, quadruped_medium, quadruped_small, Body},
    path::Chaser,
    rtsim::RtSimController,
    trade::{PendingTrade, ReducedInventory, SiteId, SitePrices, TradeId, TradeResult},
    uid::Uid,
};
use specs::{Component, Entity as EcsEntity};
use specs_idvs::IdvStorage;
use std::collections::VecDeque;
use vek::*;

pub const DEFAULT_INTERACTION_TIME: f32 = 3.0;
pub const TRADE_INTERACTION_TIME: f32 = 300.0;

#[derive(Eq, PartialEq)]
pub enum Tactic {
    Melee,
    Axe,
    Hammer,
    Sword,
    Bow,
    Staff,
    StoneGolemBoss,
    CircleCharge { radius: u32, circle_time: u32 },
    QuadLowRanged,
    TailSlap,
    QuadLowQuick,
    QuadLowBasic,
    QuadLowBeam,
    QuadMedJump,
    QuadMedBasic,
    Lavadrake,
    Theropod,
    Turret,
    FixedTurret,
    RotatingTurret,
    Mindflayer,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Alignment {
    /// Wild animals and gentle giants
    Wild,
    /// Dungeon cultists and bandits
    Enemy,
    /// Friendly folk in villages
    Npc,
    /// Farm animals and pets of villagers
    Tame,
    /// Pets you've tamed with a collar
    Owned(Uid),
    /// Passive objects like training dummies
    Passive,
}

impl Alignment {
    // Always attacks
    pub fn hostile_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Enemy, Alignment::Enemy) => false,
            (Alignment::Enemy, Alignment::Wild) => false,
            (Alignment::Wild, Alignment::Enemy) => false,
            (Alignment::Wild, Alignment::Wild) => false,
            (Alignment::Npc, Alignment::Wild) => false,
            (Alignment::Npc, Alignment::Enemy) => true,
            (_, Alignment::Enemy) => true,
            (Alignment::Enemy, _) => true,
            _ => false,
        }
    }

    // Never attacks
    pub fn passive_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Enemy, Alignment::Enemy) => true,
            (Alignment::Owned(a), Alignment::Owned(b)) if a == b => true,
            (Alignment::Npc, Alignment::Npc) => true,
            (Alignment::Npc, Alignment::Tame) => true,
            (Alignment::Enemy, Alignment::Wild) => true,
            (Alignment::Wild, Alignment::Enemy) => true,
            (Alignment::Tame, Alignment::Npc) => true,
            (Alignment::Tame, Alignment::Tame) => true,
            (_, Alignment::Passive) => true,
            _ => false,
        }
    }

    // TODO: Remove this hack
    pub fn is_friendly_to_players(&self) -> bool {
        matches!(self, Alignment::Npc | Alignment::Tame | Alignment::Owned(_))
    }
}

impl Component for Alignment {
    type Storage = IdvStorage<Self>;
}

#[derive(Clone, Debug, Default)]
pub struct Psyche {
    pub aggro: f32, // 0.0 = always flees, 1.0 = always attacks, 0.5 = flee at 50% health
}

impl<'a> From<&'a Body> for Psyche {
    fn from(body: &'a Body) -> Self {
        Self {
            aggro: match body {
                Body::Humanoid(humanoid) => match humanoid.species {
                    humanoid::Species::Danari => 0.9,
                    humanoid::Species::Dwarf => 0.8,
                    humanoid::Species::Elf => 0.7,
                    humanoid::Species::Human => 0.6,
                    humanoid::Species::Orc => 0.9,
                    humanoid::Species::Undead => 0.9,
                },
                Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
                    quadruped_small::Species::Pig => 0.5,
                    quadruped_small::Species::Fox => 0.3,
                    quadruped_small::Species::Sheep => 0.5,
                    quadruped_small::Species::Boar => 0.8,
                    quadruped_small::Species::Jackalope => 0.4,
                    quadruped_small::Species::Skunk => 0.6,
                    quadruped_small::Species::Cat => 0.2,
                    quadruped_small::Species::Batfox => 0.6,
                    quadruped_small::Species::Raccoon => 0.4,
                    quadruped_small::Species::Quokka => 0.4,
                    quadruped_small::Species::Dodarock => 0.9,
                    quadruped_small::Species::Holladon => 1.0,
                    quadruped_small::Species::Hyena => 0.4,
                    quadruped_small::Species::Rabbit => 0.1,
                    quadruped_small::Species::Truffler => 0.8,
                    quadruped_small::Species::Frog => 0.4,
                    quadruped_small::Species::Hare => 0.2,
                    quadruped_small::Species::Goat => 0.5,
                    _ => 0.0,
                },
                Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                    quadruped_medium::Species::Tuskram => 0.7,
                    quadruped_medium::Species::Frostfang => 0.9,
                    quadruped_medium::Species::Mouflon => 0.7,
                    quadruped_medium::Species::Catoblepas => 0.8,
                    quadruped_medium::Species::Deer => 0.6,
                    quadruped_medium::Species::Hirdrasil => 0.7,
                    quadruped_medium::Species::Donkey => 0.7,
                    quadruped_medium::Species::Camel => 0.7,
                    quadruped_medium::Species::Zebra => 0.7,
                    quadruped_medium::Species::Antelope => 0.6,
                    quadruped_medium::Species::Horse => 0.7,
                    quadruped_medium::Species::Cattle => 0.7,
                    quadruped_medium::Species::Darkhound => 0.9,
                    quadruped_medium::Species::Dreadhorn => 0.8,
                    quadruped_medium::Species::Snowleopard => 0.7,
                    _ => 0.5,
                },
                Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                    quadruped_low::Species::Salamander => 0.7,
                    quadruped_low::Species::Monitor => 0.7,
                    quadruped_low::Species::Asp => 0.9,
                    quadruped_low::Species::Pangolin => 0.4,
                    _ => 0.6,
                },
                Body::BipedSmall(_) => 0.5,
                Body::BirdMedium(_) => 0.5,
                Body::BirdSmall(_) => 0.4,
                Body::FishMedium(_) => 0.15,
                Body::FishSmall(_) => 0.0,
                Body::BipedLarge(_) => 1.0,
                Body::Object(_) => 1.0,
                Body::Golem(_) => 1.0,
                Body::Theropod(_) => 1.0,
                Body::Dragon(_) => 1.0,
                Body::Ship(_) => 1.0,
            },
        }
    }
}

#[derive(Clone, Debug)]
/// Events that affect agent behavior from other entities/players/environment
pub enum AgentEvent {
    /// Engage in conversation with entity with Uid
    Talk(Uid),
    TradeInvite(Uid),
    FinishedTrade(TradeResult),
    UpdatePendingTrade(
        // this data structure is large so box it to keep AgentEvent small
        Box<(
            TradeId,
            PendingTrade,
            SitePrices,
            [Option<ReducedInventory>; 2],
        )>,
    ),
    // Add others here
}

#[derive(Clone, Debug)]
pub struct Target {
    pub target: EcsEntity,
    pub hostile: bool,
    pub selected_at: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Agent {
    pub rtsim_controller: RtSimController,
    pub patrol_origin: Option<Vec3<f32>>,
    pub target: Option<Target>,
    pub chaser: Chaser,
    /// Does the agent talk when e.g. hit by the player
    // TODO move speech patterns into a Behavior component
    pub can_speak: bool,
    pub trade_for_site: Option<SiteId>,
    pub trading: bool,
    pub psyche: Psyche,
    pub inbox: VecDeque<AgentEvent>,
    pub action_timer: f32,
    pub bearing: Vec2<f32>,
}

impl Agent {
    pub fn with_patrol_origin(mut self, origin: Vec3<f32>) -> Self {
        self.patrol_origin = Some(origin);
        self
    }

    pub fn with_destination(pos: Vec3<f32>) -> Self {
        Self {
            can_speak: true,
            psyche: Psyche { aggro: 1.0 },
            rtsim_controller: RtSimController::with_destination(pos),
            ..Default::default()
        }
    }

    pub fn new(
        patrol_origin: Option<Vec3<f32>>,
        can_speak: bool,
        trade_for_site: Option<SiteId>,
        body: &Body,
        no_flee: bool,
    ) -> Self {
        Agent {
            patrol_origin,
            can_speak,
            trade_for_site,
            psyche: if no_flee {
                Psyche { aggro: 1.0 }
            } else {
                Psyche::from(body)
            },
            ..Default::default()
        }
    }
}

impl Component for Agent {
    type Storage = IdvStorage<Self>;
}
