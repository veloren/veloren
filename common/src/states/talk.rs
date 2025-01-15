use super::utils::*;
use crate::{
    comp::{CharacterState, InventoryAction, StateUpdate, character_state::OutputEvents},
    states::{
        behavior::{CharacterBehavior, JoinData},
        idle,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const TURN_RATE: f32 = 40.0;
const MIN_TALK_TIME: Duration = Duration::from_millis(500);

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data {
    pub timer: Duration,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_wield(data, &mut update);
        handle_orientation(data, &mut update, TURN_RATE, None);

        update.character = if self.timer >= MIN_TALK_TIME {
            CharacterState::Idle(idle::Data::default())
        } else {
            CharacterState::Talk(Self {
                timer: self.timer + Duration::from_secs_f32(data.dt.0),
                ..*self
            })
        };

        update
    }

    fn talk(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        // Refresh timer
        update.character = CharacterState::Talk(Self::default());

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

    fn sit(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Idle(idle::Data::default());
        attempt_sit(data, &mut update);
        update
    }

    fn dance(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Idle(idle::Data::default());
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
