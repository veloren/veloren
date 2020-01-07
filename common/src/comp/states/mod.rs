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

use super::{
    ActionState, ActionState::*, AttackKind::*, BlockKind::*, DodgeKind::*, EcsStateData,
    MoveState, MoveState::*, StateUpdate,
};

/// ## A type for implementing State Handling Behavior.
///
/// Called by state machines' update functions to allow current states to handle updating
/// their parent machine's current state.
///
/// Structures must implement a `handle()` fn to handle update behavior, and a `new()` for
/// instantiating new instances of a state. `handle()` function recieves `EcsStateData`, a struct
/// of readonly ECS Component data, and returns a `StateUpdate` tuple, with new components that will
/// overwrite an entitie's old components.
///
/// ## Example Implementation:
/// ```
/// use crate::comp::{
///    ClimbState, EcsStateData, GlideState, JumpState, MoveState::*, SitState, StateHandler,
///    StateUpdate,
/// };
/// use crate::util::state_utils::*
/// #[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
/// pub struct RunState {
///     active_duration: Duration,
/// }
///
/// impl StateHandler for RunState {
///     fn new(ecs_data: &EcsStateData) -> Self {
///         Self {
///             active_duration: Duration::default(),
///         }
///     }
///
///     fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
///         let mut update = StateUpdate {
///             character: *ecs_data.character,
///             pos: *ecs_data.pos,
///             vel: *ecs_data.vel,
///             ori: *ecs_data.ori,
///         };
///
///         // Move player according to move_dir
///         update.vel.0 += Vec2::broadcast(ecs_data.dt.0)
///             * ecs_data.inputs.move_dir
///             * if update.vel.0.magnitude_squared() < HUMANOID_SPEED.powf(2.0) {
///                 HUMANOID_ACCEL
///             } else {
///                 0.0
///             };
///
///         // Set direction based on move direction when on the ground
///         let ori_dir = if update.character.action_state.is_attacking()
///             || update.character.action_state.is_blocking()
///         {
///             Vec2::from(ecs_data.inputs.look_dir).normalized()
///         } else {
///             Vec2::from(update.vel.0)
///         };
///
///         if ori_dir.magnitude_squared() > 0.0001
///             && (update.ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared()
///                 > 0.001
///         {
///             update.ori.0 =
///                 vek::ops::Slerp::slerp(update.ori.0, ori_dir.into(), 9.0 * ecs_data.dt.0);
///         }
///
///         // Try to sit
///         if can_sit(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
///             update.character.move_state = Sit(Some(SitState));
///             return update;
///         }
///
///         // Try to climb
///         if can_climb(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
///             update.character.move_state = Climb(Some(ClimbState));
///             return update;
///         }
///
///         // Try to jump
///         if can_jump(ecs_data.physics, ecs_data.inputs) {
///             update.character.move_state = Jump(Some(JumpState));
///             return update;
///         }
///
///         // Try to glide
///         if can_glide(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
///             update.character.move_state = Glide(Some(GlideState));
///             return update;
///         }
///
///         // Update based on groundedness
///         update.character.move_state =
///             determine_move_from_grounded_state(ecs_data.physics, ecs_data.inputs);
///
///         return update;
///     }
/// }
/// ```
pub trait StateHandler: Default {
    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate;
    fn new(ecs_data: &EcsStateData) -> Self;
}

// fn's relating to individual `ActionState`s
// or passing data from system to handlers
impl ActionState {
    /// Passes data to variant or subvariant handlers
    /// States contain `Option<StateHandler Implementor>`s, and will be
    /// `None` if state data has not been initialized. So we have to
    /// check and intialize new state data if so.
    pub fn update(&self, ecs_data: &EcsStateData) -> StateUpdate {
        match self {
            Attack(kind) => match kind {
                BasicAttack(opt_state) => opt_state
                    // If data hasn't been initialized, initialize a new one
                    .unwrap_or_else(|| BasicAttackState::new(ecs_data))
                    // Call handler
                    .handle(ecs_data),
                Charge(opt_state) => opt_state
                    .unwrap_or_else(|| ChargeAttackState::new(ecs_data))
                    .handle(ecs_data),
            },
            Block(kind) => match kind {
                BasicBlock(opt_state) => opt_state
                    .unwrap_or_else(|| BasicBlockState::new(ecs_data))
                    .handle(ecs_data),
            },
            Dodge(kind) => match kind {
                Roll(opt_state) => opt_state
                    .unwrap_or_else(|| RollState::new(ecs_data))
                    .handle(ecs_data),
            },
            Wield(opt_state) => opt_state
                .unwrap_or_else(|| WieldState::new(ecs_data))
                .handle(ecs_data),
            Idle(opt_state) => opt_state
                .unwrap_or_else(|| IdleState::new(ecs_data))
                .handle(ecs_data),
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
            // All states should be explicitly handled
            // Do not use default match: _ => {},
        }
    }
}

// fn's that relate to individual `MoveState`s
// or passing data from system to handlers
impl MoveState {
    /// Passes data to variant or subvariant handlers
    /// States contain `Option<StateHandler Implementor>`s, and will be
    /// `None` if state data has not been initialized. So we have to
    /// check and intialize new state data if so.
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
            // All states should be explicitly handled
            // Do not use default match: _ => {},
        }
    }

    /// Passes handle to variant handlers
    pub fn update(&self, ecs_data: &EcsStateData) -> StateUpdate {
        match self {
            Stand(opt_state) => opt_state
                // If data hasn't been initialized, initialize a new one
                .unwrap_or_else(|| StandState::new(ecs_data))
                // Call handler
                .handle(ecs_data),
            Run(opt_state) => opt_state
                .unwrap_or_else(|| RunState::new(ecs_data))
                .handle(ecs_data),
            Jump(opt_state) => opt_state
                .unwrap_or_else(|| JumpState::new(ecs_data))
                .handle(ecs_data),
            Climb(opt_state) => opt_state
                .unwrap_or_else(|| ClimbState::new(ecs_data))
                .handle(ecs_data),
            Glide(opt_state) => opt_state
                .unwrap_or_else(|| GlideState::new(ecs_data))
                .handle(ecs_data),
            Swim(opt_state) => opt_state
                .unwrap_or_else(|| SwimState::new(ecs_data))
                .handle(ecs_data),
            Fall(opt_state) => opt_state
                .unwrap_or_else(|| FallState::new(ecs_data))
                .handle(ecs_data),
            Sit(opt_state) => opt_state
                .unwrap_or_else(|| SitState::new(ecs_data))
                .handle(ecs_data),
            // All states should be explicitly handled
            // Do not use default match: _ => {},
        }
    }
}
