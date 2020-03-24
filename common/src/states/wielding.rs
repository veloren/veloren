use super::utils::*;
use crate::{
    comp::{CharacterState, StateUpdate},
    sys::character_behavior::{CharacterBehavior, JoinData},
};

pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(&data, &mut update);
        handle_jump(&data, &mut update);
        handle_climb(&data, &mut update);
        handle_glide(&data, &mut update);
        handle_ability1_input(&data, &mut update);
        handle_ability2_input(&data, &mut update);
        handle_ability3_input(&data, &mut update);
        handle_dodge_input(&data, &mut update);

        update
    }

    fn toggle_sit(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_sit(data, &mut update);
        update
    }

    fn toggle_wield(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Idle;
        update
    }

    fn swap_loadout(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_swap_loadout(data, &mut update);
        update
    }
}
