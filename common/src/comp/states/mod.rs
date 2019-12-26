// Module declarations
mod basic_attack;
mod basic_block;
mod charge_attack;
mod climb;
mod fall;
mod glide;
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

// Public interface, wires character states to their handlers.
use super::{
    ActionState, ActionState::*, AttackKind::*, BlockKind::*, CharacterState, DodgeKind::*,
    ECSStateData, ECSStateUpdate, MoveState, MoveState::*,
};

pub trait StateHandle {
    fn handle(&self, ecs_data: &ECSStateData) -> ECSStateUpdate;
}

impl StateHandle for ActionState {
    /// Passes handle to variant or subvariant handlers
    fn handle(&self, ecs_data: &ECSStateData) -> ECSStateUpdate {
        match self {
            Attack(kind) => match kind {
                BasicAttack(handler) => handler.handle(ecs_data),
                Charge(handler) => handler.handle(ecs_data),
            },
            Block(kind) => match kind {
                BasicBlock(handler) => handler.handle(ecs_data),
            },
            Dodge(kind) => match kind {
                Roll(handler) => handler.handle(ecs_data),
            },
            Wield(handler) => handler.handle(ecs_data),
            Idle => ECSStateUpdate {
                character: *ecs_data.character,
                pos: *ecs_data.pos,
                vel: *ecs_data.vel,
                ori: *ecs_data.ori,
            },
            // All states should be explicitly handled
            // Do not use default match: _ => {},
        }
    }
}

impl StateHandle for MoveState {
    /// Passes handle to variant handlers
    fn handle(&self, ecs_data: &ECSStateData) -> ECSStateUpdate {
        match self {
            Stand(handler) => handler.handle(&ecs_data),
            Run(handler) => handler.handle(&ecs_data),
            Jump(handler) => handler.handle(&ecs_data),
            Climb(handler) => handler.handle(&ecs_data),
            Glide(handler) => handler.handle(&ecs_data),
            Swim(handler) => handler.handle(&ecs_data),
            Fall(handler) => handler.handle(&ecs_data),
            Sit(handler) => handler.handle(&ecs_data),
            // All states should be explicitly handled
            // Do not use default match: _ => {},
        }
    }
}
