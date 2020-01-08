use crate::comp::{ActionState::Wield, EcsStateData, ItemKind::Tool, StateHandler, StateUpdate};

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct State;

impl StateHandler for State {
    fn new(_ecs_data: &EcsStateData) -> Self {
        Self {}
    }

    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Try to wield
        if ecs_data.inputs.primary.is_pressed()
            || ecs_data.inputs.secondary.is_pressed()
            || (ecs_data.inputs.toggle_wield.is_just_pressed()
                && update.character.action_state.is_equip_finished())
        {
            if let Some(Tool(_)) = ecs_data.stats.equipment.main.as_ref().map(|i| &i.kind) {
                update.character.action_state = Wield(None);
            }

            // else unarmed stuff?
        }

        return update;
    }
}
