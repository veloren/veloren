use super::{ActionState::*, CharacterState, ECSStateData, ECSStateUpdate, StateHandle};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct WieldHandler {
    /// How long before a new action can be performed
    /// after equipping
    pub equip_delay: Duration,
}

impl StateHandle for WieldHandler {
    fn handle(&self, ecs_data: &ECSStateData) -> ECSStateUpdate {
        let mut update = ECSStateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Toggling Weapons
        if ecs_data.inputs.toggle_wield.is_pressed()
            && ecs_data.character.action_state.is_equip_finished()
        {
            update.character = CharacterState {
                action_state: Idle,
                move_state: ecs_data.character.move_state,
            };

            return update;
        }

        if ecs_data.inputs.primary.is_pressed() {
            // TODO: PrimaryStart
        } else if ecs_data.inputs.secondary.is_pressed() {
            // TODO: SecondaryStart
        }

        // Update wield delay
        update.character = CharacterState {
            action_state: Wield(WieldHandler {
                equip_delay: self
                    .equip_delay
                    .checked_sub(Duration::from_secs_f32(ecs_data.dt.0))
                    .unwrap_or_default(),
            }),
            move_state: ecs_data.character.move_state,
        };

        return update;
    }
}
