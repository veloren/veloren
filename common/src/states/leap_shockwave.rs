use crate::{
    combat::{
        self, Attack, AttackDamage, AttackEffect, CombatEffect, CombatRequirement, Damage,
        DamageKind, DamageSource, GroupTarget, Knockback,
    },
    comp::{
        character_state::OutputEvents,
        item::Reagent,
        shockwave::{self, ShockwaveDodgeable},
        CharacterState, StateUpdate,
    },
    event::{ExplosionEvent, LocalEvent, ShockwaveEvent},
    outcome::Outcome,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::{StageSection, *},
    },
    Explosion, KnockbackDir, RadiusEffect,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long the state is moving
    pub movement_duration: Duration,
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
    pub dodgeable: ShockwaveDodgeable,
    /// Movement speed efficiency
    pub move_efficiency: f32,
    /// Adds an effect onto the main damage of the attack
    pub damage_effect: Option<CombatEffect>,
    /// What kind of damage the attack does
    pub damage_kind: DamageKind,
    /// Used to specify the shockwave to the frontend
    pub specifier: shockwave::FrontendSpecifier,
    /// Affects how far forward the player leaps
    pub forward_leap_strength: f32,
    /// Affects how high the player leaps
    pub vertical_leap_strength: f32,
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
    /// Whether the attack can deal more damage
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);

        match self.stage_section {
            // Delay before leaping into the air
            StageSection::Buildup => {
                handle_move(data, &mut update, 0.3);
                handle_jump(data, output_events, &mut update, 1.0);
                // Wait for `buildup_duration` to expire
                if self.timer < self.static_data.buildup_duration {
                    update.character = CharacterState::LeapShockwave(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to leap portion of state after buildup delay
                    update.character = CharacterState::LeapShockwave(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Movement,
                        ..*self
                    });
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
                    update.character = CharacterState::LeapShockwave(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else if data.physics.on_ground.is_some() | data.physics.in_liquid().is_some() {
                    // Transitions to swing portion of state upon hitting ground
                    update.character = CharacterState::LeapShockwave(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                handle_move(data, &mut update, 0.3);
                handle_jump(data, output_events, &mut update, 1.0);
                if self.timer < self.static_data.swing_duration {
                    // Swings
                    update.character = CharacterState::LeapShockwave(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });

                    // Attack
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
                            source: DamageSource::Shockwave,
                            kind: self.static_data.damage_kind,
                            value: self.static_data.damage,
                        },
                        Some(GroupTarget::OutOfGroup),
                        rand::random(),
                    );
                    if let Some(effect) = self.static_data.damage_effect {
                        damage = damage.with_effect(effect);
                    }
                    let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
                    let attack = Attack::default()
                        .with_damage(damage)
                        .with_precision(precision_mult)
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
                    // Send local event used for frontend shenanigans
                    match self.static_data.specifier {
                        shockwave::FrontendSpecifier::IceSpikes => {
                            let damage = AttackDamage::new(
                                Damage {
                                    source: DamageSource::Explosion,
                                    kind: self.static_data.damage_kind,
                                    value: self.static_data.damage / 2.,
                                },
                                Some(GroupTarget::OutOfGroup),
                                rand::random(),
                            );
                            let attack = Attack::default().with_damage(damage).with_effect(
                                AttackEffect::new(
                                    Some(GroupTarget::OutOfGroup),
                                    CombatEffect::Knockback(Knockback {
                                        direction: KnockbackDir::Away,
                                        strength: 10.,
                                    }),
                                ),
                            );
                            let explosion = Explosion {
                                effects: vec![RadiusEffect::Attack(attack)],
                                radius: data.body.max_radius() * 3.0,
                                reagent: Some(Reagent::White),
                                min_falloff: 0.5,
                            };
                            output_events.emit_server(ExplosionEvent {
                                pos: data.pos.0,
                                explosion,
                                owner: Some(*data.uid),
                            });
                            output_events.emit_local(LocalEvent::CreateOutcome(
                                Outcome::IceSpikes {
                                    pos: data.pos.0
                                        + *data.ori.look_dir() * (data.body.max_radius()),
                                },
                            ));
                        },
                        shockwave::FrontendSpecifier::Steam => {
                            output_events.emit_local(LocalEvent::CreateOutcome(Outcome::Steam {
                                pos: data.pos.0 + *data.ori.look_dir() * (data.body.max_radius()),
                            }));
                        },
                        _ => {},
                    };
                } else {
                    // Transitions to recover
                    update.character = CharacterState::LeapShockwave(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                handle_move(data, &mut update, 0.3);
                handle_jump(data, output_events, &mut update, 1.0);
                if self.timer < self.static_data.recover_duration {
                    // Recovers
                    update.character = CharacterState::LeapShockwave(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
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
