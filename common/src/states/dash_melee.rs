use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::{StageSection, *},
    sys::character_behavior::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// How much damage the attack initially does
    pub base_damage: u32,
    /// How much damage the attack does at max charge distance
    pub max_damage: u32,
    /// How much the attack knocks the target back initially
    pub base_knockback: f32,
    /// How much knockback happens at max charge distance
    pub max_knockback: f32,
    /// Range of the attack
    pub range: f32,
    /// Angle of the attack
    pub angle: f32,
    /// Rate of energy drain
    pub energy_drain: u32,
    /// How quickly dasher moves forward
    pub forward_speed: f32,
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state charges for until it reaches max damage
    pub charge_duration: Duration,
    /// How high timer got while in charge potion
    pub charge_duration_attained: Duration,
    /// Whether state keeps charging after reaching max charge duration
    pub infinite_charge: bool,
    /// How long the state swings for
    pub swing_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0);
        handle_move(data, &mut update, 0.1);

        if self.stage_section == StageSection::Buildup && self.timer < self.buildup_duration {
            // Build up
            update.character = CharacterState::DashMelee(Data {
                base_damage: self.base_damage,
                max_damage: self.max_damage,
                base_knockback: self.base_knockback,
                max_knockback: self.max_knockback,
                range: self.range,
                angle: self.angle,
                energy_drain: self.energy_drain,
                forward_speed: self.forward_speed,
                buildup_duration: self.buildup_duration,
                charge_duration: self.charge_duration,
                charge_duration_attained: self.charge_duration_attained,
                infinite_charge: self.infinite_charge,
                swing_duration: self.swing_duration,
                recover_duration: self.recover_duration,
                timer: self
                    .timer
                    .checked_add(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                stage_section: self.stage_section,
            })
        } else if self.stage_section == StageSection::Buildup {
            // Transitions to charge section of stage
            update.character = CharacterState::DashMelee(Data {
                base_damage: self.base_damage,
                max_damage: self.max_damage,
                base_knockback: self.base_knockback,
                max_knockback: self.max_knockback,
                range: self.range,
                angle: self.angle,
                energy_drain: self.energy_drain,
                forward_speed: self.forward_speed,
                buildup_duration: self.buildup_duration,
                charge_duration: self.charge_duration,
                charge_duration_attained: self.charge_duration_attained,
                infinite_charge: self.infinite_charge,
                swing_duration: self.swing_duration,
                recover_duration: self.recover_duration,
                timer: Duration::default(),
                stage_section: StageSection::Charge,
            })
        } else if self.stage_section == StageSection::Charge /*&& data.physics.touch_entities.is_empty()*/ && ((self.timer < self.charge_duration && !self.infinite_charge) || (data.inputs.secondary.is_pressed() && self.infinite_charge)) && update.energy.current() > 0
        {
            // Forward movement
            forward_move(data, &mut update, 0.1, self.forward_speed);

            // Charges
            update.character = CharacterState::DashMelee(Data {
                base_damage: self.base_damage,
                max_damage: self.max_damage,
                base_knockback: self.base_knockback,
                max_knockback: self.max_knockback,
                range: self.range,
                angle: self.angle,
                energy_drain: self.energy_drain,
                forward_speed: self.forward_speed,
                buildup_duration: self.buildup_duration,
                charge_duration: self.charge_duration,
                charge_duration_attained: self.charge_duration_attained,
                infinite_charge: self.infinite_charge,
                swing_duration: self.swing_duration,
                recover_duration: self.recover_duration,
                timer: self
                    .timer
                    .checked_add(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                stage_section: self.stage_section,
            });

            // Consumes energy if there's enough left and charge has not stopped
            update.energy.change_by(
                -(self.energy_drain as f32 * data.dt.0) as i32,
                EnergySource::Ability,
            );
        } else if self.stage_section == StageSection::Charge {
            // Transitions to swing section of stage
            update.character = CharacterState::DashMelee(Data {
                base_damage: self.base_damage,
                max_damage: self.max_damage,
                base_knockback: self.base_knockback,
                max_knockback: self.max_knockback,
                range: self.range,
                angle: self.angle,
                energy_drain: self.energy_drain,
                forward_speed: self.forward_speed,
                buildup_duration: self.buildup_duration,
                charge_duration: self.charge_duration,
                charge_duration_attained: self.charge_duration_attained,
                infinite_charge: self.infinite_charge,
                swing_duration: self.swing_duration,
                recover_duration: self.recover_duration,
                timer: Duration::default(),
                stage_section: StageSection::Swing,
            })
        } else if self.stage_section == StageSection::Swing && self.timer < self.swing_duration {
            // Swings
            let charge_attained = if self.timer > self.charge_duration_attained {
                if self.timer > self.charge_duration_attained {
                    self.charge_duration
                } else {
                    self.timer
                }
            } else {
                self.charge_duration_attained
            };
            update.character = CharacterState::DashMelee(Data {
                base_damage: self.base_damage,
                max_damage: self.max_damage,
                base_knockback: self.base_knockback,
                max_knockback: self.max_knockback,
                range: self.range,
                angle: self.angle,
                energy_drain: self.energy_drain,
                forward_speed: self.forward_speed,
                buildup_duration: self.buildup_duration,
                charge_duration: self.charge_duration,
                charge_duration_attained: charge_attained,
                infinite_charge: self.infinite_charge,
                swing_duration: self.swing_duration,
                recover_duration: self.recover_duration,
                timer: self
                    .timer
                    .checked_add(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                stage_section: self.stage_section,
            })
        } else if self.stage_section == StageSection::Swing {
            // Hit attempt
            let charge_frac =
                (self.charge_duration_attained.as_secs_f32() / self.charge_duration.as_secs_f32()).min(1.0);
            let damage = (self.max_damage as f32 - self.base_damage as f32) * charge_frac
                + self.base_damage as f32;
            let knockback =
                (self.max_knockback - self.base_knockback) * charge_frac + self.base_knockback;
            data.updater.insert(data.entity, Attacking {
                base_healthchange: -damage as i32,
                range: self.range,
                max_angle: self.angle.to_radians(),
                applied: false,
                hit_count: 0,
                knockback,
            });

            // Transitions to recover section of stage
            update.character = CharacterState::DashMelee(Data {
                base_damage: self.base_damage,
                max_damage: self.max_damage,
                base_knockback: self.base_knockback,
                max_knockback: self.max_knockback,
                range: self.range,
                angle: self.angle,
                energy_drain: self.energy_drain,
                forward_speed: self.forward_speed,
                buildup_duration: self.buildup_duration,
                charge_duration: self.charge_duration,
                charge_duration_attained: self.charge_duration_attained,
                infinite_charge: self.infinite_charge,
                swing_duration: self.swing_duration,
                recover_duration: self.recover_duration,
                timer: Duration::default(),
                stage_section: StageSection::Recover,
            })
        } else if self.stage_section == StageSection::Recover && self.timer < self.recover_duration
        {
            // Recover
            update.character = CharacterState::DashMelee(Data {
                base_damage: self.base_damage,
                max_damage: self.max_damage,
                base_knockback: self.base_knockback,
                max_knockback: self.max_knockback,
                range: self.range,
                angle: self.angle,
                energy_drain: self.energy_drain,
                forward_speed: self.forward_speed,
                buildup_duration: self.buildup_duration,
                charge_duration: self.charge_duration,
                charge_duration_attained: self.charge_duration_attained,
                infinite_charge: self.infinite_charge,
                swing_duration: self.swing_duration,
                recover_duration: self.recover_duration,
                timer: self
                    .timer
                    .checked_add(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                stage_section: self.stage_section,
            })
        } else {
            // Done
            update.character = CharacterState::Wielding;
            // Make sure attack component is removed
            data.updater.remove::<Attacking>(data.entity);
        }

        update
    }
}
