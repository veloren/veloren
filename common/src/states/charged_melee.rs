use crate::{
    combat,
    comp::{
        CharacterState, MeleeConstructor, StateUpdate, character_state::OutputEvents,
        melee::CustomCombo,
    },
    event::LocalEvent,
    outcome::Outcome,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::{StageSection, *},
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
/// Separated out to condense update portions of character state
pub struct StaticData {
    /// How much energy is drained per second when charging
    pub energy_drain: f32,
    /// Energy cost per attack
    pub energy_cost: f32,
    /// The state can optionally have a buildup strike that applies after
    /// buildup before charging
    pub buildup_strike: Option<(Duration, MeleeConstructor)>,
    /// How long it takes to charge the weapon to max damage and knockback
    pub charge_duration: Duration,
    /// How long the weapon is swinging for
    pub swing_duration: Duration,
    /// At what fraction of the swing duration to apply the melee "hit"
    pub hit_timing: f32,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Used to construct the Melee attack
    pub melee_constructor: MeleeConstructor,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Used to specify the melee attack to the frontend
    pub specifier: Option<FrontendSpecifier>,
    /// The actual additional combo is modified by duration of charge
    pub custom_combo: CustomCombo,
    /// Adjusts move speed during the attack per stage
    pub movement_modifier: MovementModifier,
    /// Adjusts turning rate during the attack per stage
    pub ori_modifier: OrientationModifier,
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
    /// Whether the attack executed already
    pub exhausted: bool,
    /// How much the attack charged by
    pub charge_amount: f32,
    /// Adjusts move speed during the attack per stage
    pub movement_modifier: Option<f32>,
    /// Adjusts turning rate during the attack per stage
    pub ori_modifier: Option<f32>,
}

impl Data {
    /// How complete the charge is, on a scale of 0.0 to 1.0
    pub fn charge_frac(&self) -> f32 {
        if let StageSection::Charge = self.stage_section {
            (self.timer.as_secs_f32() / self.static_data.charge_duration.as_secs_f32()).min(1.0)
        } else {
            0.0
        }
    }
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, self.ori_modifier.unwrap_or(1.0), None);
        handle_move(data, &mut update, self.movement_modifier.unwrap_or(0.7));
        handle_jump(data, output_events, &mut update, 1.0);

        match self.stage_section {
            StageSection::Buildup => {
                if let Some((buildup, strike)) = self.static_data.buildup_strike {
                    if self.timer < buildup {
                        if let CharacterState::ChargedMelee(c) = &mut update.character {
                            c.timer = tick_attack_or_default(data, self.timer, None);
                        }
                    } else {
                        let precision_mult =
                            combat::compute_precision_mult(data.inventory, data.msm);
                        let tool_stats = get_tool_stats(data, self.static_data.ability_info);
                        data.updater
                            .insert(data.entity, strike.create_melee(precision_mult, tool_stats));

                        if let CharacterState::ChargedMelee(c) = &mut update.character {
                            c.stage_section = StageSection::Charge;
                            c.timer = Duration::default();
                        }
                    }
                } else if let CharacterState::ChargedMelee(c) = &mut update.character {
                    c.stage_section = StageSection::Charge;
                    c.timer = Duration::default();
                }
            },
            StageSection::Charge => {
                if input_is_pressed(data, self.static_data.ability_info.input)
                    && update.energy.current() >= self.static_data.energy_cost
                    && self.timer < self.static_data.charge_duration
                {
                    let charge = (self.timer.as_secs_f32()
                        / self.static_data.charge_duration.as_secs_f32())
                    .min(1.0);

                    // Charge the attack
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        charge_amount: charge,
                        ..*self
                    });

                    // Consumes energy if there's enough left and RMB is held down
                    update
                        .energy
                        .change_by(-self.static_data.energy_drain * data.dt.0);
                } else if input_is_pressed(data, self.static_data.ability_info.input)
                    && update.energy.current() >= self.static_data.energy_cost
                {
                    // Maintains charge
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });

                    // Consumes energy if there's enough left and RMB is held down
                    update
                        .energy
                        .change_by(-self.static_data.energy_drain * data.dt.0 / 5.0);
                } else {
                    // Transitions to swing
                    update.character = CharacterState::ChargedMelee(Data {
                        stage_section: StageSection::Action,
                        timer: Duration::default(),
                        movement_modifier: self.static_data.movement_modifier.swing,
                        ori_modifier: self.static_data.ori_modifier.swing,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                if self.timer.as_millis() as f32
                    > self.static_data.hit_timing
                        * self.static_data.swing_duration.as_millis() as f32
                    && !self.exhausted
                {
                    // Swing
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        exhausted: true,
                        ..*self
                    });

                    let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
                    let tool_stats = get_tool_stats(data, self.static_data.ability_info);
                    let custom_combo = CustomCombo {
                        base: self
                            .static_data
                            .custom_combo
                            .base
                            .map(|b| (self.charge_amount * b as f32).round() as i32),
                        conditional: self
                            .static_data
                            .custom_combo
                            .conditional
                            .map(|c| ((self.charge_amount * c.0 as f32).round() as i32, c.1)),
                    };

                    data.updater.insert(
                        data.entity,
                        self.static_data
                            .melee_constructor
                            .custom_combo(custom_combo)
                            .handle_scaling(self.charge_amount)
                            .create_melee(precision_mult, tool_stats),
                    );

                    if let Some(FrontendSpecifier::GroundCleave) = self.static_data.specifier {
                        // Send local event used for frontend shenanigans
                        output_events.emit_local(LocalEvent::CreateOutcome(Outcome::GroundSlam {
                            pos: data.pos.0
                                + *data.ori.look_dir()
                                    * (data.body.max_radius()
                                        + self.static_data.melee_constructor.range),
                        }));
                    }
                } else if self.timer < self.static_data.swing_duration {
                    // Swings
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to recover
                    update.character = CharacterState::ChargedMelee(Data {
                        stage_section: StageSection::Recover,
                        timer: Duration::default(),
                        movement_modifier: self.static_data.movement_modifier.recover,
                        ori_modifier: self.static_data.ori_modifier.recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovers
                    update.character = CharacterState::ChargedMelee(Data {
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

/// Used to specify a particular effect for frontend purposes
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrontendSpecifier {
    GroundCleave,
}
