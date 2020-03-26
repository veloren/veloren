use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::*,
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use std::time::Duration;
use vek::vec::{Vec2, Vec3};

// In millis
const STAGE_DURATION: u64 = 600;

const INITIAL_ACCEL: f32 = 200.0;
const BASE_SPEED: f32 = 25.0;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum Stage {
    First,
    Second,
    Third,
}

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
    /// What stage (of 3) the attack is in.
    pub stage: Stage,
    /// How long current stage has been active
    pub stage_time_active: Duration,
    /// Whether current stage has exhausted its attack
    pub stage_exhausted: bool,
    /// Whether to go to next stage
    pub should_transition: bool,
    /// Whether state has performed intialization logic
    pub initialized: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        let stage_time_active = self
            .stage_time_active
            .checked_add(Duration::from_secs_f32(data.dt.0))
            .unwrap_or(Duration::default());

        // If player stops holding input, don't go to the next stage
        let mut should_transition = data.inputs.primary.is_pressed() && self.should_transition;

        if !self.initialized {
            update.vel.0 = Vec3::zero();
            if let Some(dir) = data.inputs.look_dir.try_normalized() {
                update.ori.0 = dir;
            }
        }
        let initialized = true;

        // Handle hit applied
        if let Some(attack) = data.attacking {
            if attack.applied {
                if attack.hit_count > 0 {
                    // Take energy on successful hit
                    update.energy.change_by(100, EnergySource::HitEnemy);
                } else {
                    // Prevent transition on failure
                    should_transition = false;
                }
                // Always remove component
                data.updater.remove::<Attacking>(data.entity);
            }
        }

        // Handling movement
        if stage_time_active < Duration::from_millis(STAGE_DURATION / 3) {
            let adjusted_accel = match (self.stage, data.physics.touch_entity.is_none()) {
                (Stage::First, true) => INITIAL_ACCEL,
                (Stage::Second, true) => INITIAL_ACCEL * 0.75,
                (Stage::Third, true) => INITIAL_ACCEL * 0.75,
                (_, _) => 0.0,
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
            handle_orientation(data, &mut update, 20.0);
        }

        // Handling attacking
        update.character = if stage_time_active > Duration::from_millis(STAGE_DURATION / 2)
            && !self.stage_exhausted
        {
            let dmg = match self.stage {
                Stage::First => self.base_damage / 2,
                Stage::Second => self.base_damage,
                Stage::Third => (self.base_damage as f32 * 1.5) as u32,
            };

            // Try to deal damage in second half of stage
            data.updater.insert(data.entity, Attacking {
                base_healthchange: -(dmg as i32),
                range: 3.5,
                max_angle: 180_f32.to_radians(),
                applied: false,
                hit_count: 0,
                knockback: 7.0,
            });

            CharacterState::TripleStrike(Data {
                base_damage: self.base_damage,
                stage: self.stage,
                stage_time_active,
                stage_exhausted: true,
                should_transition,
                initialized,
            })
        } else if stage_time_active > Duration::from_millis(STAGE_DURATION) {
            let next_stage = match self.stage {
                _ if !should_transition => None,
                Stage::First => Some(Stage::Second),
                Stage::Second => Some(Stage::Third),
                Stage::Third => None,
            };

            if let Some(stage) = next_stage {
                CharacterState::TripleStrike(Data {
                    base_damage: self.base_damage,
                    stage,
                    stage_time_active: Duration::default(),
                    stage_exhausted: false,
                    should_transition,
                    initialized,
                })
            } else {
                // Make sure attack component is removed
                data.updater.remove::<Attacking>(data.entity);
                // Done
                CharacterState::Wielding
            }
        } else {
            CharacterState::TripleStrike(Data {
                base_damage: self.base_damage,
                stage: self.stage,
                stage_time_active,
                stage_exhausted: self.stage_exhausted,
                should_transition,
                initialized,
            })
        };

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
