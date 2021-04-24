use super::utils::*;
use crate::{
    comp::{CharacterState, InventoryAction, StateUpdate},
    states::behavior::{CharacterBehavior, JoinData},
};

pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0);
        handle_move(data, &mut update, 0.4);
        handle_jump(data, &mut update, 1.0);
        handle_wield(data, &mut update);
        handle_climb(data, &mut update);
        handle_dodge_input(data, &mut update);

        // Try to Fall/Stand up/Move
        if !data.physics.on_ground {
            update.character = CharacterState::Idle;
        }

        update
    }

    fn wield(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_wield(data, &mut update);
        update
    }

    fn sit(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_sit(data, &mut update);
        update
    }

    fn dance(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_dance(data, &mut update);
        update
    }

    fn glide_wield(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_glide_wield(data, &mut update);
        update
    }

    fn swap_equipped_weapons(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_swap_equipped_weapons(data, &mut update);
        update
    }

    fn stand(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Idle;
        update
    }

    fn manipulate_loadout(&self, data: &JoinData, inv_action: InventoryAction) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        handle_manipulate_loadout(&data, &mut update, inv_action);
        update
    }
}
