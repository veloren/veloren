use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::*,
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use std::{collections::VecDeque, time::Duration};
use vek::vec::Vec2;

// In millis
const STAGE_DURATION: u64 = 600;

const BASE_ACCEL: f32 = 200.0;
const BASE_SPEED: f32 = 250.0;
/// ### A sequence of 3 incrementally increasing attacks.
///
/// While holding down the `primary` button, perform a series of 3 attacks,
/// each one pushes the player forward as the character steps into the swings.
/// The player can let go of the left mouse button at any time
/// and stop their attacks by interrupting the attack animation.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data {
    /// The tool this state will read to handle damage, etc.
    pub base_damage: u32,
    /// `int` denoting what stage (of 3) the attack is in.
    pub stage: i8,
    /// How long current stage has been active
    pub stage_time_active: Duration,
    /// Whether current stage has exhausted its attack
    pub stage_exhausted: bool,
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

        let new_stage_time_active = self
            .stage_time_active
            .checked_add(Duration::from_secs_f32(data.dt.0))
            .unwrap_or(Duration::default());

        // If player stops holding input,
        if !data.inputs.primary.is_pressed() {
            // Done
            update.character = CharacterState::Wielding;
            // Make sure attack component is removed
            data.updater.remove::<Attacking>(data.entity);
            return update;
        }

        if self.stage < 3 {
            if new_stage_time_active < Duration::from_millis(STAGE_DURATION / 3) {
                // Move player forward while in first third of each stage
                // Move player according to move_dir
                if update.vel.0.magnitude_squared() < BASE_SPEED.powf(2.0) {
                    update.vel.0 =
                        update.vel.0 + Vec2::broadcast(data.dt.0) * data.ori.0 * BASE_ACCEL;
                    let mag2 = update.vel.0.magnitude_squared();
                    if mag2 > BASE_SPEED.powf(2.0) {
                        update.vel.0 = update.vel.0.normalized() * BASE_SPEED;
                    }
                };

                update.character = CharacterState::TripleStrike(Data {
                    base_damage: self.base_damage,
                    stage: self.stage,
                    stage_time_active: new_stage_time_active,
                    stage_exhausted: false,
                });
            } else if new_stage_time_active > Duration::from_millis(STAGE_DURATION / 2)
                && !self.stage_exhausted
            {
                // Allow player to influence orientation a little
                handle_move(data, &mut update);

                // Try to deal damage in second half of stage
                data.updater.insert(data.entity, Attacking {
                    base_damage: self.base_damage * (self.stage as u32 + 1),
                    max_angle: 180_f32.to_radians(),
                    applied: false,
                    hit_count: 0,
                });

                update.character = CharacterState::TripleStrike(Data {
                    base_damage: self.base_damage,
                    stage: self.stage,
                    stage_time_active: new_stage_time_active,
                    stage_exhausted: true,
                });
            } else if new_stage_time_active > Duration::from_millis(STAGE_DURATION) {
                // Allow player to influence orientation a little
                handle_move(data, &mut update);

                update.character = CharacterState::TripleStrike(Data {
                    base_damage: self.base_damage,
                    stage: self.stage + 1,
                    stage_time_active: Duration::default(),
                    stage_exhausted: false,
                });
            } else {
                update.character = CharacterState::TripleStrike(Data {
                    base_damage: self.base_damage,
                    stage: self.stage,
                    stage_time_active: new_stage_time_active,
                    stage_exhausted: self.stage_exhausted,
                });
            }
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
                4
            }
        }

        update
    }
}
