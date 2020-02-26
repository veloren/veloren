use super::utils::*;
use crate::{
    comp::{CharacterState, StateUpdate},
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use std::{collections::VecDeque, time::Duration};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data {
    /// Time left before next state
    pub time_left: Duration,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *data.character,
            pos: *data.pos,
            vel: *data.vel,
            ori: *data.ori,
            energy: *data.energy,
            local_events: VecDeque::new(),
            server_events: VecDeque::new(),
        };

        handle_move(&data, &mut update);
        handle_jump(&data, &mut update);

        if self.time_left == Duration::default() {
            // Wield delay has expired
            update.character = CharacterState::Wielding;
        } else {
            // Wield delay hasn't expired yet
            // Update wield delay
            update.character = CharacterState::Equipping(Data {
                time_left: self
                    .time_left
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
            });
        }

        update
    }
}
