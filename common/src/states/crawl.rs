use super::utils::*;
use crate::{
    comp::{CharacterState, StateUpdate, character_state::OutputEvents},
    states::behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Data;

fn can_stand(data: &JoinData) -> bool {
    data.health
        .is_none_or(|health| !health.has_consumed_death_protection())
}

// NOTE: In the future we might want to allow using some items while downed, but
// right now we just ignore those events.
impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        leave_stance(data, output_events);
        handle_orientation(data, &mut update, 0.2, None);
        handle_move(data, &mut update, 0.2);

        update
    }

    fn sit(&self, data: &JoinData, _output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        if can_stand(data) {
            attempt_sit(data, &mut update);
        }
        update
    }

    fn stand(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        if can_stand(data) {
            update.character = CharacterState::Idle(Default::default());
        }
        update
    }

    fn dance(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        if can_stand(data) {
            attempt_dance(data, &mut update);
        }
        update
    }
}
