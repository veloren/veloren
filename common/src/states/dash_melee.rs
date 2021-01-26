use crate::{
    combat::{Attack, AttackEffect, DamageComponent},
    comp::{
        CharacterState, EnergyChange, EnergySource, MeleeAttack, PoiseChange, PoiseSource,
        StateUpdate,
    },
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    Damage, DamageSource, GroupTarget, Knockback, KnockbackDir,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How much damage the attack initially does
    pub base_damage: u32,
    /// How much the attack scales in damage
    pub scaled_damage: u32,
    /// Initial poise damage
    pub base_poise_damage: u32,
    /// How much the attac scales in poise damage
    pub scaled_poise_damage: u32,
    /// How much the attack knocks the target back initially
    pub base_knockback: f32,
    /// How much the attack scales in knockback
    pub scaled_knockback: f32,
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
    /// What key is used to press ability
    pub ability_key: AbilityKey,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Whether the charge should end
    pub auto_charge: bool,
    /// Timer for each stage
    pub timer: Duration,
    /// Distance used to limit how often another attack will be applied
    pub refresh_distance: f32,
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
        if !ability_key_is_pressed(data, self.static_data.ability_key) {
            handle_interrupt(data, &mut update, self.static_data.is_interruptible);
            match update.character {
                CharacterState::DashMelee(_) => {},
                _ => {
                    return update;
                },
            }
        }

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::DashMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transitions to charge section of stage
                    update.character = CharacterState::DashMelee(Data {
                        auto_charge: !ability_key_is_pressed(data, self.static_data.ability_key),
                        timer: Duration::default(),
                        stage_section: StageSection::Charge,
                        ..*self
                    });
                }
            },
            StageSection::Charge => {
                if (self.static_data.infinite_charge
                    || self.timer < self.static_data.charge_duration)
                    && (ability_key_is_pressed(data, self.static_data.ability_key)
                        || (self.auto_charge && self.timer < self.static_data.charge_duration))
                    && update.energy.current() > 0
                {
                    // Forward movement
                    let charge_frac = (self.timer.as_secs_f32()
                        / self.static_data.charge_duration.as_secs_f32())
                    .min(1.0);

                    handle_forced_movement(
                        data,
                        &mut update,
                        ForcedMovement::Forward {
                            strength: self.static_data.forward_speed * charge_frac.sqrt(),
                        },
                        0.1,
                    );

                    // This logic basically just decides if a charge should end, and prevents the
                    // character state spamming attacks while checking if it has hit something
                    if !self.exhausted {
                        // Hit attempt (also checks if player is moving)
                        if update.vel.0.distance_squared(Vec3::zero()) > 1.0 {
                            let damage = Damage {
                                source: DamageSource::Melee,
                                value: self.static_data.base_damage as f32
                                    + charge_frac * self.static_data.scaled_damage as f32,
                            };
                            let poise_damage = PoiseChange {
                                amount: -(self.static_data.base_poise_damage as f32
                                    + charge_frac * self.static_data.scaled_poise_damage as f32)
                                    as i32,
                                source: PoiseSource::Attack,
                            };
                            let knockback = self.static_data.base_knockback
                                + charge_frac * self.static_data.scaled_knockback;
                            let knockback = AttackEffect::Knockback(Knockback {
                                strength: knockback,
                                direction: KnockbackDir::Away,
                            });
                            let damage =
                                DamageComponent::new(damage, Some(GroupTarget::OutOfGroup))
                                    .with_effect(knockback);
                            let attack = Attack::default().with_damage(damage);

                            data.updater.insert(data.entity, MeleeAttack {
                                attack,
                                range: self.static_data.range,
                                max_angle: self.static_data.angle.to_radians(),
                                applied: false,
                                hit_count: 0,
                            });
                        }
                        update.character = CharacterState::DashMelee(Data {
                            timer: self
                                .timer
                                .checked_add(Duration::from_secs_f32(data.dt.0))
                                .unwrap_or_default(),
                            exhausted: true,
                            ..*self
                        })
                    } else if self.refresh_distance < self.static_data.range * 0.5 {
                        update.character = CharacterState::DashMelee(Data {
                            timer: self
                                .timer
                                .checked_add(Duration::from_secs_f32(data.dt.0))
                                .unwrap_or_default(),
                            refresh_distance: self.refresh_distance
                                + data.dt.0 * data.vel.0.magnitude(),
                            ..*self
                        })
                    } else {
                        update.character = CharacterState::DashMelee(Data {
                            timer: self
                                .timer
                                .checked_add(Duration::from_secs_f32(data.dt.0))
                                .unwrap_or_default(),
                            refresh_distance: 0.0,
                            exhausted: false,
                            ..*self
                        })
                    }

                    // Consumes energy if there's enough left and charge has not stopped
                    update.energy.change_by(EnergyChange {
                        amount: -(self.static_data.energy_drain as f32 * data.dt.0) as i32,
                        source: EnergySource::Ability,
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::DashMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Swing,
                        ..*self
                    });
                }
            },
            StageSection::Swing => {
                if self.timer < self.static_data.swing_duration {
                    // Swings
                    update.character = CharacterState::DashMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
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
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                    // Make sure attack component is removed
                    data.updater.remove::<MeleeAttack>(data.entity);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
                // Make sure attack component is removed
                data.updater.remove::<MeleeAttack>(data.entity);
            },
        }

        update
    }
}
