// Module declarations
pub mod basic_attack;
pub mod climb;
pub mod glide;
pub mod idle;
pub mod roll;
pub mod sit;
pub mod utils;
pub mod wielded;
pub mod wielding;

use crate::comp::{EcsStateData, StateUpdate};

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
