// We'd like to not have this file in `common`, but sadly there are
// things in `common` that require it (currently, `ServerEvent` and
// `Agent`). When possible, this should be moved to the `rtsim`
// module in `server`.

use specs::Component;
use vek::*;

use crate::comp::dialogue::MoodState;

pub type RtSimId = usize;

#[derive(Copy, Clone, Debug)]
pub struct RtSimEntity(pub RtSimId);

impl Component for RtSimEntity {
    type Storage = specs::VecStorage<Self>;
}

#[derive(Clone, Debug)]
pub enum RtSimEvent {
    AddMemory(Memory),
    SetMood(Memory),
    ForgetEnemy(String),
    PrintMemories,
}

#[derive(Clone, Debug)]
pub struct Memory {
    pub item: MemoryItem,
    pub time_to_forget: f64,
}

#[derive(Clone, Debug)]
pub enum MemoryItem {
    // These are structs to allow more data beyond name to be stored
    // such as clothing worn, weapon used, etc.
    CharacterInteraction { name: String },
    CharacterFight { name: String },
    Mood { state: MoodState },
}

/// This type is the map route through which the rtsim (real-time simulation)
/// aspect of the game communicates with the rest of the game. It is analagous
/// to `comp::Controller` in that it provides a consistent interface for
/// simulation NPCs to control their actions. Unlike `comp::Controller`, it is
/// very abstract and is intended for consumption by both the agent code and the
/// internal rtsim simulation code (depending on whether the entity is loaded
/// into the game as a physical entity or not). Agent code should attempt to act
/// upon its instructions where reasonable although deviations for various
/// reasons (obstacle avoidance, counter-attacking, etc.) are expected.
#[derive(Clone, Debug)]
pub struct RtSimController {
    /// When this field is `Some(..)`, the agent should attempt to make progress
    /// toward the given location, accounting for obstacles and other
    /// high-priority situations like being attacked.
    pub travel_to: Option<(Vec3<f32>, String)>,
    /// Proportion of full speed to move
    pub speed_factor: f32,
    /// Events
    pub events: Vec<RtSimEvent>,
}

impl Default for RtSimController {
    fn default() -> Self {
        Self {
            travel_to: None,
            speed_factor: 1.0,
            events: Vec::new(),
        }
    }
}

impl RtSimController {
    pub fn reset(&mut self) { *self = Self::default(); }

    pub fn with_destination(pos: Vec3<f32>) -> Self {
        Self {
            travel_to: Some((pos, format!("{:0.1?}", pos))),
            speed_factor: 0.25,
            events: Vec::new(),
        }
    }
}
