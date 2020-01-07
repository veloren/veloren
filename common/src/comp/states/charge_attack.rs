use crate::comp::{
    ActionState::Attack, AttackKind::Charge, EcsStateData, HealthChange, HealthSource,
    ItemKind::Tool, MoveState::Run, StateHandler, StateUpdate, ToolData,
};
use crate::event::ServerEvent;
use crate::util::state_utils::*;
use std::time::Duration;
use vek::Vec3;

const CHARGE_SPEED: f32 = 20.0;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct ChargeAttackState {
    /// How long the state has until exitting
    pub remaining_duration: Duration,
}

impl StateHandler for ChargeAttackState {
    fn new(ecs_data: &EcsStateData) -> Self {
        let tool_data =
            if let Some(Tool(data)) = ecs_data.stats.equipment.main.as_ref().map(|i| i.kind) {
                data
            } else {
                ToolData::default()
            };
        Self {
            remaining_duration: tool_data.attack_duration(),
        }
    }

    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            character: *ecs_data.character,
        };

        // Prevent move state handling, handled here
        update.character.move_state = Run(None);

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

            // Go back to wielding or idling
            update.character.action_state = attempt_wield(ecs_data.stats);
            return update;
        }

        // Check if charge timed out or can't keep moving forward
        if self.remaining_duration == Duration::default() || update.vel.0.magnitude_squared() < 10.0
        {
            update.character.action_state = attempt_wield(ecs_data.stats);
            return update;
        }

        // Tick remaining-duration and keep charging
        update.character.action_state = Attack(Charge(Some(ChargeAttackState {
            remaining_duration: self
                .remaining_duration
                .checked_sub(Duration::from_secs_f32(ecs_data.dt.0))
                .unwrap_or_default(),
        })));

        return update;
    }
}
