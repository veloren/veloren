use crate::comp::{
    AbilityAction, AbilityActionKind::*, ActionState::*, EcsStateData, IdleState, ItemKind::Tool,
    StateHandler, StateUpdate, ToolData,
};
use crate::util::state_utils::*;
use std::time::Duration;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct WieldState {
    /// How long before a new action can be performed
    /// after equipping
    pub equip_delay: Duration,
}

impl StateHandler for WieldState {
    fn new(ecs_data: &EcsStateData) -> Self {
        let tool_data =
            if let Some(Tool(data)) = ecs_data.stats.equipment.main.as_ref().map(|i| &i.kind) {
                data
            } else {
                &ToolData::default()
            };
        Self {
            equip_delay: tool_data.equip_time(),
        }
    }

    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Only act once equip_delay has expired
        if self.equip_delay == Duration::default() {
            // Toggle Weapons
            if ecs_data.inputs.toggle_wield.is_just_pressed()
                && ecs_data.character.action_state.is_equip_finished()
            {
                update.character.action_state = Idle(Some(IdleState));
                return update;
            }

            // Try weapon actions
            if ecs_data.inputs.primary.is_pressed() {
                // ecs_data
                //     .updater
                //     .insert(*ecs_data.entity, AbilityAction(Primary));
            } else if ecs_data.inputs.secondary.is_pressed() {
                // ecs_data
                //     .updater
                //     .insert(*ecs_data.entity, AbilityAction(Secondary));
            }
        } else {
            // Equip delay hasn't expired yet
            // Update wield delay
            update.character.action_state = Wield(Some(WieldState {
                equip_delay: self
                    .equip_delay
                    .checked_sub(Duration::from_secs_f32(ecs_data.dt.0))
                    .unwrap_or_default(),
            }));
        }

        return update;
    }
}
