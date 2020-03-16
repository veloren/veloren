use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::{utils::*, wielding},
    sys::character_behavior::*,
};
use std::{collections::VecDeque, time::Duration};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// How long the state has until exiting
    pub duration: Duration,
    pub only_up: bool,
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

        // Still going
        if self.duration != Duration::default() {
            if self.only_up {
                update.vel.0.z = 30.0;
            } else {
                update.vel.0 = data.inputs.look_dir * 30.0;
            }
            update.character = CharacterState::Boost(Data {
                duration: self
                    .duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                only_up: self.only_up,
            });
        }
        // Done
        else {
            update.character = CharacterState::Wielding;
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
