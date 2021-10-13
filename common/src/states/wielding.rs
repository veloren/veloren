use super::utils::*;
use crate::{
    comp::{
        character_state::OutputEvents,
        slot::{EquipSlot, Slot},
        CharacterState, InventoryAction, StateUpdate,
    },
    states::behavior::{CharacterBehavior, JoinData},
};

pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, 1.0);
        handle_climb(data, &mut update);
        attempt_input(data, output_events, &mut update);

        update
    }

    fn swap_equipped_weapons(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_swap_equipped_weapons(data, &mut update);
        update
    }

    fn manipulate_loadout(
        &self,
        data: &JoinData,
        output_events: &mut OutputEvents,
        inv_action: InventoryAction,
    ) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        match inv_action {
            InventoryAction::Drop(EquipSlot::ActiveMainhand | EquipSlot::ActiveOffhand)
            | InventoryAction::Swap(EquipSlot::ActiveMainhand | EquipSlot::ActiveOffhand, _)
            | InventoryAction::Swap(
                _,
                Slot::Equip(EquipSlot::ActiveMainhand | EquipSlot::ActiveOffhand),
            ) => {
                update.character = CharacterState::Idle;
            },
            _ => (),
        }
        handle_manipulate_loadout(data, output_events, &mut update, inv_action);
        update
    }

    fn glide_wield(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_glide_wield(data, &mut update);
        update
    }

    fn unwield(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Idle;
        update
    }

    fn sit(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_sit(data, &mut update);
        update
    }

    fn dance(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_dance(data, &mut update);
        update
    }

    fn sneak(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_sneak(data, &mut update);
        update
    }
}
