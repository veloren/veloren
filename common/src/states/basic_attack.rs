use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::{utils::*, wielding},
    sys::character_behavior::*,
};
use std::{collections::VecDeque, time::Duration};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data {
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Base damage
    pub base_damage: u32,
    /// Whether the attack can deal more damage
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate {
            pos: *data.pos,
            vel: *data.vel,
            ori: *data.ori,
            energy: *data.energy,
            character: *data.character,
            local_events: VecDeque::new(),
            server_events: VecDeque::new(),
        };

        handle_move(data, &mut update);

        // Build up
        if self.buildup_duration != Duration::default() {
            update.character = CharacterState::BasicAttack(Data {
                buildup_duration: self
                    .buildup_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                recover_duration: self.recover_duration,
                base_damage: self.base_damage,
                exhausted: false,
            });
        }
        // Hit attempt
        else if !self.exhausted {
            if let Some(tool) = unwrap_tool_data(data) {
                data.updater.insert(data.entity, Attacking {
                    base_damage: self.base_damage,
                    applied: false,
                    hit_count: 0,
                });
            }

            update.character = CharacterState::BasicAttack(Data {
                buildup_duration: self.buildup_duration,
                recover_duration: self.recover_duration,
                base_damage: self.base_damage,
                exhausted: true,
            });
        }
        // Recovery
        else if self.recover_duration != Duration::default() {
            update.character = CharacterState::BasicAttack(Data {
                buildup_duration: self.buildup_duration,
                recover_duration: self
                    .recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                base_damage: self.base_damage,
                exhausted: true,
            });
        }
        // Done
        else {
            if let Some(tool) = unwrap_tool_data(data) {
                update.character = CharacterState::Wielding;
                // Make sure attack component is removed
                data.updater.remove::<Attacking>(data.entity);
            } else {
                update.character = CharacterState::Idle;
            }
        }

        // Grant energy on successful hit
        if let Some(attack) = data.attacking {
            if attack.applied && attack.hit_count > 0 {
                data.updater.remove::<Attacking>(data.entity);
                update.energy.change_by(100, EnergySource::HitEnemy);
            }
        }

        update
    }
}
