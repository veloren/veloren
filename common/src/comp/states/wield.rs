use super::{ActionState::*, EcsCharacterState, EcsStateUpdate, StateHandle};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct WieldHandler {
    /// How long before a new action can be performed
    /// after equipping
    pub equip_delay: Duration,
}

impl StateHandle for WieldHandler {
    fn handle(&self, ecs_data: &EcsCharacterState) -> EcsStateUpdate {
        let mut update = EcsStateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Only act once equip_delay has expired
        if self.equip_delay == Duration::default() {
            // Toggle Weapons
            if ecs_data.inputs.toggle_wield.is_pressed()
                && ecs_data.character.action_state.is_equip_finished()
            {
                update.character.action_state = Idle;

                return update;
            }

            // Try weapon actions
            if ecs_data.inputs.primary.is_pressed() {
                // TODO: PrimaryStart
            } else if ecs_data.inputs.secondary.is_pressed() {
                // TODO: SecondaryStart
            }
        }
        // Equip delay hasn't expired yet
        else {
            // Update wield delay
            update.character.action_state = Wield(WieldHandler {
                equip_delay: self
                    .equip_delay
                    .checked_sub(Duration::from_secs_f32(ecs_data.dt.0))
                    .unwrap_or_default(),
            });
        }

        return update;
    }
}
