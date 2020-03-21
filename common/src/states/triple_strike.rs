use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::*,
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use std::time::Duration;
use vek::vec::Vec2;

// In millis
const STAGE_DURATION: u64 = 600;

const INITIAL_ACCEL: f32 = 200.0;
const SECONDARY_ACCEL: f32 = 100.0;
const BASE_SPEED: f32 = 25.0;
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
    /// Whether to go to next stage
    pub should_transition: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update= StateUpdate::from(data);

        let stage_time_active = self
            .stage_time_active
            .checked_add(Duration::from_secs_f32(data.dt.0))
            .unwrap_or(Duration::default());

        let mut should_transition = self.should_transition;

        // If player stops holding input,
        if !data.inputs.primary.is_pressed() {
            // // Done
            // update.character = CharacterState::Wielding;
            // // Make sure attack component is removed
            // data.updater.remove::<Attacking>(data.entity);
            // return update;

            should_transition = false;
        }

        if self.stage < 3 {
            // Handling movement
            if stage_time_active < Duration::from_millis(STAGE_DURATION / 3) {
                let adjusted_accel = if self.stage == 0 {
                    INITIAL_ACCEL
                } else {
                    SECONDARY_ACCEL
                };

                // Move player forward while in first third of each stage
                if update.vel.0.magnitude_squared() < BASE_SPEED.powf(2.0) {
                    update.vel.0 =
                        update.vel.0 + Vec2::broadcast(data.dt.0) * data.ori.0 * adjusted_accel;
                    let mag2 = update.vel.0.magnitude_squared();
                    if mag2 > BASE_SPEED.powf(2.0) {
                        update.vel.0 = update.vel.0.normalized() * BASE_SPEED;
                    }
                };
            } else {
                handle_orientation(data, &mut update, 10.0);
            }

            // Handling attacking
            if stage_time_active > Duration::from_millis(STAGE_DURATION / 2)
                && !self.stage_exhausted
            {
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
                    stage_time_active,
                    stage_exhausted: true,
                    should_transition,
                });
            } else if stage_time_active > Duration::from_millis(STAGE_DURATION) {
                if should_transition {
                    update.character = CharacterState::TripleStrike(Data {
                        base_damage: self.base_damage,
                        stage: self.stage + 1,
                        stage_time_active: Duration::default(),
                        stage_exhausted: false,
                        should_transition,
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                    // Make sure attack component is removed
                    data.updater.remove::<Attacking>(data.entity);
                }
            } else {
                update.character = CharacterState::TripleStrike(Data {
                    base_damage: self.base_damage,
                    stage: self.stage,
                    stage_time_active,
                    stage_exhausted: self.stage_exhausted,
                    should_transition,
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
            }
        }

        update
    }
}
