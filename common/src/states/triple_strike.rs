use crate::{
    comp::{Attacking, CharacterState, ItemKind::Tool, StateUpdate},
    states::utils::*,
    sys::character_behavior::JoinData,
};
use std::{collections::VecDeque, time::Duration};

/// ### This behavior is a series of 3 attacks in sequence.
///
/// Holding down the `primary` button executes a series of 3 attacks,
/// each one moves the player forward as the character steps into the swings.
/// The player can let go of the left mouse button at any time
/// and stop their attacks by interrupting the attack animation.
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

    if let CharacterState::TripleStrike {
        tool,
        stage,
        stage_time_active,
        stage_exhausted,
    } = data.character
    {
        let mut new_stage_exhausted = *stage_exhausted;
        let new_stage_time_active = stage_time_active
            .checked_add(Duration::from_secs_f32(data.dt.0))
            .unwrap_or(Duration::default());

        if !data.inputs.primary.is_pressed() {
            attempt_wield(data, &mut update);
        }

        match stage {
            1 => {
                println!("1");
                attempt_wield(data, &mut update);
            },
            2 => {
                println!("2");
                attempt_wield(data, &mut update);
            },
            3 => {
                println!("3");
                attempt_wield(data, &mut update);
            },
            _ => {
                // Should never get here.
            },
        }
    }

    update
}
