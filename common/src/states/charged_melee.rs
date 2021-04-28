use crate::{
    combat::{Attack, AttackDamage, AttackEffect, CombatBuff, CombatEffect, CombatRequirement},
    comp::{tool::ToolKind, CharacterState, EnergyChange, EnergySource, Melee, StateUpdate},
    event::LocalEvent,
    outcome::Outcome,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::{StageSection, *},
    },
    Damage, DamageSource, GroupTarget, Knockback, KnockbackDir,
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
    /// How much damage is dealt with no charge
    pub initial_damage: f32,
    /// How much the damage is scaled by
    pub scaled_damage: f32,
    /// How much poise damage is dealt with no charge
    pub initial_poise_damage: f32,
    /// How much poise damage is scaled by
    pub scaled_poise_damage: f32,
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
    pub ability_info: AbilityInfo,
    /// Used to specify the melee attack to the frontend
    pub specifier: Option<FrontendSpecifier>,
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

        handle_orientation(data, &mut update, 1.0);
        handle_move(data, &mut update, 0.7);
        handle_jump(data, &mut update, 1.0);

        match self.stage_section {
            StageSection::Charge => {
                if input_is_pressed(data, self.static_data.ability_info.input)
                    && update.energy.current() as f32 >= self.static_data.energy_cost
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
                } else if input_is_pressed(data, self.static_data.ability_info.input)
                    && update.energy.current() as f32 >= self.static_data.energy_cost
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
                    let poise = AttackEffect::new(
                        Some(GroupTarget::OutOfGroup),
                        CombatEffect::Poise(
                            self.static_data.initial_poise_damage as f32
                                + self.charge_amount * self.static_data.scaled_poise_damage as f32,
                        ),
                    )
                    .with_requirement(CombatRequirement::AnyDamage);
                    let knockback = AttackEffect::new(
                        Some(GroupTarget::OutOfGroup),
                        CombatEffect::Knockback(Knockback {
                            strength: self.static_data.initial_knockback
                                + self.charge_amount * self.static_data.scaled_knockback,
                            direction: KnockbackDir::Away,
                        }),
                    )
                    .with_requirement(CombatRequirement::AnyDamage);
                    let buff = CombatEffect::Buff(CombatBuff::default_physical());
                    let damage = AttackDamage::new(
                        Damage {
                            source: DamageSource::Melee,
                            value: self.static_data.initial_damage as f32
                                + self.charge_amount * self.static_data.scaled_damage as f32,
                        },
                        Some(GroupTarget::OutOfGroup),
                    )
                    .with_effect(buff);
                    let (crit_chance, crit_mult) =
                        get_crit_data(data, self.static_data.ability_info);
                    let attack = Attack::default()
                        .with_damage(damage)
                        .with_crit(crit_chance, crit_mult)
                        .with_effect(poise)
                        .with_effect(knockback)
                        .with_combo_increment();

                    // Hit attempt
                    data.updater.insert(data.entity, Melee {
                        attack,
                        range: self.static_data.range,
                        max_angle: self.static_data.max_angle.to_radians(),
                        applied: false,
                        hit_count: 0,
                        break_block: data
                            .inputs
                            .select_pos
                            .map(|p| {
                                (
                                    p.map(|e| e.floor() as i32),
                                    self.static_data.ability_info.tool,
                                )
                            })
                            .filter(|(_, tool)| tool == &Some(ToolKind::Pick)),
                    });

                    if let Some(FrontendSpecifier::GroundCleave) = self.static_data.specifier {
                        // Send local event used for frontend shenanigans
                        update
                            .local_events
                            .push_front(LocalEvent::CreateOutcome(Outcome::Bonk {
                                pos: data.pos.0
                                    + *data.ori.look_dir()
                                        * (data.body.radius() + self.static_data.range),
                            }));
                    }
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
                    data.updater.remove::<Melee>(data.entity);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
                // Make sure attack component is removed
                data.updater.remove::<Melee>(data.entity);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        if !input_is_pressed(data, self.static_data.ability_info.input) {
            handle_state_interrupt(data, &mut update, false);
        }

        update
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum FrontendSpecifier {
    GroundCleave,
}
