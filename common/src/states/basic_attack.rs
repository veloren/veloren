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
        remaining_duration,
    } = data.character
    {
        let tool_kind = data.stats.equipment.main.as_ref().map(|i| i.kind);
        if let Some(Tool(tool)) = tool_kind {
            handle_move(data, &mut update);

            let mut new_exhausted = *exhausted;

            if !*exhausted && *remaining_duration < tool.attack_recover_duration() {
                data.updater.insert(data.entity, Attacking {
                    weapon: Some(tool),
                    applied: false,
                    hit_count: 0,
                });
                new_exhausted = true;
            }

            let new_remaining_duration = remaining_duration
                .checked_sub(Duration::from_secs_f32(data.dt.0))
                .unwrap_or_default();

            if let Some(attack) = data.attacking {
                if attack.applied && attack.hit_count > 0 {
                    data.updater.remove::<Attacking>(data.entity);
                    update.energy.change_by(100, EnergySource::HitEnemy);
                }
            }

            // Tick down
            update.character = CharacterState::BasicAttack {
                remaining_duration: new_remaining_duration,
                exhausted: new_exhausted,
            };

            // Check if attack duration has expired
            if new_remaining_duration == Duration::default() {
                update.character = CharacterState::Wielding { tool };
                data.updater.remove::<Attacking>(data.entity);
            }

            update
        } else {
            update
        }
    } else {
        update.character = CharacterState::Idle {};
        update
    }
}
