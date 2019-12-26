use super::{ROLL_SPEED, TEMP_EQUIP_DELAY};
use crate::comp::{
    ActionState::*, DodgeKind::*, EcsCharacterState, EcsStateUpdate, ItemKind::Tool, OverrideMove,
    StateHandle, WieldHandler,
};
use std::time::Duration;
use vek::Vec3;
#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct RollHandler {
    /// How long the state has until exitting
    remaining_duration: Duration,
}

impl StateHandle for RollHandler {
    fn handle(&self, ecs_data: &EcsCharacterState) -> EcsStateUpdate {
        let mut update = EcsStateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Prevent move state handling, handled here
        ecs_data.updater.insert(*ecs_data.entity, OverrideMove);

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
            update.character.action_state = if let Some(Tool { .. }) =
                ecs_data.stats.equipment.main.as_ref().map(|i| &i.kind)
            {
                Wield(WieldHandler {
                    equip_delay: Duration::from_millis(TEMP_EQUIP_DELAY),
                })
            } else {
                Idle
            };

            ecs_data.updater.remove::<OverrideMove>(*ecs_data.entity);
            return update;
        }

        // Otherwise, tick down remaining_duration, and keep rolling
        update.character.action_state = Dodge(Roll(RollHandler {
            remaining_duration: self
                .remaining_duration
                .checked_sub(Duration::from_secs_f32(ecs_data.dt.0))
                .unwrap_or_default(),
        }));

        return update;
    }
}
