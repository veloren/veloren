use super::utils::*;
use crate::{
    comp::{character_state::OutputEvents, CharacterState, Stance, StateUpdate},
    event::ServerEvent,
    states::behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    pub buildup_duration: Duration,
    pub stance: Stance,
    pub ability_info: AbilityInfo,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub static_data: StaticData,
    pub timer: Duration,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, 1.0);
        handle_jump(data, output_events, &mut update, 1.0);
        handle_interrupts(data, &mut update);

        if self.timer < self.static_data.buildup_duration {
            if let CharacterState::BasicStance(c) = &mut update.character {
                c.timer = tick_attack_or_default(data, self.timer, None);
            }
        } else {
            let stance = if Some(self.static_data.stance) == data.stance.copied() {
                Stance::None
            } else {
                self.static_data.stance
            };
            output_events.emit_server(ServerEvent::ChangeStance {
                entity: data.entity,
                stance,
            });
            end_ability(data, &mut update);
        }

        update
    }
}
