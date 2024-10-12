use crate::{
    combat,
    comp::{CharacterState, MeleeConstructor, StateUpdate, character_state::OutputEvents},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// Rate of energy drain
    pub energy_drain: f32,
    /// How quickly dasher moves forward
    pub forward_speed: f32,
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state charges for until it reaches max damage
    pub charge_duration: Duration,
    /// Duration of state spent in swing
    pub swing_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Used to construct the Melee attack
    pub melee_constructor: MeleeConstructor,
    /// How fast can you turn during charge
    pub ori_modifier: f32,
    /// Controls whether charge should always go until end or enemy hit
    pub auto_charge: bool,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Whether the charge should last a default amount of time or until the
    /// mouse is released
    pub auto_charge: bool,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.1);

        let create_melee = |charge_frac: f32| {
            let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
            let tool_stats = get_tool_stats(data, self.static_data.ability_info);
            self.static_data
                .melee_constructor
                .handle_scaling(charge_frac)
                .create_melee(precision_mult, tool_stats, data.stats)
        };

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    handle_orientation(data, &mut update, 1.0, None);
                    // Build up
                    update.character = CharacterState::DashMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to charge section of stage
                    update.character = CharacterState::DashMelee(Data {
                        auto_charge: !input_is_pressed(data, self.static_data.ability_info.input)
                            || self.static_data.auto_charge,
                        timer: Duration::default(),
                        stage_section: StageSection::Charge,
                        ..*self
                    });
                }
            },
            StageSection::Charge => {
                if self.timer < self.static_data.charge_duration
                    && (input_is_pressed(data, self.static_data.ability_info.input)
                        || self.auto_charge)
                    && update.energy.current() >= 0.0
                {
                    // Forward movement
                    let charge_frac = (self.timer.as_secs_f32()
                        / self.static_data.charge_duration.as_secs_f32())
                    .min(1.0);

                    handle_orientation(data, &mut update, self.static_data.ori_modifier, None);
                    handle_forced_movement(
                        data,
                        &mut update,
                        ForcedMovement::Forward(
                            self.static_data.forward_speed * charge_frac.sqrt(),
                        ),
                    );

                    // Determines if charge ends by continually refreshing melee component until it
                    // detects a hit, at which point the charge ends
                    if let Some(melee) = data.melee_attack {
                        if !melee.applied {
                            // If melee attack has not applied, just tick duration
                            update.character = CharacterState::DashMelee(Data {
                                timer: tick_attack_or_default(data, self.timer, None),
                                ..*self
                            });
                        } else if melee.hit_count == 0 {
                            // If melee attack has applied, but not hit anything, reset melee attack
                            data.updater.insert(data.entity, create_melee(charge_frac));
                            update.character = CharacterState::DashMelee(Data {
                                timer: tick_attack_or_default(data, self.timer, None),
                                ..*self
                            });
                        } else {
                            // Stop charging now and go to swing stage section
                            update.character = CharacterState::DashMelee(Data {
                                timer: Duration::default(),
                                stage_section: StageSection::Action,
                                ..*self
                            });
                        }
                    } else {
                        // If no melee attack, add it and tick duration
                        data.updater.insert(data.entity, create_melee(charge_frac));

                        update.character = CharacterState::DashMelee(Data {
                            timer: tick_attack_or_default(data, self.timer, None),
                            ..*self
                        });
                    }

                    // Consumes energy if there's enough left and charge has not stopped
                    update
                        .energy
                        .change_by(-self.static_data.energy_drain * data.dt.0);
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::DashMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.swing_duration {
                    // Swings
                    update.character = CharacterState::DashMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::DashMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recover
                    update.character = CharacterState::DashMelee(Data {
                        timer: tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        ),
                        ..*self
                    });
                } else {
                    // Done
                    end_melee_ability(data, &mut update);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                end_melee_ability(data, &mut update);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        handle_interrupts(data, &mut update, output_events);

        update
    }
}
