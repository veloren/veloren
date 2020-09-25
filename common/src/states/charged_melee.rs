use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::*,
    sys::character_behavior::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Whether the attack fired already
    pub exhausted: bool,
    /// How much energy is drained per second when charging
    pub energy_drain: u32,
    /// How much damage is dealt with no charge
    pub initial_damage: u32,
    /// How much damage is dealt with max charge
    pub max_damage: u32,
    /// How much knockback there is with no charge
    pub initial_knockback: f32,
    /// How much knockback there is at max charge
    pub max_knockback: f32,
    /// How long the weapon needs to be prepared for
    pub prepare_duration: Duration,
    /// How long it takes to charge the weapon to max damage and knockback
    pub charge_duration: Duration,
    /// How long the state has been charging
    pub charge_timer: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Max range
    pub range: f32,
    /// Max angle (45.0 will give you a 90.0 angle window)
    pub max_angle: f32,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.3);
        handle_jump(data, &mut update);

        if self.prepare_duration != Duration::default() {
            // Prepare (draw back weapon)
            update.character = CharacterState::ChargedMelee(Data {
                exhausted: self.exhausted,
                energy_drain: self.energy_drain,
                initial_damage: self.initial_damage,
                max_damage: self.max_damage,
                initial_knockback: self.initial_knockback,
                max_knockback: self.max_knockback,
                prepare_duration: self
                    .prepare_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                charge_duration: self.charge_duration,
                charge_timer: self.charge_timer,
                recover_duration: self.recover_duration,
                range: self.range,
                max_angle: self.max_angle,
            });
        } else if data.inputs.secondary.is_pressed()
            && self.charge_timer < self.charge_duration
            && update.energy.current() > 0
        {
            // Charge the attack
            update.character = CharacterState::ChargedMelee(Data {
                exhausted: self.exhausted,
                energy_drain: self.energy_drain,
                initial_damage: self.initial_damage,
                max_damage: self.max_damage,
                initial_knockback: self.initial_knockback,
                max_knockback: self.max_knockback,
                prepare_duration: self.prepare_duration,
                charge_timer: self
                    .charge_timer
                    .checked_add(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                charge_duration: self.charge_duration,
                recover_duration: self.recover_duration,
                range: self.range,
                max_angle: self.max_angle,
            });

            // Consumes energy if there's enough left and RMB is held down
            update.energy.change_by(
                -(self.energy_drain as f32 * data.dt.0) as i32,
                EnergySource::Ability,
            );
        } else if data.inputs.secondary.is_pressed() {
            // Charge the attack
            update.character = CharacterState::ChargedMelee(Data {
                exhausted: self.exhausted,
                energy_drain: self.energy_drain,
                initial_damage: self.initial_damage,
                max_damage: self.max_damage,
                initial_knockback: self.initial_knockback,
                max_knockback: self.max_knockback,
                prepare_duration: self.prepare_duration,
                charge_timer: self.charge_timer,
                charge_duration: self.charge_duration,
                recover_duration: self.recover_duration,
                range: self.range,
                max_angle: self.max_angle,
            });

            // Consumes energy if there's enough left and RMB is held down
            update.energy.change_by(
                -(self.energy_drain as f32 * data.dt.0 / 5.0) as i32,
                EnergySource::Ability,
            );
        } else if !self.exhausted {
            let charge_amount =
                (self.charge_timer.as_secs_f32() / self.charge_duration.as_secs_f32()).min(1.0);
            let damage = self.initial_damage as f32 + (charge_amount * (self.max_damage - self.initial_damage) as f32);
            // Hit attempt
            data.updater.insert(data.entity, Attacking {
                base_damage: damage as u32,
                base_heal: 0,
                range: self.range,
                max_angle: self.max_angle.to_radians(),
                applied: false,
                hit_count: 0,
                knockback: self.initial_knockback
                    + charge_amount * (self.max_knockback - self.initial_knockback),
            });

            update.character = CharacterState::ChargedMelee(Data {
                exhausted: true,
                energy_drain: self.energy_drain,
                initial_damage: self.initial_damage,
                max_damage: self.max_damage,
                initial_knockback: self.initial_knockback,
                max_knockback: self.max_knockback,
                prepare_duration: self.prepare_duration,
                charge_timer: self.charge_timer,
                charge_duration: self.charge_duration,
                recover_duration: self.recover_duration,
                range: self.range,
                max_angle: self.max_angle,
            });
        } else if self.recover_duration != Duration::default() {
            // Recovery
            update.character = CharacterState::ChargedMelee(Data {
                exhausted: self.exhausted,
                energy_drain: self.energy_drain,
                initial_damage: self.initial_damage,
                max_damage: self.max_damage,
                initial_knockback: self.initial_knockback,
                max_knockback: self.max_knockback,
                prepare_duration: self.prepare_duration,
                charge_timer: self.charge_timer,
                charge_duration: self.charge_duration,
                recover_duration: self
                    .recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                range: self.range,
                max_angle: self.max_angle,
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
