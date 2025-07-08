use crate::{
    Damage, DamageKind, DamageSource, Explosion, GroupTarget, Knockback, RadiusEffect,
    combat::{Attack, AttackDamage, AttackEffect, CombatEffect, CombatRequirement},
    comp::{
        CharacterState, StateUpdate, ability::Dodgeable, character_state::OutputEvents,
        item::Reagent,
    },
    event::ExplosionEvent,
    explosion::{ColorPreset, TerrainReplacementPreset},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state is casting the explosion for
    pub action_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Base damage
    pub damage: f32,
    /// Base poise damage
    pub poise: f32,
    /// Knockback
    pub knockback: Knockback,
    /// Range of the explosion
    pub radius: f32,
    /// Minimum falloff of explosion strength
    pub min_falloff: f32,
    /// If the explosion can be dodged, and in what way
    #[serde(default)]
    pub dodgeable: Dodgeable,
    /// Power and color of terrain destruction
    pub destroy_terrain: Option<(f32, ColorPreset)>,
    /// Range and kind of terrain replacement
    pub replace_terrain: Option<(f32, TerrainReplacementPreset)>,
    /// Whether the explosion is created at eye height instead of the entity's
    /// pos
    #[serde(default)]
    pub eye_height: bool,
    /// Controls visual effects
    pub reagent: Option<Reagent>,
    /// Adjusts move speed during the attack per stage
    #[serde(default)]
    pub movement_modifier: MovementModifier,
    /// Adjusts turning rate during the attack per stage
    #[serde(default)]
    pub ori_modifier: OrientationModifier,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Adjusts move speed during the attack
    pub movement_modifier: Option<f32>,
    /// How fast the entity should turn
    pub ori_modifier: Option<f32>,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, self.ori_modifier.unwrap_or(1.0), None);
        handle_move(data, &mut update, self.movement_modifier.unwrap_or(0.7));
        handle_jump(data, output_events, &mut update, 1.0);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::Explosion(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::Explosion(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        movement_modifier: self.static_data.movement_modifier.swing,
                        ori_modifier: self.static_data.ori_modifier.swing,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.action_duration {
                    // Swings
                    update.character = CharacterState::Explosion(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Create an explosion after action before transitioning to recover
                    let pos = if self.static_data.eye_height {
                        data.pos.0
                            + Vec3::unit_z() * data.body.eye_height(data.scale.map_or(1.0, |s| s.0))
                    } else {
                        data.pos.0
                    };

                    let mut effects = vec![RadiusEffect::Attack {
                        attack: Attack::default()
                            .with_damage(AttackDamage::new(
                                Damage {
                                    source: DamageSource::Explosion,
                                    kind: DamageKind::Crushing,
                                    value: self.static_data.damage,
                                },
                                Some(GroupTarget::OutOfGroup),
                                rand::random(),
                            ))
                            .with_effect(
                                AttackEffect::new(
                                    Some(GroupTarget::OutOfGroup),
                                    CombatEffect::Poise(self.static_data.poise),
                                )
                                .with_requirement(CombatRequirement::AnyDamage),
                            )
                            .with_effect(
                                AttackEffect::new(
                                    Some(GroupTarget::OutOfGroup),
                                    CombatEffect::Knockback(self.static_data.knockback),
                                )
                                .with_requirement(CombatRequirement::AnyDamage),
                            ),
                        dodgeable: self.static_data.dodgeable,
                    }];

                    if let Some((power, color)) = self.static_data.destroy_terrain {
                        effects.push(RadiusEffect::TerrainDestruction(power, color.to_rgb()));
                    }

                    if let Some((radius, replacement_preset)) = self.static_data.replace_terrain {
                        effects.push(RadiusEffect::ReplaceTerrain(radius, replacement_preset));
                    }

                    output_events.emit_server(ExplosionEvent {
                        pos,
                        explosion: Explosion {
                            effects,
                            radius: self.static_data.radius,
                            reagent: self.static_data.reagent,
                            min_falloff: self.static_data.min_falloff,
                        },
                        owner: Some(*data.uid),
                    });

                    // Transitions to recover section of stage
                    update.character = CharacterState::Explosion(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        movement_modifier: self.static_data.movement_modifier.recover,
                        ori_modifier: self.static_data.ori_modifier.recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovery
                    update.character = CharacterState::Explosion(Data {
                        timer: tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        ),
                        movement_modifier: self.static_data.movement_modifier.recover,
                        ori_modifier: self.static_data.ori_modifier.recover,
                        ..*self
                    });
                } else {
                    // Done
                    if input_is_pressed(data, self.static_data.ability_info.input) {
                        reset_state(self, data, output_events, &mut update);
                    } else {
                        end_ability(data, &mut update);
                    }
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

fn reset_state(
    data: &Data,
    join: &JoinData,
    output_events: &mut OutputEvents,
    update: &mut StateUpdate,
) {
    handle_input(
        join,
        output_events,
        update,
        data.static_data.ability_info.input,
    );
}
