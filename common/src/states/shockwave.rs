use crate::{
    combat::{
        self, Attack, AttackDamage, AttackEffect, CombatEffect, CombatRequirement, Damage,
        DamageKind, GroupTarget, Knockback,
    },
    comp::{
        CharacterState, StateUpdate, ability::Dodgeable, character_state::OutputEvents, shockwave,
    },
    event::{LocalEvent, ShockwaveEvent},
    outcome::Outcome,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state is swinging for
    pub swing_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Base damage
    pub damage: f32,
    /// Base poise damage
    pub poise_damage: f32,
    /// Knockback
    pub knockback: Knockback,
    /// Angle of the shockwave
    pub shockwave_angle: f32,
    /// Vertical angle of the shockwave
    pub shockwave_vertical_angle: f32,
    /// Speed of the shockwave
    pub shockwave_speed: f32,
    /// How long the shockwave travels for
    pub shockwave_duration: Duration,
    /// If the shockwave can be dodged, and in what way
    pub dodgeable: Dodgeable,
    /// Movement speed efficiency
    pub move_efficiency: f32,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Adds an effect onto the main damage of the attack
    pub damage_effect: Option<CombatEffect>,
    /// What kind of damage the attack does
    pub damage_kind: DamageKind,
    /// Used to specify the shockwave to the frontend
    pub specifier: shockwave::FrontendSpecifier,
    /// Controls outcome emission
    pub emit_outcome: bool,
    /// How fast enemy can rotate
    pub ori_rate: f32,
    /// Timing of shockwave
    pub timing: Timing,
    pub minimum_combo: Option<u32>,
    pub combo_on_use: u32,
    pub combo_consumption: ComboConsumption,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, self.static_data.ori_rate, None);
        handle_move(data, &mut update, self.static_data.move_efficiency);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    if let CharacterState::Shockwave(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Attack
                    if matches!(self.static_data.timing, Timing::PostBuildup) {
                        self.attack(data, output_events);
                    }

                    // Transitions to swing
                    if let CharacterState::Shockwave(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Action;
                    }
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.swing_duration {
                    // Swings
                    if let CharacterState::Shockwave(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                    // Send local event used for frontend shenanigans
                    if self.static_data.emit_outcome {
                        match self.static_data.specifier {
                            shockwave::FrontendSpecifier::IceSpikes => {
                                output_events.emit_local(LocalEvent::CreateOutcome(
                                    Outcome::FlashFreeze {
                                        pos: data.pos.0
                                            + *data.ori.look_dir() * (data.body.max_radius()),
                                    },
                                ));
                            },
                            shockwave::FrontendSpecifier::Ground => {
                                output_events.emit_local(LocalEvent::CreateOutcome(
                                    Outcome::GroundSlam {
                                        pos: data.pos.0
                                            + *data.ori.look_dir() * (data.body.max_radius()),
                                    },
                                ));
                            },
                            shockwave::FrontendSpecifier::Steam => {
                                output_events.emit_local(LocalEvent::CreateOutcome(
                                    Outcome::Steam {
                                        pos: data.pos.0
                                            + *data.ori.look_dir() * (data.body.max_radius()),
                                    },
                                ));
                            },
                            shockwave::FrontendSpecifier::Fire => {
                                output_events.emit_local(LocalEvent::CreateOutcome(
                                    Outcome::FireShockwave {
                                        pos: data.pos.0
                                            + *data.ori.look_dir() * (data.body.max_radius()),
                                    },
                                ));
                            },
                            shockwave::FrontendSpecifier::FireLow => {
                                output_events.emit_local(LocalEvent::CreateOutcome(
                                    Outcome::FireLowShockwave {
                                        pos: data.pos.0
                                            + *data.ori.look_dir() * (data.body.max_radius()),
                                    },
                                ));
                            },
                            _ => {
                                output_events.emit_local(LocalEvent::CreateOutcome(
                                    Outcome::Swoosh {
                                        pos: data.pos.0
                                            + *data.ori.look_dir() * (data.body.max_radius()),
                                    },
                                ));
                            },
                        }
                    }
                } else {
                    // Attack
                    if matches!(self.static_data.timing, Timing::PostAction) {
                        self.attack(data, output_events);
                    }

                    // Transitions to recover
                    if let CharacterState::Shockwave(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover;
                    }
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovers
                    if let CharacterState::Shockwave(c) = &mut update.character {
                        c.timer = tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        );
                    }
                } else {
                    // Done
                    end_ability(data, &mut update);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                end_ability(data, &mut update);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        handle_interrupts(data, &mut update, output_events);

        update
    }
}

impl Data {
    fn attack(&self, data: &JoinData, output_events: &mut OutputEvents) {
        if let Some(min_combo) = self.static_data.minimum_combo {
            self.static_data
                .combo_consumption
                .consume(data, output_events, min_combo);
        }

        let poise = AttackEffect::new(
            Some(GroupTarget::OutOfGroup),
            CombatEffect::Poise(self.static_data.poise_damage),
        )
        .with_requirement(CombatRequirement::AnyDamage);
        let knockback = AttackEffect::new(
            Some(GroupTarget::OutOfGroup),
            CombatEffect::Knockback(self.static_data.knockback),
        )
        .with_requirement(CombatRequirement::AnyDamage);
        let mut damage = AttackDamage::new(
            Damage {
                kind: self.static_data.damage_kind,
                value: self.static_data.damage,
            },
            Some(GroupTarget::OutOfGroup),
            rand::random(),
        );
        if let Some(effect) = &self.static_data.damage_effect {
            damage = damage.with_effect(effect.clone());
        }
        let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
        let attack = Attack::new(Some(self.static_data.ability_info))
            .with_damage(damage)
            .with_precision(
                precision_mult
                    * self
                        .static_data
                        .ability_info
                        .ability_meta
                        .precision_power_mult
                        .unwrap_or(1.0),
            )
            .with_effect(poise)
            .with_effect(knockback)
            .with_combo_increment();
        let properties = shockwave::Properties {
            angle: self.static_data.shockwave_angle,
            vertical_angle: self.static_data.shockwave_vertical_angle,
            speed: self.static_data.shockwave_speed,
            duration: self.static_data.shockwave_duration,
            attack,
            dodgeable: self.static_data.dodgeable,
            owner: Some(*data.uid),
            specifier: self.static_data.specifier,
        };
        output_events.emit_server(ShockwaveEvent {
            properties,
            pos: *data.pos,
            ori: *data.ori,
        });
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Timing {
    PostBuildup,
    PostAction,
}
