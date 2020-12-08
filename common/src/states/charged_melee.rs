use crate::{
    comp::{Attacking, CharacterState, EnergyChange, EnergySource, StateUpdate},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::{StageSection, *},
    },
    Damage, DamageSource, GroupTarget, Knockback,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
/// Separated out to condense update portions of character state
pub struct StaticData {
    /// How much energy is drained per second when charging
    pub energy_drain: u32,
    /// Energy cost per attack
    pub energy_cost: u32,
    /// How much damage is dealt with no charge
    pub initial_damage: u32,
    /// How much the damage is scaled by
    pub scaled_damage: u32,
    /// How much knockback there is with no charge
    pub initial_knockback: f32,
    /// How much the knockback is scaled by
    pub scaled_knockback: f32,
    /// Max range
    pub range: f32,
    /// Max angle (45.0 will give you a 90.0 angle window)
    pub max_angle: f32,
    /// Speed stat of the weapon
    pub speed: f32,
    /// How long it takes to charge the weapon to max damage and knockback
    pub charge_duration: Duration,
    /// How long the weapon is swinging for
    pub swing_duration: Duration,
    /// At what fraction of the swing duration to apply the melee "hit"
    pub hit_timing: f32,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// What key is used to press ability
    pub ability_key: AbilityKey,
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

        handle_move(data, &mut update, 0.7);
        handle_jump(data, &mut update);
        if !ability_key_is_pressed(data, self.static_data.ability_key) {
            handle_interrupt(data, &mut update, false);
            match update.character {
                CharacterState::ChargedMelee(_) => {},
                _ => {
                    return update;
                },
            }
        }

        match self.stage_section {
            StageSection::Charge => {
                if ability_key_is_pressed(data, self.static_data.ability_key)
                    && update.energy.current() >= self.static_data.energy_cost
                    && self.timer < self.static_data.charge_duration
                {
                    let charge = (self.timer.as_secs_f32()
                        / self.static_data.charge_duration.as_secs_f32())
                    .min(1.0);

                    // Charge the attack
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(
                                data.dt.0 * self.static_data.speed,
                            ))
                            .unwrap_or_default(),
                        charge_amount: charge,
                        ..*self
                    });

                    // Consumes energy if there's enough left and RMB is held down
                    update.energy.change_by(EnergyChange {
                        amount: -(self.static_data.energy_drain as f32
                            * data.dt.0
                            * self.static_data.speed) as i32,
                        source: EnergySource::Ability,
                    });
                } else if ability_key_is_pressed(data, self.static_data.ability_key)
                    && update.energy.current() >= self.static_data.energy_cost
                {
                    // Maintains charge
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(
                                data.dt.0 * self.static_data.speed,
                            ))
                            .unwrap_or_default(),
                        ..*self
                    });

                    // Consumes energy if there's enough left and RMB is held down
                    update.energy.change_by(EnergyChange {
                        amount: -(self.static_data.energy_drain as f32
                            * data.dt.0
                            * self.static_data.speed
                            / 5.0) as i32,
                        source: EnergySource::Ability,
                    });
                } else {
                    // Transitions to swing
                    update.character = CharacterState::ChargedMelee(Data {
                        stage_section: StageSection::Swing,
                        timer: Duration::default(),
                        ..*self
                    });
                }
            },
            StageSection::Swing => {
                if self.timer.as_millis() as f32
                    > self.static_data.hit_timing
                        * self.static_data.swing_duration.as_millis() as f32
                    && !self.exhausted
                {
                    // Swing
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        exhausted: true,
                        ..*self
                    });
                    let damage = Damage {
                        source: DamageSource::Melee,
                        value: self.static_data.initial_damage as f32
                            + self.charge_amount * self.static_data.scaled_damage as f32,
                    };
                    let knockback = self.static_data.initial_knockback
                        + self.charge_amount * self.static_data.scaled_knockback;

                    // Hit attempt
                    data.updater.insert(data.entity, Attacking {
                        damages: vec![(Some(GroupTarget::OutOfGroup), damage)],
                        range: self.static_data.range,
                        max_angle: self.static_data.max_angle.to_radians(),
                        applied: false,
                        hit_count: 0,
                        knockback: Knockback::Away(knockback),
                    });
                } else if self.timer < self.static_data.swing_duration {
                    // Swings
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transitions to recover
                    update.character = CharacterState::ChargedMelee(Data {
                        stage_section: StageSection::Recover,
                        timer: Duration::default(),
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovers
                    update.character = CharacterState::ChargedMelee(Data {
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
