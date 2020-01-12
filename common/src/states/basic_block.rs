use super::utils::*;
use crate::comp::{EcsStateData, StateUpdate};
use crate::states::StateHandler;
use std::time::Duration;
use vek::Vec2;

const BLOCK_ACCEL: f32 = 30.0;
const BLOCK_SPEED: f32 = 75.0;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct State {
    /// How long the blocking state has been active
    pub active_duration: Duration,
}

impl StateHandler for State {
    fn new(_ecs_data: &EcsStateData) -> Self {
        Self {
            active_duration: Duration::default(),
        }
    }

    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            character: *ecs_data.character,
        };

        // TODO: Apply simple move speed debuff instead

        // Update movement
        update.vel.0 += Vec2::broadcast(ecs_data.dt.0)
            * ecs_data.inputs.move_dir
            * match ecs_data.physics.on_ground {
                true if update.vel.0.magnitude_squared() < BLOCK_SPEED.powf(2.0) => BLOCK_ACCEL,
                _ => 0.0,
            };

        if !ecs_data.inputs.secondary.is_pressed() {
            update.character.action_state = attempt_wield(ecs_data.stats);
            return update;
        }

        update
    }
}
