use super::{
    CharacterState, ECSStateData, ECSStateUpdate, MoveState::Run, RunHandler, Stand, StandHandler,
    StateHandle, WieldHandler,
};
use crate::comp::{
    ActionState::{Attack, Idle, Wield},
    AttackKind::Charge,
    HealthChange, HealthSource,
    ItemKind::Tool,
};
use crate::event::ServerEvent;
use std::time::Duration;

use super::TEMP_EQUIP_DELAY;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct ChargeAttackHandler {
    /// How long the state has until exitting
    pub remaining_duration: Duration,
}

impl StateHandle for ChargeAttackHandler {
    fn handle(&self, ecs_data: &ECSStateData) -> ECSStateUpdate {
        let mut update = ECSStateUpdate {
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            character: *ecs_data.character,
        };

        if let Some(uid_b) = ecs_data.physics.touch_entity {
            ecs_data.server_bus.emitter().emit(ServerEvent::Damage {
                uid: uid_b,
                change: HealthChange {
                    amount: -20,
                    cause: HealthSource::Attack { by: *ecs_data.uid },
                },
            });

            update.character = CharacterState {
                move_state: Stand(StandHandler),
                action_state: if let Some(Tool { .. }) =
                    ecs_data.stats.equipment.main.as_ref().map(|i| &i.kind)
                {
                    Wield(WieldHandler {
                        equip_delay: Duration::from_millis(TEMP_EQUIP_DELAY),
                    })
                } else {
                    Idle
                },
            };

            return update;
        }

        if self.remaining_duration == Duration::default() || update.vel.0.magnitude_squared() < 10.0
        {
            update.character = CharacterState {
                move_state: Stand(StandHandler),
                action_state: if let Some(Tool { .. }) =
                    ecs_data.stats.equipment.main.as_ref().map(|i| &i.kind)
                {
                    Wield(WieldHandler {
                        equip_delay: Duration::from_millis(TEMP_EQUIP_DELAY),
                    })
                } else {
                    Idle
                },
            };

            return update;
        }

        update.character = CharacterState {
            move_state: Run(RunHandler),
            action_state: Attack(Charge(ChargeAttackHandler {
                remaining_duration: self
                    .remaining_duration
                    .checked_sub(Duration::from_secs_f32(ecs_data.dt.0))
                    .unwrap_or_default(),
            })),
        };

        return update;
    }
}
