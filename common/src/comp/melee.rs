use crate::{
    combat::{
        Attack, AttackDamage, AttackEffect, CombatBuff, CombatBuffStrength, CombatEffect,
        CombatRequirement, Damage, DamageKind, DamageSource, GroupTarget, Knockback, KnockbackDir,
    },
    comp::{
        buff::BuffKind,
        tool::{Stats, ToolKind},
    },
};
use common_base::dev_panic;
use serde::{Deserialize, Serialize};
use specs::{Component, VecStorage};
use vek::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Melee {
    pub attack: Attack,
    pub range: f32,
    pub max_angle: f32,
    pub applied: bool,
    pub hit_count: u32,
    pub multi_target: Option<MultiTarget>,
    pub break_block: Option<(Vec3<i32>, Option<ToolKind>)>,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MultiTarget {
    Normal,
    /// Applies scaling to the power of the attack based on how many consecutive
    /// enemies have been hit. First enemy hit will be at a power of 1.0, second
    /// enemy hit will be at a power of `1.0 + scaling`, nth enemy hit will be
    /// at a power of `1.0 + (n - 1) * scaling`.
    Scaling(f32),
}

impl Melee {
    #[must_use]
    pub fn with_block_breaking(
        mut self,
        break_block: Option<(Vec3<i32>, Option<ToolKind>)>,
    ) -> Self {
        self.break_block = break_block;
        self
    }
}

impl Component for Melee {
    type Storage = VecStorage<Self>;
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeleeConstructor {
    pub kind: MeleeConstructorKind,
    // This multiplied by a fraction is added to what is specified in kind
    pub scaled: Option<MeleeConstructorKind>,
    pub range: f32,
    pub angle: f32,
    pub multi_target: Option<MultiTarget>,
    pub damage_effect: Option<CombatEffect>,
}

impl MeleeConstructor {
    pub fn create_melee(self, (crit_chance, crit_mult): (f32, f32), tool_stats: Stats) -> Melee {
        use MeleeConstructorKind::*;
        if self.scaled.is_some() {
            dev_panic!(
                "Attempted to create a melee attack that had a provided scaled value without \
                 scaling the melee attack."
            )
        }
        let instance = rand::random();
        let attack = match self.kind {
            Slash {
                damage,
                poise,
                knockback,
                energy_regen,
            } => {
                let energy = AttackEffect::new(None, CombatEffect::EnergyReward(energy_regen))
                    .with_requirement(CombatRequirement::AnyDamage);
                let buff = CombatEffect::Buff(CombatBuff {
                    kind: BuffKind::Bleeding,
                    dur_secs: 10.0,
                    strength: CombatBuffStrength::DamageFraction(0.1),
                    chance: 0.1,
                })
                .adjusted_by_stats(tool_stats);
                let mut damage = AttackDamage::new(
                    Damage {
                        source: DamageSource::Melee,
                        kind: DamageKind::Slashing,
                        value: damage,
                    },
                    Some(GroupTarget::OutOfGroup),
                    instance,
                )
                .with_effect(buff);

                if let Some(damage_effect) = self.damage_effect {
                    damage = damage.with_effect(damage_effect);
                }

                let poise =
                    AttackEffect::new(Some(GroupTarget::OutOfGroup), CombatEffect::Poise(poise))
                        .with_requirement(CombatRequirement::AnyDamage);
                let knockback = AttackEffect::new(
                    Some(GroupTarget::OutOfGroup),
                    CombatEffect::Knockback(Knockback {
                        strength: knockback,
                        direction: KnockbackDir::Away,
                    })
                    .adjusted_by_stats(tool_stats),
                )
                .with_requirement(CombatRequirement::AnyDamage);

                Attack::default()
                    .with_damage(damage)
                    .with_crit(crit_chance, crit_mult)
                    .with_effect(energy)
                    .with_effect(poise)
                    .with_effect(knockback)
                    .with_combo_increment()
            },
            Stab {
                damage,
                poise,
                knockback,
                energy_regen,
            } => {
                let energy = AttackEffect::new(None, CombatEffect::EnergyReward(energy_regen))
                    .with_requirement(CombatRequirement::AnyDamage);
                let buff = CombatEffect::Buff(CombatBuff {
                    kind: BuffKind::Bleeding,
                    dur_secs: 5.0,
                    strength: CombatBuffStrength::DamageFraction(0.05),
                    chance: 0.1,
                })
                .adjusted_by_stats(tool_stats);
                let mut damage = AttackDamage::new(
                    Damage {
                        source: DamageSource::Melee,
                        kind: DamageKind::Piercing,
                        value: damage,
                    },
                    Some(GroupTarget::OutOfGroup),
                    instance,
                )
                .with_effect(buff);

                if let Some(damage_effect) = self.damage_effect {
                    damage = damage.with_effect(damage_effect);
                }

                let poise =
                    AttackEffect::new(Some(GroupTarget::OutOfGroup), CombatEffect::Poise(poise))
                        .with_requirement(CombatRequirement::AnyDamage);
                let knockback = AttackEffect::new(
                    Some(GroupTarget::OutOfGroup),
                    CombatEffect::Knockback(Knockback {
                        strength: knockback,
                        direction: KnockbackDir::Away,
                    })
                    .adjusted_by_stats(tool_stats),
                )
                .with_requirement(CombatRequirement::AnyDamage);

                Attack::default()
                    .with_damage(damage)
                    .with_crit(crit_chance, crit_mult)
                    .with_effect(energy)
                    .with_effect(poise)
                    .with_effect(knockback)
                    .with_combo_increment()
            },
            Bash {
                damage,
                poise,
                knockback,
                energy_regen,
            } => {
                let energy = AttackEffect::new(None, CombatEffect::EnergyReward(energy_regen))
                    .with_requirement(CombatRequirement::AnyDamage);
                let mut damage = AttackDamage::new(
                    Damage {
                        source: DamageSource::Melee,
                        kind: DamageKind::Crushing,
                        value: damage,
                    },
                    Some(GroupTarget::OutOfGroup),
                    instance,
                );

                if let Some(damage_effect) = self.damage_effect {
                    damage = damage.with_effect(damage_effect);
                }

                let poise =
                    AttackEffect::new(Some(GroupTarget::OutOfGroup), CombatEffect::Poise(poise))
                        .with_requirement(CombatRequirement::AnyDamage);
                let knockback = AttackEffect::new(
                    Some(GroupTarget::OutOfGroup),
                    CombatEffect::Knockback(Knockback {
                        strength: knockback,
                        direction: KnockbackDir::Away,
                    })
                    .adjusted_by_stats(tool_stats),
                )
                .with_requirement(CombatRequirement::AnyDamage);

                Attack::default()
                    .with_damage(damage)
                    .with_crit(crit_chance, crit_mult)
                    .with_effect(energy)
                    .with_effect(poise)
                    .with_effect(knockback)
                    .with_combo_increment()
            },
            NecroticVortex {
                damage,
                pull,
                lifesteal,
            } => {
                let lifesteal = CombatEffect::Lifesteal(lifesteal);

                let mut damage = AttackDamage::new(
                    Damage {
                        source: DamageSource::Melee,
                        kind: DamageKind::Energy,
                        value: damage,
                    },
                    None,
                    instance,
                )
                .with_effect(lifesteal);

                if let Some(damage_effect) = self.damage_effect {
                    damage = damage.with_effect(damage_effect);
                }

                let knockback = AttackEffect::new(
                    Some(GroupTarget::OutOfGroup),
                    CombatEffect::Knockback(Knockback {
                        strength: pull,
                        direction: KnockbackDir::Towards,
                    })
                    .adjusted_by_stats(tool_stats),
                )
                .with_requirement(CombatRequirement::AnyDamage);

                Attack::default()
                    .with_damage(damage)
                    .with_crit(crit_chance, crit_mult)
                    .with_effect(knockback)
                    .with_combo_increment()
            },
            SonicWave {
                damage,
                poise,
                knockback,
            } => {
                let mut damage = AttackDamage::new(
                    Damage {
                        source: DamageSource::Melee,
                        kind: DamageKind::Energy,
                        value: damage,
                    },
                    Some(GroupTarget::OutOfGroup),
                    instance,
                );

                if let Some(damage_effect) = self.damage_effect {
                    damage = damage.with_effect(damage_effect);
                }

                let poise =
                    AttackEffect::new(Some(GroupTarget::OutOfGroup), CombatEffect::Poise(poise))
                        .with_requirement(CombatRequirement::AnyDamage);
                let knockback = AttackEffect::new(
                    Some(GroupTarget::OutOfGroup),
                    CombatEffect::Knockback(Knockback {
                        strength: knockback,
                        direction: KnockbackDir::Away,
                    })
                    .adjusted_by_stats(tool_stats),
                )
                .with_requirement(CombatRequirement::AnyDamage);

                Attack::default()
                    .with_damage(damage)
                    .with_crit(crit_chance, crit_mult)
                    .with_effect(poise)
                    .with_effect(knockback)
                    .with_combo_increment()
            },
        };

        Melee {
            attack,
            range: self.range,
            max_angle: self.angle.to_radians(),
            applied: false,
            hit_count: 0,
            multi_target: self.multi_target,
            break_block: None,
        }
    }

    #[must_use]
    pub fn handle_scaling(mut self, scaling: f32) -> Self {
        let scale_values = |a, b| a + b * scaling;

        if let Some(max_scale) = self.scaled {
            use MeleeConstructorKind::*;
            let scaled = match (self.kind, max_scale) {
                (
                    Slash {
                        damage: a_damage,
                        poise: a_poise,
                        knockback: a_knockback,
                        energy_regen: a_energy_regen,
                    },
                    Slash {
                        damage: b_damage,
                        poise: b_poise,
                        knockback: b_knockback,
                        energy_regen: b_energy_regen,
                    },
                ) => Slash {
                    damage: scale_values(a_damage, b_damage),
                    poise: scale_values(a_poise, b_poise),
                    knockback: scale_values(a_knockback, b_knockback),
                    energy_regen: scale_values(a_energy_regen, b_energy_regen),
                },
                (
                    Stab {
                        damage: a_damage,
                        poise: a_poise,
                        knockback: a_knockback,
                        energy_regen: a_energy_regen,
                    },
                    Stab {
                        damage: b_damage,
                        poise: b_poise,
                        knockback: b_knockback,
                        energy_regen: b_energy_regen,
                    },
                ) => Stab {
                    damage: scale_values(a_damage, b_damage),
                    poise: scale_values(a_poise, b_poise),
                    knockback: scale_values(a_knockback, b_knockback),
                    energy_regen: scale_values(a_energy_regen, b_energy_regen),
                },
                (
                    Bash {
                        damage: a_damage,
                        poise: a_poise,
                        knockback: a_knockback,
                        energy_regen: a_energy_regen,
                    },
                    Bash {
                        damage: b_damage,
                        poise: b_poise,
                        knockback: b_knockback,
                        energy_regen: b_energy_regen,
                    },
                ) => Bash {
                    damage: scale_values(a_damage, b_damage),
                    poise: scale_values(a_poise, b_poise),
                    knockback: scale_values(a_knockback, b_knockback),
                    energy_regen: scale_values(a_energy_regen, b_energy_regen),
                },
                (
                    NecroticVortex {
                        damage: a_damage,
                        pull: a_pull,
                        lifesteal: a_lifesteal,
                    },
                    NecroticVortex {
                        damage: b_damage,
                        pull: b_pull,
                        lifesteal: b_lifesteal,
                    },
                ) => NecroticVortex {
                    damage: scale_values(a_damage, b_damage),
                    pull: scale_values(a_pull, b_pull),
                    lifesteal: scale_values(a_lifesteal, b_lifesteal),
                },
                (
                    SonicWave {
                        damage: a_damage,
                        poise: a_poise,
                        knockback: a_knockback,
                    },
                    SonicWave {
                        damage: b_damage,
                        poise: b_poise,
                        knockback: b_knockback,
                    },
                ) => SonicWave {
                    damage: scale_values(a_damage, b_damage),
                    poise: scale_values(a_poise, b_poise),
                    knockback: scale_values(a_knockback, b_knockback),
                },
                _ => {
                    dev_panic!(
                        "Attempted to scale on a melee attack between two different kinds of \
                         melee constructors."
                    );
                    self.kind
                },
            };
            self.kind = scaled;
            self.scaled = None;
        } else {
            dev_panic!("Attempted to scale on a melee attack that had no provided scaling value.")
        }
        self
    }

    #[must_use]
    pub fn adjusted_by_stats(mut self, stats: Stats) -> Self {
        self.range *= stats.range;
        self.kind = self.kind.adjusted_by_stats(stats);
        if let Some(ref mut scaled) = &mut self.scaled {
            *scaled = scaled.adjusted_by_stats(stats);
        }
        self.damage_effect = self.damage_effect.map(|de| de.adjusted_by_stats(stats));
        self
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
// TODO: Get someone not me to name these variants
pub enum MeleeConstructorKind {
    Slash {
        damage: f32,
        poise: f32,
        knockback: f32,
        energy_regen: f32,
    },
    Stab {
        damage: f32,
        poise: f32,
        knockback: f32,
        energy_regen: f32,
    },
    Bash {
        damage: f32,
        poise: f32,
        knockback: f32,
        energy_regen: f32,
    },
    NecroticVortex {
        damage: f32,
        pull: f32,
        lifesteal: f32,
    },
    SonicWave {
        damage: f32,
        poise: f32,
        knockback: f32,
    },
}

impl MeleeConstructorKind {
    #[must_use]
    pub fn adjusted_by_stats(mut self, stats: Stats) -> Self {
        use MeleeConstructorKind::*;
        match self {
            Slash {
                ref mut damage,
                ref mut poise,
                knockback: _,
                energy_regen: _,
            } => {
                *damage *= stats.power;
                *poise *= stats.effect_power;
            },
            Stab {
                ref mut damage,
                ref mut poise,
                knockback: _,
                energy_regen: _,
            } => {
                *damage *= stats.power;
                *poise *= stats.effect_power;
            },
            Bash {
                ref mut damage,
                ref mut poise,
                knockback: _,
                energy_regen: _,
            } => {
                *damage *= stats.power;
                *poise *= stats.effect_power;
            },
            NecroticVortex {
                ref mut damage,
                pull: _,
                lifesteal: _,
            } => {
                *damage *= stats.power;
            },
            SonicWave {
                ref mut damage,
                ref mut poise,
                knockback: _,
            } => {
                *damage *= stats.power;
                *poise *= stats.effect_power;
            },
        }
        self
    }
}
