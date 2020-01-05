// Module declarations
mod basic_attack;
mod basic_block;
mod charge_attack;
mod climb;
mod fall;
mod glide;
mod idle;
mod jump;
mod roll;
mod run;
mod sit;
mod stand;
mod swim;
mod wield;

// Reexports
pub use basic_attack::*;
pub use basic_block::*;
pub use charge_attack::*;
pub use climb::*;
pub use fall::*;
pub use glide::*;
pub use idle::*;
pub use jump::*;
pub use roll::*;
pub use run::*;
pub use sit::*;
pub use stand::*;
pub use swim::*;
pub use wield::*;

// TODO: Attach these to racial components and/or ecs resources
pub const HUMANOID_ACCEL: f32 = 50.0;
pub const HUMANOID_SPEED: f32 = 120.0;
pub const HUMANOID_AIR_ACCEL: f32 = 10.0;
pub const HUMANOID_AIR_SPEED: f32 = 100.0;
pub const HUMANOID_WATER_ACCEL: f32 = 70.0;
pub const HUMANOID_WATER_SPEED: f32 = 120.0;
pub const HUMANOID_CLIMB_ACCEL: f32 = 5.0;
pub const ROLL_SPEED: f32 = 17.0;
pub const CHARGE_SPEED: f32 = 20.0;
pub const GLIDE_ACCEL: f32 = 15.0;
pub const GLIDE_SPEED: f32 = 45.0;
pub const BLOCK_ACCEL: f32 = 30.0;
pub const BLOCK_SPEED: f32 = 75.0;
pub const TEMP_EQUIP_DELAY: u64 = 100;
// Gravity is 9.81 * 4, so this makes gravity equal to .15
pub const GLIDE_ANTIGRAV: f32 = crate::sys::phys::GRAVITY * 0.96;
pub const CLIMB_SPEED: f32 = 5.0;
pub const MOVEMENT_THRESHOLD_VEL: f32 = 3.0;

use super::{
    ActionState, ActionState::*, AttackKind::*, BlockKind::*, DodgeKind::*, EcsStateData,
    MoveState, MoveState::*, StateUpdate,
};
use std::time::Duration;

/// #### A trait for implementing state `handle()`ing logic.
///  _Mimics the typical OOP style state machine pattern where states implement their own behavior,
///  exit conditions, and return new states to the state machine upon exit.
///  This is still performant and consistent with ECS data-behavior-separation constraint
///  since trait fn's are syntactic sugar for static fn's that accept their implementor's
///  object type as its first parameter. This allows for several benefits over implementing
///  each state's behavior within the `CharacterState` update `System` itself:_
///  
///  1. Less cognitive overhead: State's handling logic is next to the its data, and component (inside the state's .rs file).
///  2. Separation of concerns (between states): all logic within a state's `handle()` is relevant only to that state.
///     States can be added/editted without concerns of affecting other state's logic.
///  3. Clearly defined API and pattern: All states accept the same `EcsStateData` struct, which can be added to as necessary,
///     without the need for updating every state's implementation. All states return the same `StateUpdate` component.
///     `CharacterState` update `System` passes `EcsStateData` to `ActionState`/`MoveState` `handle()` which matches the character's
///     current state to its `handle()` fn, hiding the implementation details, since the System is only concerned with
///     how the update flow occurs and is in charge of updating the ECS components.
pub trait StateHandler: Default {
    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate;
}

// Public interface that passes EcsStateData to `StateHandle`s `handle()` fn
impl ActionState {
    /// Passes handle to variant or subvariant handlers
    pub fn update(&self, ecs_data: &EcsStateData) -> StateUpdate {
        match self {
            Attack(kind) => match kind {
                BasicAttack(Some(state)) => state.handle(&ecs_data),
                Charge(Some(state)) => state.handle(&ecs_data),
            },
            Block(kind) => match kind {
                BasicBlock(Some(state)) => state.handle(&ecs_data),
            },
            Dodge(kind) => match kind {
                Roll(Some(state)) => state.handle(&ecs_data),
            },
            Wield(Some(state)) => state.handle(&ecs_data),
            Idle(Some(state)) => state.handle(&ecs_data),
            //
            // All states should be explicitly handled
            // Do not use default match: _ => {},
        }
    }

    /// Returns whether a given `ActionState` overrides `MoveState` `handle()`ing
    pub fn overrides_move_state(&self) -> bool {
        match self {
            Attack(kind) => match kind {
                BasicAttack(_) => false,
                Charge(_) => true,
            },
            Block(kind) => match kind {
                BasicBlock(_) => true,
            },
            Dodge(kind) => match kind {
                Roll(_) => true,
            },
            Wield(_) => false,
            Idle(_) => false,
            //
            // All states should be explicitly handled
            // Do not use default match: _ => {},
        }
    }
}

// Other fn's that relate to individual `MoveState`s
impl MoveState {
    /// Returns whether a given `ActionState` overrides `MoveState` `handle()`ing
    pub fn overrides_action_state(&self) -> bool {
        match self {
            Stand(_) => false,
            Run(_) => false,
            Jump(_) => false,
            Climb(_) => true,
            Glide(_) => true,
            Swim(_) => false,
            Fall(_) => false,
            Sit(_) => true,
            //
            // All states should be explicitly handled
            // Do not use default match: _ => {},
        }
    }

    /// Passes handle to variant handlers
    pub fn update(&self, ecs_data: &EcsStateData) -> StateUpdate {
        match self {
            Stand(Some(state)) => state.handle(&ecs_data),
            Run(Some(state)) => state.handle(&ecs_data),
            Jump(Some(state)) => state.handle(&ecs_data),
            Climb(Some(state)) => state.handle(&ecs_data),
            Glide(Some(state)) => state.handle(&ecs_data),
            Swim(Some(state)) => state.handle(&ecs_data),
            Fall(Some(state)) => state.handle(&ecs_data),
            Sit(Some(state)) => state.handle(&ecs_data),
            // All states should be explicitly handled
            // Do not use default match: _ => {},
        }
    }
}
