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
                if new_stage_time_active > tool.attack_buildup_duration() {
                    if !*stage_exhausted {
                        // Try to deal damage
                        data.updater.insert(data.entity, Attacking {
                            weapon: Some(*tool),
                            applied: false,
                            hit_count: 0,
                        });
                        new_stage_exhausted = true;
                    } else {
                        // Make sure to remove Attacking component
                        data.updater.remove::<Attacking>(data.entity);
                    }

                    // Check if player has timed click right
                    if data.inputs.primary.is_just_pressed() {
                        println!("Can transition");
                        new_can_transition = true;
                    }
                }

                if new_stage_time_active > tool.attack_duration() {
                    if new_can_transition {
                        update.character = CharacterState::TimedCombo {
                            tool: *tool,
                            stage: 2,
                            stage_time_active: Duration::default(),
                            stage_exhausted: false,
                            can_transition: false,
                        }
                    } else {
                        println!("Failed");
                        attempt_wield(data, &mut update);
                    }
                } else {
                    update.character = CharacterState::TimedCombo {
                        tool: *tool,
                        stage: 1,
                        stage_time_active: new_stage_time_active,
                        stage_exhausted: new_stage_exhausted,
                        can_transition: new_can_transition,
                    }
                }
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
