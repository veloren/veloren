use super::{BLOCK_ACCEL, BLOCK_SPEED};
use crate::comp::{EcsStateData, StateHandle, StateUpdate};
use crate::util::movement_utils::*;
use std::time::Duration;
use vek::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct BasicBlockState {
    /// How long the blocking state has been active
    pub active_duration: Duration,
}

impl StateHandle for BasicBlockState {
    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            character: *ecs_data.character,
        };

        // TODO: Apply simple move speed debuff instead
        update.character.move_disabled_this_tick = true;

        // Update movement
        update.vel.0 += Vec2::broadcast(ecs_data.dt.0)
            * ecs_data.inputs.move_dir
            * match ecs_data.physics.on_ground {
                true if update.vel.0.magnitude_squared() < BLOCK_SPEED.powf(2.0) => BLOCK_ACCEL,
                _ => 0.0,
            };

        if !ecs_data.inputs.secondary.is_pressed() {
            update.character.action_state = attempt_wield(ecs_data.stats);
            update.character.move_disabled_this_tick = false;
            return update;
        }

        return update;
    }
}
