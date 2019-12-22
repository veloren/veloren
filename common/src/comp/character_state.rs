use crate::comp::{Body, Controller, ControllerInputs, ItemKind, PhysicsState, Stats};
use specs::{Component, FlaggedStorage, HashMapStorage};
use specs::{Entities, Join, LazyUpdate, Read, ReadStorage, System};
use sphynx::{Uid, UidAllocator};
//use specs_idvs::IDVStorage;
use self::{ActionState::*, MovementState::*};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct RunData;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct StandData;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct SitData;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct JumpData;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct FallData;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct GlideData;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct SwimData;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct ClimbData;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum MovementState {
    Stand(StandData),
    Run(RunData),
    Sit(SitData),
    Jump(JumpData),
    Fall(FallData),
    Glide(GlideData),
    Swim(SwimData),
    Climb(ClimbData),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum ActionState {
    Idle,
    Wield {
        time_left: Duration,
    },
    Attack {
        time_left: Duration,
        applied: bool,
    },
    Block {
        time_active: Duration,
    },
    Roll {
        time_left: Duration,
        // Whether character was wielding before they started roll
        was_wielding: bool,
    },
    Charge {
        time_left: Duration,
    },
    // Handle(CharacterAction),
}

impl ActionState {
    pub fn is_wield(&self) -> bool {
        if let Self::Wield { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_action_finished(&self) -> bool {
        match self {
            Self::Wield { time_left }
            | Self::Attack { time_left, .. }
            | Self::Roll { time_left, .. }
            | Self::Charge { time_left } => *time_left == Duration::default(),
            Self::Idle | Self::Block { .. } => false,
        }
    }

    pub fn is_attack(&self) -> bool {
        if let Self::Attack { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_block(&self) -> bool {
        if let Self::Block { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_roll(&self) -> bool {
        if let Self::Roll { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_charge(&self) -> bool {
        if let Self::Charge { .. } = self {
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct CharacterState {
    pub movement: MovementState,
    pub action: ActionState,
}

impl CharacterState {
    pub fn is_same_movement(&self, other: &Self) -> bool {
        // Check if enum item is the same without looking at the inner data
        std::mem::discriminant(&self.movement) == std::mem::discriminant(&other.movement)
    }
    pub fn is_same_action(&self, other: &Self) -> bool {
        // Check if enum item is the same without looking at the inner data
        std::mem::discriminant(&self.action) == std::mem::discriminant(&other.action)
    }
    pub fn is_same_state(&self, other: &Self) -> bool {
        self.is_same_movement(other) && self.is_same_action(other)
    }
}

impl Default for CharacterState {
    fn default() -> Self {
        Self {
            movement: MovementState::Fall(FallData),
            action: ActionState::Idle,
        }
    }
}

impl Component for CharacterState {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}
