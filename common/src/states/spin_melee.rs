use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    sys::character_behavior::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data {
    /// How long until the state attacks
    pub buildup_duration: Duration,
    /// Allows for buildup_duration to be reset to default value
    pub buildup_duration_default: Duration,
    /// How long until state ends
    pub recover_duration: Duration,
    /// Allows for recover_duration to be reset to default value
    pub recover_duration_default: Duration,
    /// Base damage
    pub base_damage: u32,
    /// Whether the attack can deal more damage
    pub exhausted: bool,
    /// How many hits it can do before ending
    pub hits_remaining: u32,
    /// Allows for hits_remaining to be reset to default value
    pub hits_remaining_default: u32,
    /// Energy cost per attack
    pub energy_cost: u32,
}

const MOVE_SPEED: f32 = 5.0;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        if self.buildup_duration != Duration::default() {
            // Allows for moving
            update.vel.0 =
                Vec3::new(data.inputs.move_dir.x, data.inputs.move_dir.y, 0.0) * MOVE_SPEED;

            update.character = CharacterState::SpinMelee(Data {
                buildup_duration: self
                    .buildup_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                buildup_duration_default: self.buildup_duration_default,
                recover_duration: self.recover_duration,
                recover_duration_default: self.recover_duration_default,
                base_damage: self.base_damage,
                exhausted: self.exhausted,
                hits_remaining: self.hits_remaining,
                hits_remaining_default: self.hits_remaining_default,
                energy_cost: self.energy_cost,
            });
        } else if !self.exhausted {
            //Hit attempt
            data.updater.insert(data.entity, Attacking {
                base_healthchange: -(self.base_damage as i32),
                range: 3.5,
                max_angle: 360_f32.to_radians(),
                applied: false,
                hit_count: 0,
                knockback: 0.0,
            });

            update.character = CharacterState::SpinMelee(Data {
                buildup_duration: self.buildup_duration,
                buildup_duration_default: self.buildup_duration_default,
                recover_duration: self.recover_duration,
                recover_duration_default: self.recover_duration_default,
                base_damage: self.base_damage,
                exhausted: true,
                hits_remaining: self.hits_remaining - 1,
                hits_remaining_default: self.hits_remaining_default,
                energy_cost: self.energy_cost,
            });
        } else if self.recover_duration != Duration::default() {
            // Allows for moving
            update.vel.0 =
                Vec3::new(data.inputs.move_dir.x, data.inputs.move_dir.y, 0.0) * MOVE_SPEED;

            update.character = CharacterState::SpinMelee(Data {
                buildup_duration: self.buildup_duration,
                buildup_duration_default: self.buildup_duration_default,
                recover_duration: self
                    .recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                recover_duration_default: self.recover_duration_default,
                base_damage: self.base_damage,
                exhausted: self.exhausted,
                hits_remaining: self.hits_remaining,
                hits_remaining_default: self.hits_remaining_default,
                energy_cost: self.energy_cost,
            });
        } else if self.hits_remaining != 0 {
            // Allows for one ability usage to have multiple hits
            // This isn't needed for it's continuous implementation, but is left in should
            // this skill be moved to the skillbar
            update.character = CharacterState::SpinMelee(Data {
                buildup_duration: self.buildup_duration_default,
                buildup_duration_default: self.buildup_duration_default,
                recover_duration: self.recover_duration_default,
                recover_duration_default: self.recover_duration_default,
                base_damage: self.base_damage,
                exhausted: false,
                hits_remaining: self.hits_remaining,
                hits_remaining_default: self.hits_remaining_default,
                energy_cost: self.energy_cost,
            });
        } else if update.energy.current() >= self.energy_cost && data.inputs.secondary.is_pressed()
        {
            update.character = CharacterState::SpinMelee(Data {
                buildup_duration: self.buildup_duration_default,
                buildup_duration_default: self.buildup_duration_default,
                recover_duration: self.recover_duration_default,
                recover_duration_default: self.recover_duration_default,
                base_damage: self.base_damage,
                exhausted: false,
                hits_remaining: self.hits_remaining_default,
                hits_remaining_default: self.hits_remaining_default,
                energy_cost: self.energy_cost,
            });
            // Consumes energy if there's enough left and RMB is held down
            update
                .energy
                .change_by(-(self.energy_cost as i32), EnergySource::Ability);
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
                update.energy.change_by(10, EnergySource::HitEnemy);
            }
        }

        update
    }
}
