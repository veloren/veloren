use super::utils::*;
use crate::{
    comp::{CharacterState, LoadoutManip, StateUpdate},
    states::behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};

const TURN_RATE: f32 = 40.0;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_wield(data, &mut update);
        handle_orientation(data, &mut update, TURN_RATE);

        update
    }

    fn wield(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_wield(data, &mut update);
        update
    }

    fn sit(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Idle;
        attempt_sit(data, &mut update);
        update
    }

    fn dance(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Idle;
        attempt_dance(data, &mut update);
        update
    }

    fn stand(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        // Try to Fall/Stand up/Move
        update.character = CharacterState::Idle;
        update
    }

    fn manipulate_loadout(&self, data: &JoinData, loadout_manip: LoadoutManip) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        handle_manipulate_loadout(&data, &mut update, loadout_manip);
        update
    }
}
