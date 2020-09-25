use crate::{
    comp::{Attacking, CharacterState, StateUpdate},
    states::utils::*,
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
//#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
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
    /// Max range
    pub range: f32,
    /// Max angle (45.0 will give you a 90.0 angle window)
    pub max_angle: f32,
    /// Knockback
    pub knockback: f32,
    /// Leap speed
    pub leap_speed: f32,
    /// Leap vertical speed?
    pub leap_vert_speed: f32,
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
            update.vel.0 = Vec3::new(
                data.inputs.look_dir.x,
                data.inputs.look_dir.y,
                self.leap_vert_speed,
            ) * (2.0)
                + (update.vel.0 * Vec3::new(2.0, 2.0, 0.0)
                    + 0.25 * data.inputs.move_dir.try_normalized().unwrap_or_default())
                .try_normalized()
                .unwrap_or_default()
                    * self.leap_speed
                    * (1.0 - data.inputs.look_dir.z.abs());

            update.character = CharacterState::LeapMelee(Data {
                movement_duration: self
                    .movement_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                buildup_duration: self.buildup_duration,
                recover_duration: self.recover_duration,
                base_damage: self.base_damage,
                exhausted: false,
                range: self.range,
                max_angle: self.max_angle,
                knockback: self.knockback,
                leap_speed: self.leap_speed,
                leap_vert_speed: self.leap_vert_speed,
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
                range: self.range,
                max_angle: self.max_angle,
                knockback: self.knockback,
                leap_speed: self.leap_speed,
                leap_vert_speed: self.leap_vert_speed,
                initialize: false,
            });
        } else if !self.exhausted {
            // Hit attempt
            data.updater.insert(data.entity, Attacking {
                base_damage: self.base_damage,
                base_heal: 0,
                range: self.range,
                max_angle: self.max_angle.to_radians(),
                applied: false,
                hit_count: 0,
                knockback: self.knockback,
            });

            update.character = CharacterState::LeapMelee(Data {
                movement_duration: self.movement_duration,
                buildup_duration: Duration::default(),
                recover_duration: self.recover_duration,
                base_damage: self.base_damage,
                exhausted: true,
                range: self.range,
                max_angle: self.max_angle,
                knockback: self.knockback,
                leap_speed: self.leap_speed,
                leap_vert_speed: self.leap_vert_speed,
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
                range: self.range,
                max_angle: self.max_angle,
                knockback: self.knockback,
                leap_speed: self.leap_speed,
                leap_vert_speed: self.leap_vert_speed,
                initialize: false,
            });
        } else {
            // Done
            update.character = CharacterState::Wielding;
            // Make sure attack component is removed
            data.updater.remove::<Attacking>(data.entity);
        }

        update
    }
}
