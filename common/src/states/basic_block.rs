use super::utils::*;
use crate::{
    comp::{CharacterState, ItemKind, StateUpdate},
    sys::character_behavior::JoinData,
};
use std::collections::VecDeque;

// const BLOCK_ACCEL: f32 = 30.0;
// const BLOCK_SPEED: f32 = 75.0;

pub fn behavior(data: &JoinData) -> StateUpdate {
    let mut update = StateUpdate {
        pos: *data.pos,
        vel: *data.vel,
        ori: *data.ori,
        energy: *data.energy,
        character: *data.character,
        local_events: VecDeque::new(),
        server_events: VecDeque::new(),
    };

    handle_move(&data, &mut update);

    if !data.physics.on_ground || !data.inputs.secondary.is_pressed() {
        if let Some(ItemKind::Tool(tool)) = data.stats.equipment.main.as_ref().map(|i| i.kind) {
            update.character = CharacterState::Equipping {
                tool,
                time_left: tool.equip_time(),
            };
        } else {
            update.character = CharacterState::Idle {};
        };
    }
    update
}
