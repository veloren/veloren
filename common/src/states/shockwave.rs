use crate::{
    combat::{
        Attack, AttackEffect, CombatRequirement, Damage, DamageComponent, DamageSource,
        EffectComponent, GroupTarget, Knockback,
    },
    comp::{shockwave, CharacterState, StateUpdate},
    event::ServerEvent,
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
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state is swinging for
    pub swing_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Base damage
    pub damage: u32,
    /// Base poise damage
    pub poise_damage: u32,
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
    /// Whether the shockwave requires the target to be on the ground
    pub requires_ground: bool,
    /// Movement speed efficiency
    pub move_efficiency: f32,
    /// What key is used to press ability
    pub ability_key: AbilityKey,
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
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, self.static_data.move_efficiency);
        if !ability_key_is_pressed(data, self.static_data.ability_key) {
            handle_interrupt(data, &mut update, false);
            match update.character {
                CharacterState::Shockwave(_) => {},
                _ => {
                    return update;
                },
            }
        }

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::Shockwave(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Attack
                    let poise = AttackEffect::Poise(self.static_data.poise_damage as f32);
                    let poise = EffectComponent::new(Some(GroupTarget::OutOfGroup), poise)
                        .with_requirement(CombatRequirement::AnyDamage);
                    let knockback = AttackEffect::Knockback(self.static_data.knockback);
                    let knockback = EffectComponent::new(Some(GroupTarget::OutOfGroup), knockback)
                        .with_requirement(CombatRequirement::AnyDamage);
                    let damage = Damage {
                        source: DamageSource::Shockwave,
                        value: self.static_data.damage as f32,
                    };
                    let damage = DamageComponent::new(damage, Some(GroupTarget::OutOfGroup));
                    let attack = Attack::default()
                        .with_damage(damage)
                        .with_effect(poise)
                        .with_effect(knockback);
                    let properties = shockwave::Properties {
                        angle: self.static_data.shockwave_angle,
                        vertical_angle: self.static_data.shockwave_vertical_angle,
                        speed: self.static_data.shockwave_speed,
                        duration: self.static_data.shockwave_duration,
                        attack,
                        requires_ground: self.static_data.requires_ground,
                        owner: Some(*data.uid),
                    };
                    update.server_events.push_front(ServerEvent::Shockwave {
                        properties,
                        pos: *data.pos,
                        ori: *data.ori,
                    });

                    // Transitions to swing
                    update.character = CharacterState::Shockwave(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Swing,
                        ..*self
                    });
                }
            },
            StageSection::Swing => {
                if self.timer < self.static_data.swing_duration {
                    // Swings
                    update.character = CharacterState::Shockwave(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transitions to recover
                    update.character = CharacterState::Shockwave(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.swing_duration {
                    // Recovers
                    update.character = CharacterState::Shockwave(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
            },
        }

        update
    }
}
