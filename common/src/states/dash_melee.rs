use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::{StageSection, *},
    sys::character_behavior::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
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
    /// Whether state keeps charging after reaching max charge duration
    pub infinite_charge: bool,
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state charges for until it reaches max damage
    pub charge_duration: Duration,
    /// Suration of state spent in swing
    pub swing_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Whether the state can be interrupted by other abilities
    pub is_interruptible: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Whether the charge should end
    pub end_charge: bool,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Whether the state should attempt attacking again
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0);
        handle_move(data, &mut update, 0.1);

        // Allows for other states to interrupt this state
        if self.static_data.is_interruptible && !data.inputs.secondary.is_pressed() {
            handle_dodge_input(data, &mut update);
            handle_ability1_input(data, &mut update);
            match update.character {
                CharacterState::DashMelee(_) => {},
                _ => {
                    return update;
                },
            }
        }

        if self.stage_section == StageSection::Buildup
            && self.timer < self.static_data.buildup_duration
        {
            // Build up
            update.character = CharacterState::DashMelee(Data {
                static_data: self.static_data,
                end_charge: self.end_charge,
                timer: self
                    .timer
                    .checked_add(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                stage_section: self.stage_section,
                exhausted: self.exhausted,
            })
        } else if self.stage_section == StageSection::Buildup {
            // Transitions to charge section of stage
            update.character = CharacterState::DashMelee(Data {
                static_data: self.static_data,
                end_charge: self.end_charge,
                timer: Duration::default(),
                stage_section: StageSection::Charge,
                exhausted: self.exhausted,
            })
        } else if self.stage_section == StageSection::Charge
            && (self.timer < self.static_data.charge_duration
                || (self.static_data.infinite_charge && data.inputs.secondary.is_pressed()))
            && update.energy.current() > 0
            && !self.end_charge
        {
            // Forward movement
            forward_move(data, &mut update, 0.1, self.static_data.forward_speed);

            // Hit attempt (also checks if player is moving)
            if !self.exhausted && update.vel.0.distance_squared(Vec3::zero()) > 1.0 {
                let charge_frac = (self.timer.as_secs_f32()
                    / self.static_data.charge_duration.as_secs_f32())
                .min(1.0);
                let damage = (self.static_data.max_damage as f32
                    - self.static_data.base_damage as f32)
                    * charge_frac
                    + self.static_data.base_damage as f32;
                let knockback = (self.static_data.max_knockback - self.static_data.base_knockback)
                    * charge_frac
                    + self.static_data.base_knockback;
                data.updater.insert(data.entity, Attacking {
                    base_healthchange: -damage as i32,
                    range: self.static_data.range,
                    max_angle: self.static_data.angle.to_radians(),
                    applied: false,
                    hit_count: 0,
                    knockback,
                });
            }

            // This logic basically just decides if a charge should end, and prevents the
            // character state spamming attacks while checking if it has hit something
            if !self.exhausted {
                update.character = CharacterState::DashMelee(Data {
                    static_data: self.static_data,
                    end_charge: self.end_charge,
                    timer: self
                        .timer
                        .checked_add(Duration::from_secs_f32(data.dt.0))
                        .unwrap_or_default(),
                    stage_section: StageSection::Charge,
                    exhausted: true,
                })
            } else if let Some(attack) = data.attacking {
                if attack.applied && attack.hit_count > 0 {
                    update.character = CharacterState::DashMelee(Data {
                        static_data: self.static_data,
                        end_charge: !self.static_data.infinite_charge,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: StageSection::Charge,
                        exhausted: false,
                    })
                } else if attack.applied {
                    update.character = CharacterState::DashMelee(Data {
                        static_data: self.static_data,
                        end_charge: self.end_charge,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: StageSection::Charge,
                        exhausted: false,
                    })
                } else {
                    update.character = CharacterState::DashMelee(Data {
                        static_data: self.static_data,
                        end_charge: !self.static_data.infinite_charge,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: StageSection::Charge,
                        exhausted: self.exhausted,
                    })
                }
            } else {
                update.character = CharacterState::DashMelee(Data {
                    static_data: self.static_data,
                    end_charge: !self.static_data.infinite_charge,
                    timer: self
                        .timer
                        .checked_add(Duration::from_secs_f32(data.dt.0))
                        .unwrap_or_default(),
                    stage_section: StageSection::Charge,
                    exhausted: self.exhausted,
                })
            }

            // Consumes energy if there's enough left and charge has not stopped
            update.energy.change_by(
                -(self.static_data.energy_drain as f32 * data.dt.0) as i32,
                EnergySource::Ability,
            );
        } else if self.stage_section == StageSection::Charge {
            // Transitions to swing section of stage
            update.character = CharacterState::DashMelee(Data {
                static_data: self.static_data,
                end_charge: self.end_charge,
                timer: Duration::default(),
                stage_section: StageSection::Swing,
                exhausted: self.exhausted,
            })
        } else if self.stage_section == StageSection::Swing
            && self.timer < self.static_data.swing_duration
        {
            // Swings
            update.character = CharacterState::DashMelee(Data {
                static_data: self.static_data,
                end_charge: self.end_charge,
                timer: self
                    .timer
                    .checked_add(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                stage_section: self.stage_section,
                exhausted: self.exhausted,
            })
        } else if self.stage_section == StageSection::Swing {
            // Transitions to recover section of stage
            update.character = CharacterState::DashMelee(Data {
                static_data: self.static_data,
                end_charge: self.end_charge,
                timer: Duration::default(),
                stage_section: StageSection::Recover,
                exhausted: self.exhausted,
            })
        } else if self.stage_section == StageSection::Recover
            && self.timer < self.static_data.recover_duration
        {
            // Recover
            update.character = CharacterState::DashMelee(Data {
                static_data: self.static_data,
                end_charge: self.end_charge,
                timer: self
                    .timer
                    .checked_add(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                stage_section: self.stage_section,
                exhausted: self.exhausted,
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
