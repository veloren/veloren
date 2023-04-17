use super::utils::*;
use crate::{
    comp::{
        character_state::OutputEvents,
        slot::{EquipSlot, Slot},
        CharacterState, InventoryAction, StateUpdate,
    },
    states::{
        behavior::{CharacterBehavior, JoinData},
        idle,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Data {
    pub is_sneaking: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, if self.is_sneaking { 0.4 } else { 1.0 });
        handle_climb(data, &mut update);
        attempt_input(data, output_events, &mut update);
        handle_jump(data, output_events, &mut update, 1.0);

        if self.is_sneaking
            && (data.physics.on_ground.is_none() || data.physics.in_liquid().is_some())
        {
            update.character = CharacterState::Wielding(Data { is_sneaking: false });
        }

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
        let reset_to_idle = match inv_action {
            InventoryAction::Drop(slot)
            | InventoryAction::Swap(slot, _)
            | InventoryAction::Swap(_, Slot::Equip(slot))
                if matches!(slot, EquipSlot::ActiveMainhand | EquipSlot::ActiveOffhand) =>
            {
                true
            },
            InventoryAction::Use(_) => true,
            _ => false,
        };
        if reset_to_idle {
            update.character = CharacterState::Idle(idle::Data {
                is_sneaking: data.character.is_stealthy(),
                time_entered: *data.time,
                footwear: None,
            });
        }
        handle_manipulate_loadout(data, output_events, &mut update, inv_action);
        update
    }

    fn glide_wield(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_glide_wield(data, &mut update, output_events);
        update
    }

    fn unwield(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Idle(idle::Data {
            is_sneaking: self.is_sneaking,
            time_entered: *data.time,
            footwear: None,
        });
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
        if data.physics.on_ground.is_some() && data.body.is_humanoid() {
            update.character = CharacterState::Wielding(Data { is_sneaking: true });
        }
        update
    }

    fn stand(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Wielding(Data { is_sneaking: false });
        update
    }
}
