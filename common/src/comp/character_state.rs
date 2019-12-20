use crate::comp::{Body, CharacterState, Controller, ControllerInputs, PhysicsState};
use specs::{Component, FlaggedStorage, HashMapStorage};
use specs::{Entities, Join, LazyUpdate, Read, ReadStorage, System};
use sphynx::{Uid, UidAllocator};
//use specs_idvs::IDVStorage;
use self::{ActionState::*, MovementState::*};
use std::time::Duration;
pub trait State {
    fn handle(
        &self,
        character: &CharacterState,
        inputs: &ControllerInputs,
        body: &Body,
        physics: &PhysicsState,
    ) -> CharacterState;
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct RunData;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct StandData;

impl State for StandData {
    fn handle(
        &self,
        character: &CharacterState,
        inputs: &ControllerInputs,
        body: &Body,
        physics: &PhysicsState,
    ) -> CharacterState {
        let mut new_move: MovementState = if inputs.move_dir.magnitude_squared() > 0.0 {
            MovementState::Run(RunData)
        } else {
            MovementState::Stand(StandData)
        };

        // Try to sit
        if inputs.sit.is_pressed() && physics.on_ground && body.is_humanoid() {
            return CharacterState {
                movement: Sit,
                action: Idle,
            };
        }

        // Try to climb
        if let (true, Some(_wall_dir)) = (
            inputs.climb.is_pressed() | inputs.climb_down.is_pressed() && body.is_humanoid(),
            physics.on_wall,
        ) {
            return CharacterState {
                movement: Climb,
                action: Idle,
            };
        }

        // Try to swim
        if !physics.on_ground && physics.in_fluid {
            return CharacterState {
                action: character.action,
                movement: Swim,
            };
        }

        // While on ground ...
        if physics.on_ground {
            // Try to jump
            if inputs.jump.is_pressed() && !inputs.jump.is_held_down() {
                return CharacterState {
                    action: character.action,
                    movement: Jump,
                };
            }

            // Try to charge
            if inputs.charge.is_pressed() && !inputs.charge.is_held_down() {
                return CharacterState {
                    action: Charge {
                        time_left: Duration::from_millis(250),
                    },
                    movement: Run(RunData),
                };
            }

            // Try to roll
            if inputs.roll.is_pressed() && body.is_humanoid() {
                return CharacterState {
                    action: Roll {
                        time_left: Duration::from_millis(600),
                        was_wielding: character.action.is_wield(),
                    },
                    movement: Run(RunData),
                };
            }
        }
        // While not on ground ...
        else {
            // Try to glide
            if physics.on_wall == None
                && inputs.glide.is_pressed()
                && !inputs.glide.is_held_down()
                && body.is_humanoid()
            {
                character.movement = Glide;
                continue;
            }
            character.movement = Fall;
        }

        // Tool Actions
        if inputs.toggle_wield.is_just_pressed() {
            match action_state {
                Wield { .. } | Attack { .. } => {
                    // Prevent instantaneous reequipping by checking
                    // for done wielding
                    if character.action.is_action_finished() {
                        character.action = Idle;
                    }
                    continue;
                }
                Idle => {
                    character.action = try_wield(stats);
                    continue;
                }
                Charge { .. } | Roll { .. } | Block { .. } => {}
            }
        }
        if inputs.primary.is_pressed() {
            // TODO: PrimaryStart
        } else if inputs.secondary.is_pressed() {
            // TODO: SecondaryStart
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum MovementState {
    Stand(Stand),
    Sit,
    Run(Run),
    Jump,
    Fall,
    Glide,
    Swim,
    Climb,
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
            movement: MovementState::Jump,
            action: ActionState::Idle,
        }
    }
}

impl Component for CharacterState {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}
