use super::ROLL_SPEED;
use crate::comp::{ActionState::*, DodgeKind::*, EcsStateData, StateHandle, StateUpdate};
use crate::util::movement_utils::*;
use std::time::Duration;
use vek::Vec3;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct RollState {
    /// How long the state has until exitting
    remaining_duration: Duration,
}

impl StateHandle for RollState {
    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Prevent move state handling, handled here
        update.character.move_disabled_this_tick= true;

        // Update velocity
        update.vel.0 = Vec3::new(0.0, 0.0, update.vel.0.z)
            + (update.vel.0 * Vec3::new(1.0, 1.0, 0.0)
                + 1.5
                    * ecs_data
                        .inputs
                        .move_dir
                        .try_normalized()
                        .unwrap_or_default())
            .try_normalized()
            .unwrap_or_default()
                * ROLL_SPEED;

        // Check if roll duration has expired
        if self.remaining_duration == Duration::default() {
            // If so, go back to wielding or idling
            update.character.action_state = attempt_wield(ecs_data.stats);
            update.character.move_disabled_this_tick= false;
            return update;
        }

        // Otherwise, tick down remaining_duration
        update.character.action_state = Dodge(Roll(RollState {
            remaining_duration: self
                .remaining_duration
                .checked_sub(Duration::from_secs_f32(ecs_data.dt.0))
                .unwrap_or_default(),
        }));

        // Keep rolling
        return update;
    }
}
