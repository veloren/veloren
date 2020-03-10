use crate::{
    comp::{Energy, Ori, Pos, ToolData, Vel},
    event::{LocalEvent, ServerEvent},
};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage, HashMapStorage, VecStorage};
use std::{collections::VecDeque, time::Duration};

/// Data returned from character behavior fn's to Character Behavior System.
pub struct StateUpdate {
    pub character: CharacterState,
    pub pos: Pos,
    pub vel: Vel,
    pub ori: Ori,
    pub energy: Energy,
    pub local_events: VecDeque<LocalEvent>,
    pub server_events: VecDeque<ServerEvent>,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum CharacterState {
    Idle {},
    Climb {},
    Sit {},
    Equipping {
        /// The weapon being equipped
        tool: ToolData,
        /// Time left before next state
        time_left: Duration,
    },
    Wielding {
        /// The weapon being wielded
        tool: ToolData,
    },
    Glide {},
    /// A basic attacking state
    BasicAttack {
        /// How long the state has until exiting
        remaining_duration: Duration,
        /// Whether the attack can deal more damage
        exhausted: bool,
    },
    /// A basic blocking state
    BasicBlock {},
    ChargeAttack {
        /// How long the state has until exiting
        remaining_duration: Duration,
    },
    Roll {
        /// How long the state has until exiting
        remaining_duration: Duration,
    },
    /// A three-stage attack where play must click at appropriate times
    /// to continue attack chain.
    TripleAttack {
        /// The tool this state will read to handle damage, etc.
        tool: ToolData,
        /// `int` denoting what stage (of 3) the attack is in.
        stage: i8,
        /// How long current stage has been active
        stage_time_active: Duration,
        /// Whether current stage has exhausted its attack
        stage_exhausted: bool,
        /// Whether player has clicked at the proper time to go to next stage
        can_transition: bool,
    },
}

impl CharacterState {
    pub fn is_wield(&self) -> bool {
        match self {
            CharacterState::Wielding { .. } => true,
            CharacterState::BasicAttack { .. } => true,
            CharacterState::BasicBlock { .. } => true,
            _ => false,
        }
    }

    pub fn is_attack(&self) -> bool {
        match self {
            CharacterState::BasicAttack { .. } | CharacterState::ChargeAttack { .. } => true,
            _ => false,
        }
    }

    pub fn is_block(&self) -> bool {
        match self {
            CharacterState::BasicBlock { .. } => true,
            _ => false,
        }
    }

    pub fn is_dodge(&self) -> bool {
        match self {
            CharacterState::Roll { .. } => true,
            _ => false,
        }
    }

    /// Compares for shallow equality (does not check internal struct equality)
    pub fn equals(&self, other: &Self) -> bool {
        // Check if state is the same without looking at the inner data
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Default for CharacterState {
    fn default() -> Self { Self::Idle {} }
}

impl Component for CharacterState {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Attacking {
    pub weapon: Option<ToolData>,
    pub applied: bool,
    pub hit_count: u32,
}

impl Component for Attacking {
    type Storage = VecStorage<Self>;
}
