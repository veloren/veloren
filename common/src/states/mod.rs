// Module declarations
pub mod basic_attack;
pub mod basic_block;
pub mod charge_attack;
pub mod climb;
pub mod fall;
pub mod glide;
pub mod idle;
pub mod jump;
pub mod roll;
pub mod run;
pub mod sit;
pub mod stand;
pub mod swim;
pub mod utils;
pub mod wield;

use crate::comp::{
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
/// use crate::states::utils;
///
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
///         // -- snip --
///         // Updates; checks for gliding, climbing, etc.
///
///         // Update based on groundedness
///         update.character.move_state =
///             utils::determine_move_from_grounded_state(ecs_data.physics, ecs_data.inputs);
///
///         update
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
                    .unwrap_or_else(|| basic_attack::State::new(ecs_data))
                    // Call handler
                    .handle(ecs_data),
                Charge(opt_state) => opt_state
                    .unwrap_or_else(|| charge_attack::State::new(ecs_data))
                    .handle(ecs_data),
            },
            Block(kind) => match kind {
                BasicBlock(opt_state) => opt_state
                    .unwrap_or_else(|| basic_block::State::new(ecs_data))
                    .handle(ecs_data),
            },
            Dodge(kind) => match kind {
                Roll(opt_state) => opt_state
                    .unwrap_or_else(|| roll::State::new(ecs_data))
                    .handle(ecs_data),
            },
            Wield(opt_state) => opt_state
                .unwrap_or_else(|| wield::State::new(ecs_data))
                .handle(ecs_data),
            Idle(opt_state) => opt_state
                .unwrap_or_else(|| idle::State::new(ecs_data))
                .handle(ecs_data),
            // All states should be explicitly handled
            // Do not use default match: _ => {},
        }
    }

    // TODO: remove when we split up character states into SingleAction and MultiAction enum variants
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
    // TODO: remove when we split up character states into SingleAction and MultiAction enum variants
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
                .unwrap_or_else(|| stand::State::new(ecs_data))
                // Call handler
                .handle(ecs_data),
            Run(opt_state) => opt_state
                .unwrap_or_else(|| run::State::new(ecs_data))
                .handle(ecs_data),
            Jump(opt_state) => opt_state
                .unwrap_or_else(|| jump::State::new(ecs_data))
                .handle(ecs_data),
            Climb(opt_state) => opt_state
                .unwrap_or_else(|| climb::State::new(ecs_data))
                .handle(ecs_data),
            Glide(opt_state) => opt_state
                .unwrap_or_else(|| glide::State::new(ecs_data))
                .handle(ecs_data),
            Swim(opt_state) => opt_state
                .unwrap_or_else(|| swim::State::new(ecs_data))
                .handle(ecs_data),
            Fall(opt_state) => opt_state
                .unwrap_or_else(|| fall::State::new(ecs_data))
                .handle(ecs_data),
            Sit(opt_state) => opt_state
                .unwrap_or_else(|| sit::State::new(ecs_data))
                .handle(ecs_data),
            // All states should be explicitly handled
            // Do not use default match: _ => {},
        }
    }
}
