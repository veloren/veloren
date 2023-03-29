use super::utils::*;
use crate::{
    comp::{
        character_state::OutputEvents, controller::InputKind, inventory::item::armor::Friction,
        CharacterState, InventoryAction, StateUpdate,
    },
    resources::Time,
    states::behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Data {
    pub is_sneaking: bool,
    pub(crate) time_entered: Time,
    // None means unknown
    pub(crate) footwear: Option<Friction>,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        const LEAVE_STANCE_DELAY: f64 = 30.0;
        if (self.time_entered.0 + LEAVE_STANCE_DELAY) < data.time.0 {
            leave_stance(data, output_events);
        }

        handle_skating(data, &mut update);
        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, if self.is_sneaking { 0.4 } else { 1.0 });
        handle_jump(data, output_events, &mut update, 1.0);
        handle_wield(data, &mut update);
        handle_climb(data, &mut update);
        handle_wallrun(data, &mut update);
        if input_is_pressed(data, InputKind::Roll) {
            handle_input(data, output_events, &mut update, InputKind::Roll);
        }

        // Try to Fall/Stand up/Move
        if self.is_sneaking
            && (data.physics.on_ground.is_none() || data.physics.in_liquid().is_some())
        {
            update.character = CharacterState::Idle(Data {
                is_sneaking: false,
                time_entered: self.time_entered,
                footwear: self.footwear,
            });
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
        handle_manipulate_loadout(data, output_events, &mut update, inv_action);
        update
    }

    fn wield(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_wield(data, &mut update);
        update
    }

    fn glide_wield(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_glide_wield(data, &mut update, output_events);
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
        update.character = CharacterState::Idle(Data {
            is_sneaking: true,
            time_entered: self.time_entered,
            footwear: self.footwear,
        });
        update
    }

    fn stand(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Idle(Data::default());
        update
    }

    fn talk(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_talk(data, &mut update);
        update
    }
}
