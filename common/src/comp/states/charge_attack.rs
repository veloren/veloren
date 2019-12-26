use crate::comp::{
    ActionState::{Attack, Idle, Wield},
    AttackKind::Charge,
    EcsCharacterState, EcsStateUpdate, HealthChange, HealthSource,
    ItemKind::Tool,
    MoveState::Run,
    RunHandler, StateHandle, WieldHandler,
};
use crate::event::ServerEvent;
use std::time::Duration;
use vek::Vec3;

use super::{CHARGE_SPEED, TEMP_EQUIP_DELAY};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct ChargeAttackHandler {
    /// How long the state has until exitting
    pub remaining_duration: Duration,
}

impl StateHandle for ChargeAttackHandler {
    fn handle(&self, ecs_data: &EcsCharacterState) -> EcsStateUpdate {
        let mut update = EcsStateUpdate {
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            character: *ecs_data.character,
        };

        // Prevent move state handling, handled here
        // ecs_data.updater.insert(*ecs_data.entity, OverrideMove);
        update.character.action_disabled = true;

        update.character.move_state = Run(RunHandler);

        // Move player
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
                * CHARGE_SPEED;

        // Check if hitting another entity
        if let Some(uid_b) = ecs_data.physics.touch_entity {
            // Send Damage event
            ecs_data.server_bus.emitter().emit(ServerEvent::Damage {
                uid: uid_b,
                change: HealthChange {
                    amount: -20,
                    cause: HealthSource::Attack { by: *ecs_data.uid },
                },
            });

            // Go back to wielding
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

        // Check if charge timed out or can't keep moving forward
        if self.remaining_duration == Duration::default() || update.vel.0.magnitude_squared() < 10.0
        {
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

        // Tick remaining-duration and keep charging
        update.character.action_state = Attack(Charge(ChargeAttackHandler {
            remaining_duration: self
                .remaining_duration
                .checked_sub(Duration::from_secs_f32(ecs_data.dt.0))
                .unwrap_or_default(),
        }));

        return update;
    }
}
