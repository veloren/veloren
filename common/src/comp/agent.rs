use crate::{
    comp::{humanoid, quadruped_low, quadruped_medium, quadruped_small, ship, Body},
    path::Chaser,
    rtsim::RtSimController,
    trade::{PendingTrade, ReducedInventory, SiteId, SitePrices, TradeId, TradeResult},
    uid::Uid,
};
use specs::{Component, Entity as EcsEntity};
use specs_idvs::IdvStorage;
use std::{collections::VecDeque, fmt};
use vek::*;

use super::dialogue::Subject;

pub const DEFAULT_INTERACTION_TIME: f32 = 3.0;
pub const TRADE_INTERACTION_TIME: f32 = 300.0;
pub const MAX_LISTEN_DIST: f32 = 100.0;

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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Mark {
    Merchant,
    Guard,
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

bitflags::bitflags! {
    #[derive(Default)]
    pub struct BehaviorCapability: u8 {
        const SPEAK = 0b00000001;
    }
}
bitflags::bitflags! {
    #[derive(Default)]
    pub struct BehaviorState: u8 {
        const TRADING        = 0b00000001;
        const TRADING_ISSUER = 0b00000010;
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
    pub trade_site: Option<SiteId>,
}

impl From<BehaviorCapability> for Behavior {
    fn from(capabilities: BehaviorCapability) -> Self {
        Behavior {
            capabilities,
            state: BehaviorState::default(),
            trade_site: None,
        }
    }
}

impl Behavior {
    /// Builder function  
    /// Set capabilities if Option is Some
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
    pub fn with_trade_site(mut self, trade_site: Option<SiteId>) -> Self {
        self.trade_site = trade_site;
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
    pub fn can_trade(&self) -> bool { self.trade_site.is_some() }

    /// Set a state to the Behavior
    pub fn set(&mut self, state: BehaviorState) { self.state.set(state, true) }

    /// Unset a state to the Behavior
    pub fn unset(&mut self, state: BehaviorState) { self.state.set(state, false) }

    /// Check if the Behavior has a specific state
    pub fn is(&self, state: BehaviorState) -> bool { self.state.contains(state) }
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
                Body::BirdLarge(_) => 0.9,
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

    pub fn with_new_vol(mut self, new_vol: f32) -> Self {
        self.vol = new_vol;

        self
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SoundKind {
    Unknown,
    Movement,
    Melee,
    Projectile,
    Explosion,
    Beam,
    Shockwave,
}

#[derive(Clone, Debug)]
pub struct Target {
    pub target: EcsEntity,
    pub hostile: bool,
    pub selected_at: f64,
}

#[allow(clippy::type_complexity)]
#[derive(Clone, Debug, Default)]
pub struct Agent {
    pub rtsim_controller: RtSimController,
    pub patrol_origin: Option<Vec3<f32>>,
    pub target: Option<Target>,
    pub chaser: Chaser,
    pub behavior: Behavior,
    pub psyche: Psyche,
    pub inbox: VecDeque<AgentEvent>,
    pub action_state: ActionState,
    pub bearing: Vec2<f32>,
    pub sounds_heard: Vec<Sound>,
    pub awareness: f32,
    pub position_pid_controller: Option<PidController<fn(Vec3<f32>, Vec3<f32>) -> f32, 16>>,
}

#[derive(Clone, Debug, Default)]
pub struct ActionState {
    pub timer: f32,
    pub counter: f32,
    pub condition: bool,
    pub int_counter: u8,
}

impl Agent {
    pub fn with_patrol_origin(mut self, origin: Vec3<f32>) -> Self {
        self.patrol_origin = Some(origin);
        self
    }

    pub fn with_destination(mut self, pos: Vec3<f32>) -> Self {
        self.psyche = Psyche { aggro: 1.0 };
        self.rtsim_controller = RtSimController::with_destination(pos);
        self.behavior.allow(BehaviorCapability::SPEAK);
        self
    }

    #[allow(clippy::type_complexity)]
    pub fn with_position_pid_controller(
        mut self,
        pid: PidController<fn(Vec3<f32>, Vec3<f32>) -> f32, 16>,
    ) -> Self {
        self.position_pid_controller = Some(pid);
        self
    }

    pub fn new(
        patrol_origin: Option<Vec3<f32>>,
        body: &Body,
        behavior: Behavior,
        no_flee: bool,
    ) -> Self {
        Agent {
            patrol_origin,
            psyche: if no_flee {
                Psyche { aggro: 1.0 }
            } else {
                Psyche::from(body)
            },
            behavior,
            ..Default::default()
        }
    }
}

impl Component for Agent {
    type Storage = IdvStorage<Self>;
}

#[cfg(test)]
mod tests {
    use super::{Behavior, BehaviorCapability, BehaviorState};

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
            pv_samples: [(time, Vec3::zero()); NUM_SAMPLES],
            pv_idx: 0,
            e,
        }
    }

    /// Adds a measurement of the process variable to the ringbuffer
    pub fn add_measurement(&mut self, time: f64, pv: Vec3<f32>) {
        self.pv_idx += 1;
        self.pv_idx %= NUM_SAMPLES;
        self.pv_samples[self.pv_idx] = (time, pv);
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

    /// The integral error is the error function integrated over the last
    /// NUM_SAMPLES values. The trapezoid rule for numerical integration was
    /// chosen because it's fairly easy to calculate and sufficiently accurate.
    /// https://en.wikipedia.org/wiki/Trapezoidal_rule#Uniform_grid
    pub fn integral_err(&self) -> f32 {
        let f = |x| (self.e)(self.sp, x);
        let (a, x0) = self.pv_samples[(self.pv_idx + 1) % NUM_SAMPLES];
        let (b, xn) = self.pv_samples[self.pv_idx];
        let dx = (b - a) / NUM_SAMPLES as f64;
        let mut err = 0.0;
        // \Sigma_{k=1}^{N-1} f(x_k)
        for k in 1..=NUM_SAMPLES - 1 {
            let xk = self.pv_samples[(self.pv_idx + 1 + k) % NUM_SAMPLES].1;
            err += f(xk);
        }
        // (\Sigma_{k=1}^{N-1} f(x_k)) + \frac{f(x_N) + f(x_0)}{2}
        err += (f(xn) - f(x0)) / 2.0;
        // \Delta x * ((\Sigma_{k=1}^{N-1} f(x_k)) + \frac{f(x_N) + f(x_0)}{2})
        err *= dx as f32;
        err
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
pub fn pid_coefficients(body: &Body) -> (f32, f32, f32) {
    match body {
        Body::Ship(ship::Body::DefaultAirship) => {
            let kp = 1.0;
            let ki = 1.0;
            let kd = 1.0;
            (kp, ki, kd)
        },
        // default to a pure-proportional controller, which is the first step when tuning
        _ => (1.0, 0.0, 0.0),
    }
}
