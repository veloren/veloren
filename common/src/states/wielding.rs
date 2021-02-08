use super::utils::*;
use crate::{
    comp::{
        slot::{EquipSlot, Slot},
        CharacterState, LoadoutManip, StateUpdate,
    },
    states::behavior::{CharacterBehavior, JoinData},
};

pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(&data, &mut update, 1.0);
        handle_jump(&data, &mut update);
        handle_climb(&data, &mut update);
        handle_ability1_input(&data, &mut update);
        handle_ability2_input(&data, &mut update);
        handle_ability3_input(&data, &mut update);
        handle_dodge_input(&data, &mut update);

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

    fn sneak(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_sneak(data, &mut update);
        update
    }

    fn unwield(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Idle;
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

    fn manipulate_loadout(&self, data: &JoinData, loadout_manip: LoadoutManip) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        match loadout_manip {
            LoadoutManip::Drop(EquipSlot::Mainhand)
            | LoadoutManip::Swap(EquipSlot::Mainhand, _)
            | LoadoutManip::Swap(_, Slot::Equip(EquipSlot::Mainhand)) => {
                update.character = CharacterState::Idle;
            },
            _ => (),
        }
        handle_manipulate_loadout(&data, &mut update, loadout_manip);
        update
    }
}
