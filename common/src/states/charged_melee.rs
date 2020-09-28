use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::{StageSection, *},
    sys::character_behavior::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
/// Separated out to condense update portions of character state
pub struct StaticData {
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
    /// Max range
    pub range: f32,
    /// Max angle (45.0 will give you a 90.0 angle window)
    pub max_angle: f32,
    /// How long it takes to charge the weapon to max damage and knockback
    pub charge_duration: Duration,
    /// How long the weapon is swinging for
    pub swing_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Checks what section a stage is in
    pub stage_section: StageSection,
    /// Timer for each stage
    pub timer: Duration,
    /// Whether the attack fired already
    pub exhausted: bool,
    /// How much the attack charged by
    pub charge_amount: f32,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.3);
        handle_jump(data, &mut update);

        match self.stage_section {
            StageSection::Charge => {
                if data.inputs.secondary.is_pressed()
                    && self.timer < self.static_data.charge_duration
                    && update.energy.current() > 0
                {
                    let charge = (self.timer.as_secs_f32()
                        / self.static_data.charge_duration.as_secs_f32())
                    .min(1.0);

                    // Charge the attack
                    update.character = CharacterState::ChargedMelee(Data {
                        static_data: self.static_data,
                        stage_section: self.stage_section,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        exhausted: self.exhausted,
                        charge_amount: charge,
                    });

                    // Consumes energy if there's enough left and RMB is held down
                    update.energy.change_by(
                        -(self.static_data.energy_drain as f32 * data.dt.0) as i32,
                        EnergySource::Ability,
                    );
                } else if data.inputs.secondary.is_pressed() {
                    // Maintains charge
                    update.character = CharacterState::ChargedMelee(Data {
                        static_data: self.static_data,
                        stage_section: self.stage_section,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        exhausted: self.exhausted,
                        charge_amount: self.charge_amount,
                    });

                    // Consumes energy if there's enough left and RMB is held down
                    update.energy.change_by(
                        -(self.static_data.energy_drain as f32 * data.dt.0 / 5.0) as i32,
                        EnergySource::Ability,
                    );
                } else {
                    // Transitions to swing
                    update.character = CharacterState::ChargedMelee(Data {
                        static_data: self.static_data,
                        stage_section: StageSection::Swing,
                        timer: Duration::default(),
                        exhausted: self.exhausted,
                        charge_amount: self.charge_amount,
                    });
                }
            },
            StageSection::Swing => {
                if !self.exhausted {
                    let damage = self.static_data.initial_damage
                        + ((self.static_data.max_damage - self.static_data.initial_damage) as f32
                            * self.charge_amount) as u32;
                    let knockback = self.static_data.initial_knockback
                        + (self.static_data.max_knockback - self.static_data.initial_knockback)
                            * self.charge_amount;

                    // Hit attempt
                    data.updater.insert(data.entity, Attacking {
                        base_damage: damage as u32,
                        base_heal: 0,
                        range: self.static_data.range,
                        max_angle: self.static_data.max_angle.to_radians(),
                        applied: false,
                        hit_count: 0,
                        knockback,
                    });

                    // Starts swinging
                    update.character = CharacterState::ChargedMelee(Data {
                        static_data: self.static_data,
                        stage_section: self.stage_section,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        exhausted: true,
                        charge_amount: self.charge_amount,
                    });
                } else if self.timer < self.static_data.swing_duration {
                    // Swings
                    update.character = CharacterState::ChargedMelee(Data {
                        static_data: self.static_data,
                        stage_section: self.stage_section,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        exhausted: self.exhausted,
                        charge_amount: self.charge_amount,
                    });
                } else {
                    // Transitions to recover
                    update.character = CharacterState::ChargedMelee(Data {
                        static_data: self.static_data,
                        stage_section: StageSection::Recover,
                        timer: Duration::default(),
                        exhausted: self.exhausted,
                        charge_amount: self.charge_amount,
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovers
                    update.character = CharacterState::ChargedMelee(Data {
                        static_data: self.static_data,
                        stage_section: self.stage_section,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        exhausted: self.exhausted,
                        charge_amount: self.charge_amount,
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                    // Make sure attack component is removed
                    data.updater.remove::<Attacking>(data.entity);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
                // Make sure attack component is removed
                data.updater.remove::<Attacking>(data.entity);
            },
        }

        update
    }
}
