use crate::{
    comp::{
        biped_large, biped_small, bird_medium, humanoid, quadruped_low, quadruped_medium,
        quadruped_small, ship, Body, UtteranceKind,
    },
    path::Chaser,
    rtsim::{NpcInput, RtSimController},
    trade::{PendingTrade, ReducedInventory, SiteId, SitePrices, TradeId, TradeResult},
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, Entity as EcsEntity};
use std::{collections::VecDeque, fmt};
use strum::{EnumIter, IntoEnumIterator};
use vek::*;

use super::{dialogue::Subject, Pos};

pub const DEFAULT_INTERACTION_TIME: f32 = 3.0;
pub const TRADE_INTERACTION_TIME: f32 = 300.0;
const SECONDS_BEFORE_FORGET_SOUNDS: f64 = 180.0;

//intentionally very few concurrent action state variables are allowed. This is
// to keep the complexity of our AI from getting too large, too quickly.
// Originally I was going to provide 30 of these, but if we decide later that
// this is too many and somebody is already using 30 in one of their AI, it will
// be difficult to go back.

/// The number of timers that a single Action node can track concurrently
/// Define constants within a given action node to index between them.
const ACTIONSTATE_NUMBER_OF_CONCURRENT_TIMERS: usize = 5;
/// The number of float counters that a single Action node can track
/// concurrently Define constants within a given action node to index between
/// them.
const ACTIONSTATE_NUMBER_OF_CONCURRENT_COUNTERS: usize = 5;
/// The number of integer counters that a single Action node can track
/// concurrently Define constants within a given action node to index between
/// them.
const ACTIONSTATE_NUMBER_OF_CONCURRENT_INT_COUNTERS: usize = 5;
/// The number of booleans that a single Action node can track concurrently
/// Define constants within a given action node to index between them.
const ACTIONSTATE_NUMBER_OF_CONCURRENT_CONDITIONS: usize = 5;
/// The number of positions that can be remembered by an agent
const ACTIONSTATE_NUMBER_OF_CONCURRENT_POSITIONS: usize = 5;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mark {
    Merchant,
    Guard,
}

impl Alignment {
    // Always attacks
    pub fn hostile_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Passive, _) => false,
            (_, Alignment::Passive) => false,
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

    // Usually never attacks
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

    // Never attacks
    pub fn friendly_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Enemy, Alignment::Enemy) => true,
            (Alignment::Owned(a), Alignment::Owned(b)) if a == b => true,
            (Alignment::Npc, Alignment::Npc) => true,
            (Alignment::Npc, Alignment::Tame) => true,
            (Alignment::Tame, Alignment::Npc) => true,
            (Alignment::Tame, Alignment::Tame) => true,
            (_, Alignment::Passive) => true,
            _ => false,
        }
    }
}

impl Component for Alignment {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct BehaviorCapability: u8 {
        const SPEAK = 0b00000001;
        const TRADE = 0b00000010;
    }
}
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct BehaviorState: u8 {
        const TRADING        = 0b00000001;
        const TRADING_ISSUER = 0b00000010;
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub enum TradingBehavior {
    #[default]
    None,
    RequireBalanced {
        trade_site: SiteId,
    },
    AcceptFood,
}

impl TradingBehavior {
    fn can_trade(&self, alignment: Option<Alignment>, counterparty: Uid) -> bool {
        match self {
            TradingBehavior::RequireBalanced { .. } => true,
            TradingBehavior::AcceptFood => alignment == Some(Alignment::Owned(counterparty)),
            TradingBehavior::None => false,
        }
    }
}

/// # Behavior Component
/// This component allow an Entity to register one or more behavior tags.
/// These tags act as flags of what an Entity can do, or what it is doing.
/// Behaviors Tags can be added and removed as the Entity lives, to update its
/// state when needed
#[derive(Default, Copy, Clone, Debug)]
pub struct Behavior {
    capabilities: BehaviorCapability,
    state: BehaviorState,
    pub trading_behavior: TradingBehavior,
}

impl From<BehaviorCapability> for Behavior {
    fn from(capabilities: BehaviorCapability) -> Self {
        Behavior {
            capabilities,
            state: BehaviorState::default(),
            trading_behavior: TradingBehavior::None,
        }
    }
}

impl Behavior {
    /// Builder function
    /// Set capabilities if Option is Some
    #[must_use]
    pub fn maybe_with_capabilities(
        mut self,
        maybe_capabilities: Option<BehaviorCapability>,
    ) -> Self {
        if let Some(capabilities) = maybe_capabilities {
            self.allow(capabilities)
        }
        self
    }

    /// Builder function
    /// Set trade_site if Option is Some
    #[must_use]
    pub fn with_trade_site(mut self, trade_site: Option<SiteId>) -> Self {
        if let Some(trade_site) = trade_site {
            self.trading_behavior = TradingBehavior::RequireBalanced { trade_site };
        }
        self
    }

    /// Set capabilities to the Behavior
    pub fn allow(&mut self, capabilities: BehaviorCapability) {
        self.capabilities.set(capabilities, true)
    }

    /// Unset capabilities to the Behavior
    pub fn deny(&mut self, capabilities: BehaviorCapability) {
        self.capabilities.set(capabilities, false)
    }

    /// Check if the Behavior is able to do something
    pub fn can(&self, capabilities: BehaviorCapability) -> bool {
        self.capabilities.contains(capabilities)
    }

    /// Check if the Behavior is able to trade
    pub fn can_trade(&self, alignment: Option<Alignment>, counterparty: Uid) -> bool {
        self.trading_behavior.can_trade(alignment, counterparty)
    }

    /// Set a state to the Behavior
    pub fn set(&mut self, state: BehaviorState) { self.state.set(state, true) }

    /// Unset a state to the Behavior
    pub fn unset(&mut self, state: BehaviorState) { self.state.set(state, false) }

    /// Check if the Behavior has a specific state
    pub fn is(&self, state: BehaviorState) -> bool { self.state.contains(state) }

    /// Get the trade site at which this behavior evaluates prices, if it does
    pub fn trade_site(&self) -> Option<SiteId> {
        if let TradingBehavior::RequireBalanced { trade_site } = self.trading_behavior {
            Some(trade_site)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Psyche {
    /// The proportion of health below which entities will start fleeing.
    /// 0.0 = never flees, 1.0 = always flees, 0.5 = flee at 50% health.
    pub flee_health: f32,
    /// The distance below which the agent will see enemies if it has line of
    /// sight.
    pub sight_dist: f32,
    /// The distance below which the agent can hear enemies without seeing them.
    pub listen_dist: f32,
    /// The distance below which the agent will attack enemies. Should be lower
    /// than `sight_dist`. `None` implied that the agent is always aggro
    /// towards enemies that it is aware of.
    pub aggro_dist: Option<f32>,
    /// A factor that controls how much further an agent will wander when in the
    /// idle state. `1.0` is normal.
    pub idle_wander_factor: f32,
    /// Aggro range is multiplied by this factor. `1.0` is normal.
    ///
    /// This includes scaling the effective `sight_dist` and `listen_dist`
    /// when finding new targets to attack, adjusting the strength of
    /// wandering behavior in the idle state, and scaling `aggro_dist` in
    /// certain situations.
    pub aggro_range_multiplier: f32,
}

impl<'a> From<&'a Body> for Psyche {
    fn from(body: &'a Body) -> Self {
        Self {
            flee_health: match body {
                Body::Humanoid(humanoid) => match humanoid.species {
                    humanoid::Species::Danari => 0.4,
                    humanoid::Species::Dwarf => 0.3,
                    humanoid::Species::Elf => 0.4,
                    humanoid::Species::Human => 0.4,
                    humanoid::Species::Orc => 0.3,
                    humanoid::Species::Draugr => 0.3,
                },
                Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
                    quadruped_small::Species::Pig => 0.5,
                    quadruped_small::Species::Fox => 0.3,
                    quadruped_small::Species::Sheep => 0.25,
                    quadruped_small::Species::Boar => 0.1,
                    quadruped_small::Species::Skunk => 0.4,
                    quadruped_small::Species::Cat => 0.99,
                    quadruped_small::Species::Batfox => 0.1,
                    quadruped_small::Species::Raccoon => 0.4,
                    quadruped_small::Species::Hyena => 0.1,
                    quadruped_small::Species::Dog => 0.8,
                    quadruped_small::Species::Rabbit | quadruped_small::Species::Jackalope => 0.25,
                    quadruped_small::Species::Truffler => 0.08,
                    quadruped_small::Species::Hare => 0.3,
                    quadruped_small::Species::Goat => 0.3,
                    quadruped_small::Species::Porcupine => 0.2,
                    quadruped_small::Species::Turtle => 0.4,
                    quadruped_small::Species::Beaver => 0.2,
                    // FIXME: This is to balance for enemy rats in dungeons
                    // Normal rats should probably always flee.
                    quadruped_small::Species::Rat
                    | quadruped_small::Species::TreantSapling
                    | quadruped_small::Species::Holladon
                    | quadruped_small::Species::MossySnail => 0.0,
                    _ => 1.0,
                },
                Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                    // T1
                    quadruped_medium::Species::Antelope => 0.15,
                    quadruped_medium::Species::Donkey => 0.05,
                    quadruped_medium::Species::Horse => 0.15,
                    quadruped_medium::Species::Mouflon => 0.1,
                    quadruped_medium::Species::Zebra => 0.1,
                    // T2
                    // Should probably not have non-grouped, hostile animals flee until fleeing is
                    // improved.
                    quadruped_medium::Species::Barghest
                    | quadruped_medium::Species::Bear
                    | quadruped_medium::Species::Bristleback
                    | quadruped_medium::Species::Bonerattler => 0.0,
                    quadruped_medium::Species::Cattle => 0.1,
                    quadruped_medium::Species::Frostfang => 0.07,
                    quadruped_medium::Species::Grolgar => 0.0,
                    quadruped_medium::Species::Highland => 0.05,
                    quadruped_medium::Species::Kelpie => 0.35,
                    quadruped_medium::Species::Lion => 0.0,
                    quadruped_medium::Species::Moose => 0.15,
                    quadruped_medium::Species::Panda => 0.35,
                    quadruped_medium::Species::Saber
                    | quadruped_medium::Species::Tarasque
                    | quadruped_medium::Species::Tiger => 0.0,
                    quadruped_medium::Species::Tuskram => 0.1,
                    quadruped_medium::Species::Wolf => 0.2,
                    quadruped_medium::Species::Yak => 0.09,
                    // T3A
                    quadruped_medium::Species::Akhlut
                    | quadruped_medium::Species::Catoblepas
                    | quadruped_medium::Species::ClaySteed
                    | quadruped_medium::Species::Dreadhorn
                    | quadruped_medium::Species::Hirdrasil
                    | quadruped_medium::Species::Mammoth
                    | quadruped_medium::Species::Ngoubou
                    | quadruped_medium::Species::Roshwalr => 0.0,
                    _ => 0.15,
                },
                Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                    // T1
                    quadruped_low::Species::Pangolin => 0.3,
                    // T2,
                    quadruped_low::Species::Tortoise => 0.1,
                    // There are a lot of hostile, solo entities.
                    _ => 0.0,
                },
                Body::BipedSmall(biped_small) => match biped_small.species {
                    biped_small::Species::Gnarling => 0.2,
                    biped_small::Species::Adlet => 0.2,
                    biped_small::Species::Haniwa => 0.1,
                    biped_small::Species::Sahagin => 0.1,
                    biped_small::Species::Myrmidon => 0.0,
                    biped_small::Species::Husk
                    | biped_small::Species::Boreal
                    | biped_small::Species::Clockwork
                    | biped_small::Species::Flamekeeper
                    | biped_small::Species::Irrwurz => 0.0,

                    _ => 0.5,
                },
                Body::BirdMedium(bird_medium) => match bird_medium.species {
                    bird_medium::Species::SnowyOwl => 0.4,
                    bird_medium::Species::HornedOwl => 0.4,
                    bird_medium::Species::Duck => 0.6,
                    bird_medium::Species::Cockatiel => 0.6,
                    bird_medium::Species::Chicken => 0.5,
                    bird_medium::Species::Bat => 0.1,
                    bird_medium::Species::Penguin => 0.5,
                    bird_medium::Species::Goose => 0.4,
                    bird_medium::Species::Peacock => 0.3,
                    bird_medium::Species::Eagle => 0.2,
                    bird_medium::Species::Parrot => 0.8,
                    bird_medium::Species::Crow => 0.4,
                    bird_medium::Species::Dodo => 0.8,
                    bird_medium::Species::Parakeet => 0.8,
                    bird_medium::Species::Puffin => 0.8,
                    bird_medium::Species::Toucan => 0.4,
                },
                Body::BirdLarge(_) => 0.0,
                Body::FishSmall(_) => 1.0,
                Body::FishMedium(_) => 0.75,
                Body::BipedLarge(_) => 0.0,
                Body::Object(_) => 0.0,
                Body::ItemDrop(_) => 0.0,
                Body::Golem(_) => 0.0,
                Body::Theropod(_) => 0.0,
                Body::Ship(_) => 0.0,
                Body::Dragon(_) => 0.0,
                Body::Arthropod(_) => 0.0,
                Body::Crustacean(_) => 0.0,
            },
            sight_dist: match body {
                Body::BirdLarge(_) => 250.0,
                Body::BipedLarge(biped_large) => match biped_large.species {
                    biped_large::Species::Gigasfrost => 200.0,
                    _ => 100.0,
                },
                _ => 40.0,
            },
            listen_dist: 30.0,
            aggro_dist: match body {
                Body::Humanoid(_) => Some(20.0),
                _ => None, // Always aggressive if detected
            },
            idle_wander_factor: 1.0,
            aggro_range_multiplier: 1.0,
        }
    }
}

impl Psyche {
    /// The maximum distance that targets to attack might be detected by this
    /// agent.
    pub fn search_dist(&self) -> f32 {
        self.sight_dist.max(self.listen_dist) * self.aggro_range_multiplier
    }
}

#[derive(Clone, Debug)]
/// Events that affect agent behavior from other entities/players/environment
pub enum AgentEvent {
    /// Engage in conversation with entity with Uid
    Talk(Uid, Subject),
    TradeInvite(Uid),
    TradeAccepted(Uid),
    FinishedTrade(TradeResult),
    UpdatePendingTrade(
        // This data structure is large so box it to keep AgentEvent small
        Box<(
            TradeId,
            PendingTrade,
            SitePrices,
            [Option<ReducedInventory>; 2],
        )>,
    ),
    ServerSound(Sound),
    Hurt,
}

#[derive(Copy, Clone, Debug)]
pub struct Sound {
    pub kind: SoundKind,
    pub pos: Vec3<f32>,
    pub vol: f32,
    pub time: f64,
}

impl Sound {
    pub fn new(kind: SoundKind, pos: Vec3<f32>, vol: f32, time: f64) -> Self {
        Sound {
            kind,
            pos,
            vol,
            time,
        }
    }

    #[must_use]
    pub fn with_new_vol(mut self, new_vol: f32) -> Self {
        self.vol = new_vol;

        self
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SoundKind {
    Unknown,
    Utterance(UtteranceKind, Body),
    Movement,
    Melee,
    Projectile,
    Explosion,
    Beam,
    Shockwave,
    Trap,
}

#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub target: EcsEntity,
    /// Whether the target is hostile
    pub hostile: bool,
    /// The time at which the target was selected
    pub selected_at: f64,
    /// Whether the target has come close enough to trigger aggro.
    pub aggro_on: bool,
    pub last_known_pos: Option<Vec3<f32>>,
}

impl Target {
    pub fn new(
        target: EcsEntity,
        hostile: bool,
        selected_at: f64,
        aggro_on: bool,
        last_known_pos: Option<Vec3<f32>>,
    ) -> Self {
        Self {
            target,
            hostile,
            selected_at,
            aggro_on,
            last_known_pos,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, EnumIter)]
pub enum TimerAction {
    Interact,
}

/// A time used for managing agent-related timeouts. The timer is designed to
/// keep track of the start of any number of previous actions. However,
/// starting/progressing an action will end previous actions. Therefore, the
/// timer should be used for actions that are mutually-exclusive.
#[derive(Clone, Debug)]
pub struct Timer {
    action_starts: Vec<Option<f64>>,
    last_action: Option<TimerAction>,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            action_starts: TimerAction::iter().map(|_| None).collect(),
            last_action: None,
        }
    }
}

impl Timer {
    fn idx_for(action: TimerAction) -> usize {
        TimerAction::iter()
            .enumerate()
            .find(|(_, a)| a == &action)
            .unwrap()
            .0 // Can't fail, EnumIter is exhaustive
    }

    /// Reset the timer for the given action, returning true if the timer was
    /// not already reset.
    pub fn reset(&mut self, action: TimerAction) -> bool {
        self.action_starts[Self::idx_for(action)].take().is_some()
    }

    /// Start the timer for the given action, even if it was already started.
    pub fn start(&mut self, time: f64, action: TimerAction) {
        self.action_starts[Self::idx_for(action)] = Some(time);
        self.last_action = Some(action);
    }

    /// Continue timing the given action, starting it if it was not already
    /// started.
    pub fn progress(&mut self, time: f64, action: TimerAction) {
        if self.last_action != Some(action) {
            self.start(time, action);
        }
    }

    /// Return the time that the given action was last performed at.
    pub fn time_of_last(&self, action: TimerAction) -> Option<f64> {
        self.action_starts[Self::idx_for(action)]
    }

    /// Return `true` if the time since the action was last started exceeds the
    /// given timeout.
    pub fn time_since_exceeds(&self, time: f64, action: TimerAction, timeout: f64) -> bool {
        self.time_of_last(action)
            .map_or(true, |last_time| (time - last_time).max(0.0) > timeout)
    }

    /// Return `true` while the time since the action was last started is less
    /// than the given period. Once the time has elapsed, reset the timer.
    pub fn timeout_elapsed(
        &mut self,
        time: f64,
        action: TimerAction,
        timeout: f64,
    ) -> Option<bool> {
        if self.time_since_exceeds(time, action, timeout) {
            Some(self.reset(action))
        } else {
            self.progress(time, action);
            None
        }
    }
}

/// For use with the builder pattern <https://doc.rust-lang.org/1.0.0/style/ownership/builders.html>
#[derive(Clone, Debug)]
pub struct Agent {
    pub rtsim_controller: RtSimController,
    pub patrol_origin: Option<Vec3<f32>>,
    pub target: Option<Target>,
    pub chaser: Chaser,
    pub behavior: Behavior,
    pub psyche: Psyche,
    pub inbox: VecDeque<AgentEvent>,
    pub combat_state: ActionState,
    pub behavior_state: ActionState,
    pub timer: Timer,
    pub bearing: Vec2<f32>,
    pub sounds_heard: Vec<Sound>,
    pub position_pid_controller: Option<PidController<fn(Vec3<f32>, Vec3<f32>) -> f32, 16>>,
    /// Position from which to flee. Intended to be the agent's position plus a
    /// random position offset, to be used when a random flee direction is
    /// required and reset each time the flee timer is reset.
    pub flee_from_pos: Option<Pos>,
    pub awareness: Awareness,
    pub stay_pos: Option<Pos>,
    /// Inputs sent up to rtsim
    pub rtsim_outbox: Option<VecDeque<NpcInput>>,
}

#[derive(Clone, Debug)]
/// Always clamped between `0.0` and `1.0`.
pub struct Awareness {
    level: f32,
    reached: bool,
}
impl fmt::Display for Awareness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{:.2}", self.level) }
}
impl Awareness {
    const ALERT: f32 = 1.0;
    const HIGH: f32 = 0.6;
    const LOW: f32 = 0.1;
    const MEDIUM: f32 = 0.3;
    const UNAWARE: f32 = 0.0;

    pub fn new(level: f32) -> Self {
        Self {
            level: level.clamp(Self::UNAWARE, Self::ALERT),
            reached: false,
        }
    }

    /// The level of awareness as a decimal.
    pub fn level(&self) -> f32 { self.level }

    /// The level of awareness in English. To see if awareness has been fully
    /// reached, use `self.reached()`.
    pub fn state(&self) -> AwarenessState {
        if self.level == Self::ALERT {
            AwarenessState::Alert
        } else if self.level.is_between(Self::HIGH, Self::ALERT) {
            AwarenessState::High
        } else if self.level.is_between(Self::MEDIUM, Self::HIGH) {
            AwarenessState::Medium
        } else if self.level.is_between(Self::LOW, Self::MEDIUM) {
            AwarenessState::Low
        } else {
            AwarenessState::Unaware
        }
    }

    /// Awareness was reached at some point and has not been reset.
    pub fn reached(&self) -> bool { self.reached }

    pub fn change_by(&mut self, amount: f32) {
        self.level = (self.level + amount).clamp(Self::UNAWARE, Self::ALERT);

        if self.state() == AwarenessState::Alert {
            self.reached = true;
        } else if self.state() == AwarenessState::Unaware {
            self.reached = false;
        }
    }

    pub fn set_maximally_aware(&mut self) {
        self.reached = true;
        self.level = Self::ALERT;
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Eq)]
pub enum AwarenessState {
    Unaware = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Alert = 4,
}

/// State persistence object for the behavior tree
/// Allows for state to be stored between subsequent, sequential calls of a
/// single action node. If the executed action node changes between ticks, then
/// the state should be considered lost.
#[derive(Clone, Debug, Default)]
pub struct ActionState {
    pub timers: [f32; ACTIONSTATE_NUMBER_OF_CONCURRENT_TIMERS],
    pub counters: [f32; ACTIONSTATE_NUMBER_OF_CONCURRENT_COUNTERS],
    pub conditions: [bool; ACTIONSTATE_NUMBER_OF_CONCURRENT_CONDITIONS],
    pub int_counters: [u8; ACTIONSTATE_NUMBER_OF_CONCURRENT_INT_COUNTERS],
    pub positions: [Option<Vec3<f32>>; ACTIONSTATE_NUMBER_OF_CONCURRENT_POSITIONS],
    pub initialized: bool,
}

impl Agent {
    /// Instantiates agent from body using the body's psyche
    pub fn from_body(body: &Body) -> Self {
        Agent {
            rtsim_controller: RtSimController::default(),
            patrol_origin: None,
            target: None,
            chaser: Chaser::default(),
            behavior: Behavior::default(),
            psyche: Psyche::from(body),
            inbox: VecDeque::new(),
            combat_state: ActionState::default(),
            behavior_state: ActionState::default(),
            timer: Timer::default(),
            bearing: Vec2::zero(),
            sounds_heard: Vec::new(),
            position_pid_controller: None,
            flee_from_pos: None,
            stay_pos: None,
            awareness: Awareness::new(0.0),
            rtsim_outbox: None,
        }
    }

    #[must_use]
    pub fn with_patrol_origin(mut self, origin: Vec3<f32>) -> Self {
        self.patrol_origin = Some(origin);
        self
    }

    #[must_use]
    pub fn with_behavior(mut self, behavior: Behavior) -> Self {
        self.behavior = behavior;
        self
    }

    #[must_use]
    pub fn with_no_flee_if(mut self, condition: bool) -> Self {
        if condition {
            self.psyche.flee_health = 0.0;
        }
        self
    }

    pub fn set_no_flee(&mut self) { self.psyche.flee_health = 0.0; }

    // FIXME: Only one of *three* things in this method sets a location.
    #[must_use]
    pub fn with_destination(mut self, pos: Vec3<f32>) -> Self {
        self.psyche.flee_health = 0.0;
        self.rtsim_controller = RtSimController::with_destination(pos);
        self.behavior.allow(BehaviorCapability::SPEAK);
        self
    }

    #[must_use]
    pub fn with_idle_wander_factor(mut self, idle_wander_factor: f32) -> Self {
        self.psyche.idle_wander_factor = idle_wander_factor;
        self
    }

    pub fn with_aggro_range_multiplier(mut self, aggro_range_multiplier: f32) -> Self {
        self.psyche.aggro_range_multiplier = aggro_range_multiplier;
        self
    }

    #[must_use]
    pub fn with_position_pid_controller(
        mut self,
        pid: PidController<fn(Vec3<f32>, Vec3<f32>) -> f32, 16>,
    ) -> Self {
        self.position_pid_controller = Some(pid);
        self
    }

    /// Makes agent aggressive without warning
    #[must_use]
    pub fn with_aggro_no_warn(mut self) -> Self {
        self.psyche.aggro_dist = None;
        self
    }

    pub fn forget_old_sounds(&mut self, time: f64) {
        if !self.sounds_heard.is_empty() {
            // Keep (retain) only newer sounds
            self.sounds_heard
                .retain(|&sound| time - sound.time <= SECONDS_BEFORE_FORGET_SOUNDS);
        }
    }

    pub fn allowed_to_speak(&self) -> bool { self.behavior.can(BehaviorCapability::SPEAK) }
}

impl Component for Agent {
    type Storage = specs::DenseVecStorage<Self>;
}

#[cfg(test)]
mod tests {
    use super::{humanoid, Agent, Behavior, BehaviorCapability, BehaviorState, Body};

    /// Test to verify that Behavior is working correctly at its most basic
    /// usages
    #[test]
    pub fn behavior_basic() {
        let mut b = Behavior::default();
        // test capabilities
        assert!(!b.can(BehaviorCapability::SPEAK));
        b.allow(BehaviorCapability::SPEAK);
        assert!(b.can(BehaviorCapability::SPEAK));
        b.deny(BehaviorCapability::SPEAK);
        assert!(!b.can(BehaviorCapability::SPEAK));
        // test states
        assert!(!b.is(BehaviorState::TRADING));
        b.set(BehaviorState::TRADING);
        assert!(b.is(BehaviorState::TRADING));
        b.unset(BehaviorState::TRADING);
        assert!(!b.is(BehaviorState::TRADING));
        // test `from`
        let b = Behavior::from(BehaviorCapability::SPEAK);
        assert!(b.can(BehaviorCapability::SPEAK));
    }

    /// Makes agent flee
    #[test]
    pub fn enable_flee() {
        let body = Body::Humanoid(humanoid::Body::random());
        let mut agent = Agent::from_body(&body);

        agent.psyche.flee_health = 1.0;
        agent = agent.with_no_flee_if(false);
        assert_eq!(agent.psyche.flee_health, 1.0);
    }

    /// Makes agent not flee
    #[test]
    pub fn set_no_flee() {
        let body = Body::Humanoid(humanoid::Body::random());
        let mut agent = Agent::from_body(&body);

        agent.psyche.flee_health = 1.0;
        agent.set_no_flee();
        assert_eq!(agent.psyche.flee_health, 0.0);
    }

    #[test]
    pub fn with_aggro_no_warn() {
        let body = Body::Humanoid(humanoid::Body::random());
        let mut agent = Agent::from_body(&body);

        agent.psyche.aggro_dist = Some(1.0);
        agent = agent.with_aggro_no_warn();
        assert_eq!(agent.psyche.aggro_dist, None);
    }
}

/// PID controllers are used for automatically adapting nonlinear controls (like
/// buoyancy for airships) to target specific outcomes (i.e. a specific height)
#[derive(Clone)]
pub struct PidController<F: Fn(Vec3<f32>, Vec3<f32>) -> f32, const NUM_SAMPLES: usize> {
    /// The coefficient of the proportional term
    pub kp: f32,
    /// The coefficient of the integral term
    pub ki: f32,
    /// The coefficient of the derivative term
    pub kd: f32,
    /// The setpoint that the process has as its goal
    pub sp: Vec3<f32>,
    /// A ring buffer of the last NUM_SAMPLES measured process variables
    pv_samples: [(f64, Vec3<f32>); NUM_SAMPLES],
    /// The index into the ring buffer of process variables
    pv_idx: usize,
    /// The total integral error
    integral_error: f64,
    /// The error function, to change how the difference between the setpoint
    /// and process variables are calculated
    e: F,
}

impl<F: Fn(Vec3<f32>, Vec3<f32>) -> f32, const NUM_SAMPLES: usize> fmt::Debug
    for PidController<F, NUM_SAMPLES>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PidController")
            .field("kp", &self.kp)
            .field("ki", &self.ki)
            .field("kd", &self.kd)
            .field("sp", &self.sp)
            .field("pv_samples", &self.pv_samples)
            .field("pv_idx", &self.pv_idx)
            .finish()
    }
}

impl<F: Fn(Vec3<f32>, Vec3<f32>) -> f32, const NUM_SAMPLES: usize> PidController<F, NUM_SAMPLES> {
    /// Constructs a PidController with the specified weights, setpoint,
    /// starting time, and error function
    pub fn new(kp: f32, ki: f32, kd: f32, sp: Vec3<f32>, time: f64, e: F) -> Self {
        Self {
            kp,
            ki,
            kd,
            sp,
            pv_samples: [(time, sp); NUM_SAMPLES],
            pv_idx: 0,
            integral_error: 0.0,
            e,
        }
    }

    /// Adds a measurement of the process variable to the ringbuffer
    pub fn add_measurement(&mut self, time: f64, pv: Vec3<f32>) {
        self.pv_idx += 1;
        self.pv_idx %= NUM_SAMPLES;
        self.pv_samples[self.pv_idx] = (time, pv);
        self.update_integral_err();
    }

    /// The amount to set the control variable to is a weighed sum of the
    /// proportional error, the integral error, and the derivative error.
    /// https://en.wikipedia.org/wiki/PID_controller#Mathematical_form
    pub fn calc_err(&self) -> f32 {
        self.kp * self.proportional_err()
            + self.ki * self.integral_err()
            + self.kd * self.derivative_err()
    }

    /// The proportional error is the error function applied to the set point
    /// and the most recent process variable measurement
    pub fn proportional_err(&self) -> f32 { (self.e)(self.sp, self.pv_samples[self.pv_idx].1) }

    /// The integral error is the error function integrated over all previous
    /// values, updated per point. The trapezoid rule for numerical integration
    /// was chosen because it's fairly easy to calculate and sufficiently
    /// accurate. https://en.wikipedia.org/wiki/Trapezoidal_rule#Uniform_grid
    pub fn integral_err(&self) -> f32 { self.integral_error as f32 }

    fn update_integral_err(&mut self) {
        let f = |x| (self.e)(self.sp, x) as f64;
        let (a, x0) = self.pv_samples[(self.pv_idx + NUM_SAMPLES - 1) % NUM_SAMPLES];
        let (b, x1) = self.pv_samples[self.pv_idx];
        let dx = b - a;
        // Discard updates with too long between them, likely caused by either
        // initialization or latency, since they're likely to be spurious
        if dx < 5.0 {
            self.integral_error += dx * (f(x1) + f(x0)) / 2.0;
        }
    }

    /// The derivative error is the numerical derivative of the error function
    /// based on the most recent 2 samples. Using more than 2 samples might
    /// improve the accuracy of the estimate of the derivative, but it would be
    /// an estimate of the derivative error further in the past.
    /// https://en.wikipedia.org/wiki/Numerical_differentiation#Finite_differences
    pub fn derivative_err(&self) -> f32 {
        let f = |x| (self.e)(self.sp, x);
        let (a, x0) = self.pv_samples[(self.pv_idx + NUM_SAMPLES - 1) % NUM_SAMPLES];
        let (b, x1) = self.pv_samples[self.pv_idx];
        let h = b - a;
        (f(x1) - f(x0)) / h as f32
    }
}

/// Get the PID coefficients associated with some Body, since it will likely
/// need to be tuned differently for each body type
pub fn pid_coefficients(body: &Body) -> Option<(f32, f32, f32)> {
    // A pure-proportional controller is { kp: 1.0, ki: 0.0, kd: 0.0 }
    match body {
        Body::Ship(ship::Body::DefaultAirship) => {
            let kp = 1.0;
            let ki = 0.1;
            let kd = 1.2;
            Some((kp, ki, kd))
        },
        Body::Ship(ship::Body::AirBalloon) => {
            let kp = 1.0;
            let ki = 0.1;
            let kd = 0.8;
            Some((kp, ki, kd))
        },
        _ => None,
    }
}
