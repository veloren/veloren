use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::*,
    sys::character_behavior::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Whether the attack can currently deal damage
    pub exhausted: bool,
    /// Used for particle stuffs
    pub particle_ori: Option<Vec3<f32>>,
    /// How long until state should deal damage or heal
    pub buildup_duration: Duration,
    /// How long until weapon can deal another tick of damage
    pub cooldown_duration: Duration,
    /// Value that cooldown_duration defaults to
    pub cooldown_duration_default: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Base healing per second
    pub base_hps: u32,
    /// Base damage per second
    pub base_dps: u32,
    /// Max range
    pub range: f32,
    /// Max angle (45.0 will give you a 90.0 angle window)
    pub max_angle: f32,
    /// Lifesteal efficiency (0 gives 0% conversion of damage to health, 1 gives
    /// 100% conversion of damage to health)
    pub lifesteal_eff: f32,
    /// Energy regened per second
    pub energy_regen: u32,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.4);
        handle_jump(data, &mut update);

        let ticks_per_sec = 1.0 / self.cooldown_duration_default.as_secs_f32();

        if self.buildup_duration != Duration::default() {
            // Build up
            update.character = CharacterState::BasicBeam(Data {
                exhausted: self.exhausted,
                particle_ori: Some(*data.inputs.look_dir),
                buildup_duration: self
                    .buildup_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                cooldown_duration: self.cooldown_duration,
                cooldown_duration_default: self.cooldown_duration_default,
                recover_duration: self.recover_duration,
                base_hps: self.base_hps,
                base_dps: self.base_dps,
                range: self.range,
                max_angle: self.max_angle,
                lifesteal_eff: self.lifesteal_eff,
                energy_regen: self.energy_regen,
            });
        } else if data.inputs.primary.is_pressed() && !self.exhausted {
            let damage = (self.base_dps as f32 / ticks_per_sec) as u32;
            let heal = (self.base_hps as f32 / ticks_per_sec) as u32;
            // Hit attempt
            data.updater.insert(data.entity, Attacking {
                base_damage: damage,
                base_heal: heal,
                range: self.range,
                max_angle: self.max_angle.to_radians(),
                applied: false,
                hit_count: 0,
                knockback: 0.0,
                is_melee: false,
                lifesteal_eff: self.lifesteal_eff,
            });

            update.character = CharacterState::BasicBeam(Data {
                exhausted: true,
                particle_ori: Some(*data.inputs.look_dir),
                buildup_duration: self.buildup_duration,
                recover_duration: self.recover_duration,
                cooldown_duration: self.cooldown_duration_default,
                cooldown_duration_default: self.cooldown_duration_default,
                base_hps: self.base_hps,
                base_dps: self.base_dps,
                range: self.range,
                max_angle: self.max_angle,
                lifesteal_eff: self.lifesteal_eff,
                energy_regen: self.energy_regen,
            });
        } else if data.inputs.primary.is_pressed() && self.cooldown_duration != Duration::default()
        {
            // Cooldown until next tick of damage
            update.character = CharacterState::BasicBeam(Data {
                exhausted: self.exhausted,
                particle_ori: Some(*data.inputs.look_dir),
                buildup_duration: self.buildup_duration,
                cooldown_duration: self
                    .cooldown_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                cooldown_duration_default: self.cooldown_duration_default,
                recover_duration: self.recover_duration,
                base_hps: self.base_hps,
                base_dps: self.base_dps,
                range: self.range,
                max_angle: self.max_angle,
                lifesteal_eff: self.lifesteal_eff,
                energy_regen: self.energy_regen,
            });
        } else if data.inputs.primary.is_pressed() {
            update.character = CharacterState::BasicBeam(Data {
                exhausted: false,
                particle_ori: Some(*data.inputs.look_dir),
                buildup_duration: self.buildup_duration,
                recover_duration: self.recover_duration,
                cooldown_duration: self.cooldown_duration_default,
                cooldown_duration_default: self.cooldown_duration_default,
                base_hps: self.base_hps,
                base_dps: self.base_dps,
                range: self.range,
                max_angle: self.max_angle,
                lifesteal_eff: self.lifesteal_eff,
                energy_regen: self.energy_regen,
            });

            // Grant energy on successful hit
            let energy = (self.energy_regen as f32 / ticks_per_sec) as i32;
            update.energy.change_by(energy, EnergySource::HitEnemy);
        } else if self.recover_duration != Duration::default() {
            // Recovery
            update.character = CharacterState::BasicBeam(Data {
                exhausted: self.exhausted,
                particle_ori: Some(*data.inputs.look_dir),
                buildup_duration: self.buildup_duration,
                cooldown_duration: self.cooldown_duration,
                cooldown_duration_default: self.cooldown_duration_default,
                recover_duration: self
                    .recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                base_hps: self.base_hps,
                base_dps: self.base_dps,
                range: self.range,
                max_angle: self.max_angle,
                lifesteal_eff: self.lifesteal_eff,
                energy_regen: self.energy_regen,
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
