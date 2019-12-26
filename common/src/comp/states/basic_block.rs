use super::{BLOCK_ACCEL, BLOCK_SPEED, TEMP_EQUIP_DELAY};
use crate::comp::{
    ActionState::{Idle, Wield},
    EcsCharacterState, EcsStateUpdate,
    ItemKind::Tool,
    StateHandle, WieldHandler,
};
use std::time::Duration;
use vek::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct BasicBlockHandler {
    /// How long the blocking state has been active
    active_duration: Duration,
}

impl StateHandle for BasicBlockHandler {
    fn handle(&self, ecs_data: &EcsCharacterState) -> EcsStateUpdate {
        let mut update = EcsStateUpdate {
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
            update.character.action_state = if let Some(Tool { .. }) =
                ecs_data.stats.equipment.main.as_ref().map(|i| &i.kind)
            {
                Wield(WieldHandler {
                    equip_delay: Duration::from_millis(TEMP_EQUIP_DELAY),
                })
            } else {
                Idle
            };

            update.character.move_disabled = false;
            return update;
        }

        return update;
    }
}
