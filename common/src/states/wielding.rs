use super::utils::*;
use crate::{
    comp::{
        slot::{EquipSlot, Slot},
        CharacterState, InputKind, InventoryAction, StateUpdate,
    },
    states::behavior::{CharacterBehavior, JoinData},
    uid::Uid,
};

pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(&data, &mut update, 1.0);
        handle_jump(&data, &mut update);
        handle_climb(&data, &mut update);
        //handle_ability1_input(&data, &mut update);
        handle_ability2_input(&data, &mut update);
        handle_ability3_input(&data, &mut update);
        handle_ability4_input(&data, &mut update);
        handle_dodge_input(&data, &mut update);

        update
    }

    fn handle_input(
        &self,
        data: &JoinData,
        ability: InputKind,
        _target: Option<Uid>,
    ) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        handle_input(&data, &mut update, ability);

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

    fn manipulate_loadout(&self, data: &JoinData, inv_action: InventoryAction) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        match inv_action {
            InventoryAction::Drop(EquipSlot::Mainhand)
            | InventoryAction::Swap(EquipSlot::Mainhand, _)
            | InventoryAction::Swap(_, Slot::Equip(EquipSlot::Mainhand)) => {
                update.character = CharacterState::Idle;
            },
            _ => (),
        }
        handle_manipulate_loadout(&data, &mut update, inv_action);
        update
    }
}
