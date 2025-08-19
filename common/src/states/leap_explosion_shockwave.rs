use crate::{
    Explosion, RadiusEffect,
    combat::{
        self, Attack, AttackDamage, AttackEffect, CombatEffect, CombatRequirement, Damage,
        DamageKind, GroupTarget, Knockback,
    },
    comp::{
        CharacterState, StateUpdate, ability::Dodgeable, character_state::OutputEvents,
        item::Reagent, shockwave,
    },
    event::{ExplosionEvent, ShockwaveEvent},
    explosion::{ColorPreset, TerrainReplacementPreset},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::{StageSection, *},
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

/// Separated out to condense update portions of character state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long the state is moving
    pub movement_duration: Duration,
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state is swinging for
    pub swing_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Affects how far forward the player leaps
    pub forward_leap_strength: f32,
    /// Affects how high the player leaps
    pub vertical_leap_strength: f32,

    /// Base explosion damage
    pub explosion_damage: f32,
    /// Base explosion poise damage
    pub explosion_poise: f32,
    /// Explosion knockback
    pub explosion_knockback: Knockback,
    /// Range of the explosion
    pub explosion_radius: f32,
    /// Minimum falloff of explosion strength
    pub min_falloff: f32,
    /// If the explosion can be dodged, and in what way
    #[serde(default)]
    pub explosion_dodgeable: Dodgeable,
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

    /// Base shockwave damage
    pub shockwave_damage: f32,
    /// Base shockwave poise damage
    pub shockwave_poise: f32,
    /// Shockwave knockback
    pub shockwave_knockback: Knockback,
    /// Angle of the shockwave
    pub shockwave_angle: f32,
    /// Vertical angle of the shockwave
    pub shockwave_vertical_angle: f32,
    /// Speed of the shockwave
    pub shockwave_speed: f32,
    /// How long the shockwave travels for
    pub shockwave_duration: Duration,
    /// If the shockwave can be dodged, and in what way
    pub shockwave_dodgeable: Dodgeable,
    /// Adds an effect onto the main damage of the shockwave
    pub shockwave_damage_effect: Option<CombatEffect>,
    /// What kind of damage the shockwave does
    pub shockwave_damage_kind: DamageKind,
    /// Used to specify the shockwave to the frontend
    pub shockwave_specifier: shockwave::FrontendSpecifier,

    /// Movement speed efficiency
    pub move_efficiency: f32,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
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
    /// Whether the attack can deal more damage
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, 0.3);
        handle_jump(data, output_events, &mut update, 1.0);

        match self.stage_section {
            // Delay before leaping into the air
            StageSection::Buildup => {
                // Wait for `buildup_duration` to expire
                if self.timer < self.static_data.buildup_duration {
                    if let CharacterState::LeapExplosionShockwave(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Transitions to leap portion of state after buildup delay
                    if let CharacterState::LeapExplosionShockwave(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Movement;
                    }
                }
            },
            StageSection::Movement => {
                if self.timer < self.static_data.movement_duration {
                    // Apply jumping force
                    let progress = 1.0
                        - self.timer.as_secs_f32()
                            / self.static_data.movement_duration.as_secs_f32();
                    handle_forced_movement(data, &mut update, ForcedMovement::Leap {
                        vertical: self.static_data.vertical_leap_strength,
                        forward: self.static_data.forward_leap_strength,
                        progress,
                        direction: MovementDirection::Look,
                    });

                    // Increment duration
                    // If we were to set a timeout for state, this would be
                    // outside if block and have else check for > movement
                    // duration * some multiplier
                    if let CharacterState::LeapExplosionShockwave(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else if data.physics.on_ground.is_some() | data.physics.in_liquid().is_some() {
                    // Transitions to swing portion of state upon hitting ground
                    if let CharacterState::LeapExplosionShockwave(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Action;
                    }
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.swing_duration {
                    // Swings
                    if let CharacterState::LeapExplosionShockwave(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Explosion
                    let explosion_pos = if self.static_data.eye_height {
                        data.pos.0
                            + Vec3::unit_z() * data.body.eye_height(data.scale.map_or(1.0, |s| s.0))
                    } else {
                        data.pos.0
                    };

                    let mut effects = vec![RadiusEffect::Attack {
                        attack: Attack::new(Some(self.static_data.ability_info))
                            .with_damage(AttackDamage::new(
                                Damage {
                                    kind: DamageKind::Crushing,
                                    value: self.static_data.explosion_damage,
                                },
                                Some(GroupTarget::OutOfGroup),
                                rand::random(),
                            ))
                            .with_effect(
                                AttackEffect::new(
                                    Some(GroupTarget::OutOfGroup),
                                    CombatEffect::Poise(self.static_data.explosion_poise),
                                )
                                .with_requirement(CombatRequirement::AnyDamage),
                            )
                            .with_effect(
                                AttackEffect::new(
                                    Some(GroupTarget::OutOfGroup),
                                    CombatEffect::Knockback(self.static_data.explosion_knockback),
                                )
                                .with_requirement(CombatRequirement::AnyDamage),
                            ),
                        dodgeable: self.static_data.explosion_dodgeable,
                    }];

                    if let Some((power, color)) = self.static_data.destroy_terrain {
                        effects.push(RadiusEffect::TerrainDestruction(power, color.to_rgb()));
                    }

                    if let Some((radius, replacement_preset)) = self.static_data.replace_terrain {
                        effects.push(RadiusEffect::ReplaceTerrain(radius, replacement_preset));
                    }

                    output_events.emit_server(ExplosionEvent {
                        pos: explosion_pos,
                        explosion: Explosion {
                            effects,
                            radius: self.static_data.explosion_radius,
                            reagent: self.static_data.reagent,
                            min_falloff: self.static_data.min_falloff,
                        },
                        owner: Some(*data.uid),
                    });

                    // Shockwave
                    let shockwave_poise = AttackEffect::new(
                        Some(GroupTarget::OutOfGroup),
                        CombatEffect::Poise(self.static_data.shockwave_poise),
                    )
                    .with_requirement(CombatRequirement::AnyDamage);
                    let shockwave_knockback = AttackEffect::new(
                        Some(GroupTarget::OutOfGroup),
                        CombatEffect::Knockback(self.static_data.shockwave_knockback),
                    )
                    .with_requirement(CombatRequirement::AnyDamage);
                    let mut shockwave_damage = AttackDamage::new(
                        Damage {
                            kind: self.static_data.shockwave_damage_kind,
                            value: self.static_data.shockwave_damage,
                        },
                        Some(GroupTarget::OutOfGroup),
                        rand::random(),
                    );
                    if let Some(effect) = &self.static_data.shockwave_damage_effect {
                        shockwave_damage = shockwave_damage.with_effect(effect.clone());
                    }
                    let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
                    let shockwave_attack = Attack::new(Some(self.static_data.ability_info))
                        .with_damage(shockwave_damage)
                        .with_precision(precision_mult)
                        .with_effect(shockwave_poise)
                        .with_effect(shockwave_knockback)
                        .with_combo_increment();
                    let properties = shockwave::Properties {
                        angle: self.static_data.shockwave_angle,
                        vertical_angle: self.static_data.shockwave_vertical_angle,
                        speed: self.static_data.shockwave_speed,
                        duration: self.static_data.shockwave_duration,
                        attack: shockwave_attack,
                        dodgeable: self.static_data.shockwave_dodgeable,
                        owner: Some(*data.uid),
                        specifier: self.static_data.shockwave_specifier,
                    };
                    output_events.emit_server(ShockwaveEvent {
                        properties,
                        pos: *data.pos,
                        ori: *data.ori,
                    });

                    // Transitions to recover
                    if let CharacterState::LeapExplosionShockwave(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover;
                    }
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovers
                    if let CharacterState::LeapExplosionShockwave(c) = &mut update.character {
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
