use super::utils::*;
use crate::{
    comp::{character_state::OutputEvents, CharacterState, InventoryAction, StateUpdate},
    states::{
        behavior::{CharacterBehavior, JoinData},
        idle,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        leave_stance(data, output_events);
        handle_wield(data, &mut update);
        handle_jump(data, output_events, &mut update, 1.0);

        // Try to Fall/Stand up/Move
        if data.physics.on_ground.is_none() || data.inputs.move_dir.magnitude_squared() > 0.0 {
            update.character = CharacterState::Idle(idle::Data::default());
        }

        update
    }

    fn manipulate_loadout(
        &self,
        data: &JoinData,
        output_events: &mut OutputEvents,
        inv_action: InventoryAction,
    ) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        handle_manipulate_loadout(data, output_events, &mut update, inv_action);
        update
    }

    fn wield(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_wield(data, &mut update);
        update
    }

    fn dance(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_dance(data, &mut update);
        update
    }

    fn stand(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        // Try to Fall/Stand up/Move
        update.character = CharacterState::Idle(idle::Data::default());
        update
    }
}
