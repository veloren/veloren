use specs::{Component, FlaggedStorage, HashMapStorage};
use specs_idvs::IDVStorage;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MovementState {
    Stand,
    Run,
    Jump,
    Glide,
    Roll { time_left: Duration },
    //Swim,
}

impl MovementState {
    pub fn is_roll(&self) -> bool {
        if let Self::Roll { .. } = self {
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ActionState {
    Idle,
    Wield { time_left: Duration },
    Attack { time_left: Duration, applied: bool },
    Block { time_left: Duration },
    //Carry,
}

impl ActionState {
    pub fn is_wield(&self) -> bool {
        if let Self::Wield { .. } = self {
            true
        } else {
            false
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
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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
            movement: MovementState::Jump,
            action: ActionState::Idle,
        }
    }
}

impl Component for CharacterState {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}
