use super::utils::*;
use crate::{
    comp::{CharacterState, StateUpdate},
    sys::character_behavior::JoinData,
};
use std::{collections::VecDeque, time::Duration};

pub fn behavior(data: &JoinData) -> StateUpdate {
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

    if let CharacterState::Equipping { tool, time_left } = data.character {
        if *time_left == Duration::default() {
            // Wield delay has expired
            update.character = CharacterState::Wielding { tool: *tool };
        } else {
            // Wield delay hasn't expired yet
            // Update wield delay
            update.character = CharacterState::Equipping {
                time_left: time_left
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                tool: *tool,
            };
        }
    }
    update
}
