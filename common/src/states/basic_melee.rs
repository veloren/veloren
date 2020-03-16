use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::*,
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
            character: data.character.clone(),
            local_events: VecDeque::new(),
            server_events: VecDeque::new(),
        };

        handle_move(data, &mut update);

        if self.buildup_duration != Duration::default() {
            // Build up
            update.character = CharacterState::BasicMelee(Data {
                buildup_duration: self
                    .buildup_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                recover_duration: self.recover_duration,
                base_damage: self.base_damage,
                exhausted: false,
            });
        } else if !self.exhausted {
            // Hit attempt
            if let Some(tool) = unwrap_tool_data(data) {
                data.updater.insert(data.entity, Attacking {
                    base_damage: self.base_damage,
                    applied: false,
                    hit_count: 0,
                });
            }

            update.character = CharacterState::BasicMelee(Data {
                buildup_duration: self.buildup_duration,
                recover_duration: self.recover_duration,
                base_damage: self.base_damage,
                exhausted: true,
            });
        } else if self.recover_duration != Duration::default() {
            // Recovery
            update.character = CharacterState::BasicMelee(Data {
                buildup_duration: self.buildup_duration,
                recover_duration: self
                    .recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                base_damage: self.base_damage,
                exhausted: true,
            });
        } else {
            // Done
            update.character = CharacterState::Wielding;
            // Make sure attack component is removed
            data.updater.remove::<Attacking>(data.entity);
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
