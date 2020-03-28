use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::*,
    sys::character_behavior::*,
    util::Dir,
};
use std::time::Duration;
use vek::Vec3;

const DASH_SPEED: f32 = 19.0;

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
    pub initialize: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        if self.initialize {
            update.vel.0 = *data.inputs.look_dir * 20.0;
            if let Some(dir) = Vec3::from(data.vel.0.xy()).try_normalized() {
                update.ori.0 = dir.into();
            }
        }

        if self.buildup_duration != Duration::default() && data.physics.touch_entity.is_none() {
            // Build up (this will move you forward)
            update.vel.0 = Vec3::new(0.0, 0.0, update.vel.0.z)
                + (update.vel.0 * Vec3::new(1.0, 1.0, 0.0)
                    + 1.5 * data.inputs.move_dir.try_normalized().unwrap_or_default())
                .try_normalized()
                .unwrap_or_default()
                    * DASH_SPEED;

            update.character = CharacterState::DashMelee(Data {
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
                range: 3.5,
                max_angle: 180_f32.to_radians(),
                applied: false,
                hit_count: 0,
                knockback: 0.0,
            });

            update.character = CharacterState::DashMelee(Data {
                buildup_duration: Duration::default(),
                recover_duration: self.recover_duration,
                base_damage: self.base_damage,
                exhausted: true,
                initialize: false,
            });
        } else if self.recover_duration != Duration::default() {
            // Recovery
            handle_move(data, &mut update, 0.7);
            update.character = CharacterState::DashMelee(Data {
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

        update.ori.0 = Dir::slerp_to_vec3(update.ori.0, update.vel.0, 9.0 * data.dt.0);

        update
    }
}
