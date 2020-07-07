use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::*,
    sys::character_behavior::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// How long the state has until exiting
    pub duration: Duration,
    pub only_up: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 1.0);

        // Still going
        if self.duration != Duration::default() {
            if self.only_up {
                update.vel.0.z += 500.0 * data.dt.0;
            } else {
                update.vel.0 += *data.inputs.look_dir * 500.0 * data.dt.0;
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
