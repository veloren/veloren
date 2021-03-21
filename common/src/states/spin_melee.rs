use crate::{
    combat::{
        Attack, AttackDamage, AttackEffect, CombatBuff, CombatEffect, CombatRequirement, Damage,
        DamageSource, GroupTarget, Knockback,
    },
    comp::{tool::ToolKind, CharacterState, EnergyChange, EnergySource, Melee, StateUpdate},
    consts::GRAVITY,
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
    /// How long until the state attacks
    pub buildup_duration: Duration,
    /// How long the state is in the swing duration
    pub swing_duration: Duration,
    /// How long until state ends
    pub recover_duration: Duration,
    /// Base damage
    pub base_damage: f32,
    /// Base poise damage
    pub base_poise_damage: f32,
    /// Knockback
    pub knockback: Knockback,
    /// Range
    pub range: f32,
    /// Adds an effect onto the main damage of the attack
    pub damage_effect: Option<CombatEffect>,
    /// Energy cost per attack
    pub energy_cost: f32,
    /// Whether spin state is infinite
    pub is_infinite: bool,
    /// Used to dictate how movement functions in this state
    pub movement_behavior: MovementBehavior,
    /// Whether the state can be interrupted by other abilities
    pub is_interruptible: bool,
    /// Used for forced forward movement
    pub forward_speed: f32,
    /// Number of spins
    pub num_spins: u32,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Used to specify the melee attack to the frontend
    pub specifier: Option<FrontendSpecifier>,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// How many spins it can do before ending
    pub spins_remaining: u32,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Whether the state can deal damage
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        match self.static_data.movement_behavior {
            MovementBehavior::ForwardGround | MovementBehavior::Stationary => {},
            MovementBehavior::AxeHover => {
                let new_vel_z = update.vel.0.z + GRAVITY * data.dt.0 * 0.5;
                update.vel.0 = Vec3::new(0.0, 0.0, new_vel_z) + data.inputs.move_dir * 5.0;
            },
            MovementBehavior::GolemHover => {
                update.vel.0 = Vec3::new(0.0, 0.0, 20.0) + *data.inputs.look_dir * 25.0;
            },
        }

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::SpinMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::SpinMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Swing,
                        ..*self
                    });
                }
            },
            StageSection::Swing => {
                if !self.exhausted {
                    update.character = CharacterState::SpinMelee(Data {
                        timer: Duration::default(),
                        exhausted: true,
                        ..*self
                    });

                    let poise = AttackEffect::new(
                        Some(GroupTarget::OutOfGroup),
                        CombatEffect::Poise(self.static_data.base_poise_damage as f32),
                    )
                    .with_requirement(CombatRequirement::AnyDamage);
                    let knockback = AttackEffect::new(
                        Some(GroupTarget::OutOfGroup),
                        CombatEffect::Knockback(self.static_data.knockback),
                    )
                    .with_requirement(CombatRequirement::AnyDamage);
                    let mut damage = AttackDamage::new(
                        Damage {
                            source: DamageSource::Melee,
                            value: self.static_data.base_damage as f32,
                        },
                        Some(GroupTarget::OutOfGroup),
                    );
                    match self.static_data.damage_effect {
                        Some(effect) => damage = damage.with_effect(effect),
                        None => {
                            let buff = CombatEffect::Buff(CombatBuff::default_physical());
                            damage = damage.with_effect(buff);
                        },
                    }
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
                        max_angle: 180_f32.to_radians(),
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
                } else if self.timer < self.static_data.swing_duration {
                    if matches!(
                        self.static_data.movement_behavior,
                        MovementBehavior::ForwardGround
                    ) {
                        handle_forced_movement(
                            data,
                            &mut update,
                            ForcedMovement::Forward {
                                strength: self.static_data.forward_speed,
                            },
                            0.1,
                        );
                        handle_orientation(data, &mut update, 1.0);
                    }

                    // Swings
                    update.character = CharacterState::SpinMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else if update.energy.current() as f32 >= self.static_data.energy_cost
                    && (self.spins_remaining != 0
                        || (self.static_data.is_infinite
                            && input_is_pressed(data, self.static_data.ability_info.input)))
                {
                    let new_spins_remaining = if self.static_data.is_infinite {
                        self.spins_remaining
                    } else {
                        self.spins_remaining - 1
                    };
                    update.character = CharacterState::SpinMelee(Data {
                        timer: Duration::default(),
                        spins_remaining: new_spins_remaining,
                        exhausted: false,
                        ..*self
                    });
                    // Consumes energy if there's enough left and RMB is held down
                    update.energy.change_by(EnergyChange {
                        amount: -(self.static_data.energy_cost as i32),
                        source: EnergySource::Ability,
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::SpinMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                    // Remove melee attack component
                    data.updater.remove::<Melee>(data.entity);
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recover
                    update.character = CharacterState::SpinMelee(Data {
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
            handle_state_interrupt(data, &mut update, self.static_data.is_interruptible);
        }

        update
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MovementBehavior {
    Stationary,
    ForwardGround,
    AxeHover,
    GolemHover,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum FrontendSpecifier {
    CultistVortex,
}
