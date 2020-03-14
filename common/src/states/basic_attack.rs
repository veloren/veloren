use crate::{
    comp::{Attacking, CharacterState, EnergySource, ItemKind::Tool, StateUpdate},
    states::utils::*,
    sys::character_behavior::JoinData,
};
use std::{collections::VecDeque, time::Duration};

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

    if let CharacterState::BasicAttack {
        exhausted,
        buildup_duration,
        recover_duration,
    } = data.character
    {
        let tool_kind = data.stats.equipment.main.as_ref().map(|i| i.kind);
        handle_move(data, &mut update);

        if buildup_duration != &Duration::default() {
            // Start to swing
            update.character = CharacterState::BasicAttack {
                buildup_duration: buildup_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                recover_duration: *recover_duration,
                exhausted: false,
            };
        } else if !*exhausted {
            // Swing hits
            if let Some(Tool(tool)) = tool_kind {
                data.updater.insert(data.entity, Attacking {
                    weapon: Some(tool),
                    applied: false,
                    hit_count: 0,
                });
            }

            update.character = CharacterState::BasicAttack {
                buildup_duration: *buildup_duration,
                recover_duration: *recover_duration,
                exhausted: true,
            };
        } else if recover_duration != &Duration::default() {
            // Recover from swing
            update.character = CharacterState::BasicAttack {
                buildup_duration: *buildup_duration,
                recover_duration: recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                exhausted: true,
            }
        } else {
            // Done
            if let Some(Tool(tool)) = tool_kind {
                update.character = CharacterState::Wielding { tool };
                data.updater.remove::<Attacking>(data.entity);
            } else {
                update.character = CharacterState::Idle;
            }
        }

        // More handling
        if let Some(attack) = data.attacking {
            if attack.applied && attack.hit_count > 0 {
                data.updater.remove::<Attacking>(data.entity);
                update.energy.change_by(100, EnergySource::HitEnemy);
            }
        }

        update
    } else {
        update.character = CharacterState::Idle {};
        update
    }
}
