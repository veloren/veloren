use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::*,
    sys::character_behavior::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

const LEAP_SPEED: f32 = 16.0;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data {
    /// How long the state is moving
    pub movement_duration: Duration,
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Base damage
    pub base_damage: u32,
    /// Whether the attack can deal more damage
    pub exhausted: bool,
    pub initialize: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        if self.initialize {
            update.vel.0 = *data.inputs.look_dir * 20.0;
            if let Some(dir) = Vec3::from(data.inputs.look_dir.xy()).try_normalized() {
                update.ori.0 = dir.into();
            }
        }

        if self.movement_duration != Duration::default() {
            // Jumping
            update.vel.0 = Vec3::new(data.inputs.look_dir.x, data.inputs.look_dir.y, 8.0)
                * ((self.movement_duration.as_millis() as f32) / 250.0)
                + (update.vel.0 * Vec3::new(2.0, 2.0, 0.0)
                    + 0.25 * data.inputs.move_dir.try_normalized().unwrap_or_default())
                .try_normalized()
                .unwrap_or_default()
                    * LEAP_SPEED;

            update.character = CharacterState::LeapMelee(Data {
                movement_duration: self
                    .movement_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                buildup_duration: self.buildup_duration,
                recover_duration: self.recover_duration,
                base_damage: self.base_damage,
                exhausted: false,
                initialize: false,
            });
        } else if self.buildup_duration != Duration::default() && !data.physics.on_ground {
            // Falling
            update.character = CharacterState::LeapMelee(Data {
                movement_duration: Duration::default(),
                buildup_duration: self
                    .buildup_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                recover_duration: self.recover_duration,
                base_damage: self.base_damage,
                exhausted: false,
                initialize: false,
            });
        } else if !self.exhausted {
            // Hit attempt
            data.updater.insert(data.entity, Attacking {
                base_healthchange: -(self.base_damage as i32),
                range: 4.5,
                max_angle: 360_f32.to_radians(),
                applied: false,
                hit_count: 0,
                knockback: 25.0,
            });

            update.character = CharacterState::LeapMelee(Data {
                movement_duration: self.movement_duration,
                buildup_duration: Duration::default(),
                recover_duration: self.recover_duration,
                base_damage: self.base_damage,
                exhausted: true,
                initialize: false,
            });
        } else if self.recover_duration != Duration::default() {
            // Recovery
            handle_move(data, &mut update, 0.7);
            update.character = CharacterState::LeapMelee(Data {
                movement_duration: self.movement_duration,
                buildup_duration: self.buildup_duration,
                recover_duration: self
                    .recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                base_damage: self.base_damage,
                exhausted: true,
                initialize: false,
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
