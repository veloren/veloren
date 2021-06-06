#[cfg(not(target_arch = "wasm32"))]
use crate::{
    comp::{
        buff::{Buff, BuffChange, BuffData, BuffKind, BuffSource},
        inventory::{
            item::{
                armor::Protection,
                tool::{self, Tool, ToolKind},
                Item, ItemDesc, ItemKind, MaterialStatManifest,
            },
            slot::EquipSlot,
        },
        poise::PoiseChange,
        skills::SkillGroupKind,
        Body, CharacterState, Combo, Energy, EnergyChange, EnergySource, Health, HealthChange,
        HealthSource, Inventory, Ori, SkillSet, Stats,
    },
    event::ServerEvent,
    outcome::Outcome,
    states::utils::StageSection,
    uid::Uid,
    util::Dir,
};

#[cfg(not(target_arch = "wasm32"))]
use rand::{thread_rng, Rng};

use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use specs::Entity as EcsEntity;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))] use vek::*;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GroupTarget {
    InGroup,
    OutOfGroup,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum AttackSource {
    Melee,
    Projectile,
    Beam,
    Shockwave,
    Explosion,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Copy, Clone)]
pub struct AttackerInfo<'a> {
    pub entity: EcsEntity,
    pub uid: Uid,
    pub energy: Option<&'a Energy>,
    pub combo: Option<&'a Combo>,
    pub inventory: Option<&'a Inventory>,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct TargetInfo<'a> {
    pub entity: EcsEntity,
    pub uid: Uid,
    pub inventory: Option<&'a Inventory>,
    pub stats: Option<&'a Stats>,
    pub health: Option<&'a Health>,
    pub pos: Vec3<f32>,
    pub ori: Option<&'a Ori>,
    pub char_state: Option<&'a CharacterState>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)] // TODO: Yeet clone derive
pub struct Attack {
    damages: Vec<AttackDamage>,
    effects: Vec<AttackEffect>,
    crit_chance: f32,
    crit_multiplier: f32,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for Attack {
    fn default() -> Self {
        Self {
            damages: Vec::new(),
            effects: Vec::new(),
            crit_chance: 0.0,
            crit_multiplier: 1.0,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Attack {
    pub fn with_damage(mut self, damage: AttackDamage) -> Self {
        self.damages.push(damage);
        self
    }

    pub fn with_effect(mut self, effect: AttackEffect) -> Self {
        self.effects.push(effect);
        self
    }

    pub fn with_crit(mut self, crit_chance: f32, crit_multiplier: f32) -> Self {
        self.crit_chance = crit_chance;
        self.crit_multiplier = crit_multiplier;
        self
    }

    pub fn with_combo_increment(self) -> Self {
        self.with_effect(
            AttackEffect::new(None, CombatEffect::Combo(1))
                .with_requirement(CombatRequirement::AnyDamage),
        )
    }

    pub fn effects(&self) -> impl Iterator<Item = &AttackEffect> { self.effects.iter() }

    pub fn compute_damage_reduction(
        target: &TargetInfo,
        source: AttackSource,
        dir: Dir,
        kind: DamageKind,
        mut emit_outcome: impl FnMut(Outcome),
    ) -> f32 {
        let damage_reduction =
            Damage::compute_damage_reduction(target.inventory, target.stats, Some(kind));
        let block_reduction = match source {
            AttackSource::Melee => {
                if let (Some(CharacterState::BasicBlock(data)), Some(ori)) =
                    (target.char_state, target.ori)
                {
                    if ori.look_vec().angle_between(-*dir) < data.static_data.max_angle.to_radians()
                    {
                        let parry = matches!(data.stage_section, StageSection::Buildup);
                        emit_outcome(Outcome::Block {
                            parry,
                            pos: target.pos,
                            uid: target.uid,
                        });
                        if parry {
                            1.0
                        } else {
                            data.static_data.block_strength
                        }
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            },
            _ => 0.0,
        };
        1.0 - (1.0 - damage_reduction) * (1.0 - block_reduction)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_attack(
        &self,
        target_group: GroupTarget,
        attacker: Option<AttackerInfo>,
        target: TargetInfo,
        dir: Dir,
        target_dodging: bool,
        // Currently just modifies damage, maybe look into modifying strength of other effects?
        strength_modifier: f32,
        attack_source: AttackSource,
        mut emit: impl FnMut(ServerEvent),
        mut emit_outcome: impl FnMut(Outcome),
    ) {
        let is_crit = thread_rng().gen::<f32>() < self.crit_chance;
        let mut accumulated_damage = 0.0;
        for damage in self
            .damages
            .iter()
            .filter(|d| d.target.map_or(true, |t| t == target_group))
            .filter(|d| !(matches!(d.target, Some(GroupTarget::OutOfGroup)) && target_dodging))
        {
            let damage_reduction = Attack::compute_damage_reduction(
                &target,
                attack_source,
                dir,
                damage.damage.kind,
                |o| emit_outcome(o),
            );
            let change = damage.damage.calculate_health_change(
                damage_reduction,
                attacker.map(|a| a.uid),
                is_crit,
                self.crit_multiplier,
                strength_modifier,
            );
            let applied_damage = -change.amount as f32;
            accumulated_damage += applied_damage;
            emit_outcome(Outcome::Damage { pos: target.pos });
            if change.amount != 0 {
                emit(ServerEvent::Damage {
                    entity: target.entity,
                    change,
                });
                for effect in damage.effects.iter() {
                    match effect {
                        CombatEffect::Knockback(kb) => {
                            let impulse = kb.calculate_impulse(dir);
                            if !impulse.is_approx_zero() {
                                emit(ServerEvent::Knockback {
                                    entity: target.entity,
                                    impulse,
                                });
                            }
                        },
                        CombatEffect::EnergyReward(ec) => {
                            if let Some(attacker) = attacker {
                                emit(ServerEvent::EnergyChange {
                                    entity: attacker.entity,
                                    change: EnergyChange {
                                        amount: (*ec
                                            * compute_energy_reward_mod(attacker.inventory))
                                            as i32,
                                        source: EnergySource::HitEnemy,
                                    },
                                });
                            }
                        },
                        CombatEffect::Buff(b) => {
                            if thread_rng().gen::<f32>() < b.chance {
                                emit(ServerEvent::Buff {
                                    entity: target.entity,
                                    buff_change: BuffChange::Add(
                                        b.to_buff(attacker.map(|a| a.uid), applied_damage),
                                    ),
                                });
                            }
                        },
                        CombatEffect::Lifesteal(l) => {
                            if let Some(attacker_entity) = attacker.map(|a| a.entity) {
                                let change = HealthChange {
                                    amount: (applied_damage * l) as i32,
                                    cause: HealthSource::Heal {
                                        by: attacker.map(|a| a.uid),
                                    },
                                };
                                if change.amount != 0 {
                                    emit(ServerEvent::Damage {
                                        entity: attacker_entity,
                                        change,
                                    });
                                }
                            }
                        },
                        CombatEffect::Poise(p) => {
                            let change = PoiseChange::from_value(*p, target.inventory);
                            if change.amount != 0 {
                                emit(ServerEvent::PoiseChange {
                                    entity: target.entity,
                                    change,
                                    kb_dir: *dir,
                                });
                            }
                        },
                        CombatEffect::Heal(h) => {
                            let change = HealthChange {
                                amount: *h as i32,
                                cause: HealthSource::Heal {
                                    by: attacker.map(|a| a.uid),
                                },
                            };
                            if change.amount != 0 {
                                emit(ServerEvent::Damage {
                                    entity: target.entity,
                                    change,
                                });
                            }
                        },
                        CombatEffect::Combo(c) => {
                            if let Some(attacker_entity) = attacker.map(|a| a.entity) {
                                emit(ServerEvent::ComboChange {
                                    entity: attacker_entity,
                                    change: *c,
                                });
                            }
                        },
                    }
                }
            }
        }
        for effect in self
            .effects
            .iter()
            .filter(|e| e.target.map_or(true, |t| t == target_group))
            .filter(|e| !(matches!(e.target, Some(GroupTarget::OutOfGroup)) && target_dodging))
        {
            if effect.requirements.iter().all(|req| match req {
                CombatRequirement::AnyDamage => accumulated_damage > 0.0 && target.health.is_some(),
                CombatRequirement::Energy(r) => {
                    if let Some(AttackerInfo {
                        entity,
                        energy: Some(e),
                        ..
                    }) = attacker
                    {
                        let sufficient_energy = e.current() as f32 >= *r;
                        if sufficient_energy {
                            emit(ServerEvent::EnergyChange {
                                entity,
                                change: EnergyChange {
                                    amount: -(*r as i32),
                                    source: EnergySource::Ability,
                                },
                            });
                        }

                        sufficient_energy
                    } else {
                        false
                    }
                },
                CombatRequirement::Combo(r) => {
                    if let Some(AttackerInfo {
                        entity,
                        combo: Some(c),
                        ..
                    }) = attacker
                    {
                        let sufficient_combo = c.counter() >= *r;
                        if sufficient_combo {
                            emit(ServerEvent::ComboChange {
                                entity,
                                change: -(*r as i32),
                            });
                        }

                        sufficient_combo
                    } else {
                        false
                    }
                },
            }) {
                match effect.effect {
                    CombatEffect::Knockback(kb) => {
                        let impulse = kb.calculate_impulse(dir);
                        if !impulse.is_approx_zero() {
                            emit(ServerEvent::Knockback {
                                entity: target.entity,
                                impulse,
                            });
                        }
                    },
                    CombatEffect::EnergyReward(ec) => {
                        if let Some(attacker) = attacker {
                            emit(ServerEvent::EnergyChange {
                                entity: attacker.entity,
                                change: EnergyChange {
                                    amount: (ec * compute_energy_reward_mod(attacker.inventory))
                                        as i32,
                                    source: EnergySource::HitEnemy,
                                },
                            });
                        }
                    },
                    CombatEffect::Buff(b) => {
                        if thread_rng().gen::<f32>() < b.chance {
                            emit(ServerEvent::Buff {
                                entity: target.entity,
                                buff_change: BuffChange::Add(
                                    b.to_buff(attacker.map(|a| a.uid), accumulated_damage),
                                ),
                            });
                        }
                    },
                    CombatEffect::Lifesteal(l) => {
                        if let Some(attacker_entity) = attacker.map(|a| a.entity) {
                            let change = HealthChange {
                                amount: (accumulated_damage * l) as i32,
                                cause: HealthSource::Heal {
                                    by: attacker.map(|a| a.uid),
                                },
                            };
                            if change.amount != 0 {
                                emit(ServerEvent::Damage {
                                    entity: attacker_entity,
                                    change,
                                });
                            }
                        }
                    },
                    CombatEffect::Poise(p) => {
                        let change = PoiseChange::from_value(p, target.inventory);
                        if change.amount != 0 {
                            emit(ServerEvent::PoiseChange {
                                entity: target.entity,
                                change,
                                kb_dir: *dir,
                            });
                        }
                    },
                    CombatEffect::Heal(h) => {
                        let change = HealthChange {
                            amount: h as i32,
                            cause: HealthSource::Heal {
                                by: attacker.map(|a| a.uid),
                            },
                        };
                        if change.amount != 0 {
                            emit(ServerEvent::Damage {
                                entity: target.entity,
                                change,
                            });
                        }
                    },
                    CombatEffect::Combo(c) => {
                        if let Some(attacker_entity) = attacker.map(|a| a.entity) {
                            emit(ServerEvent::ComboChange {
                                entity: attacker_entity,
                                change: c,
                            });
                        }
                    },
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttackDamage {
    damage: Damage,
    target: Option<GroupTarget>,
    effects: Vec<CombatEffect>,
}

#[cfg(not(target_arch = "wasm32"))]
impl AttackDamage {
    pub fn new(damage: Damage, target: Option<GroupTarget>) -> Self {
        Self {
            damage,
            target,
            effects: Vec::new(),
        }
    }

    pub fn with_effect(mut self, effect: CombatEffect) -> Self {
        self.effects.push(effect);
        self
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttackEffect {
    target: Option<GroupTarget>,
    effect: CombatEffect,
    requirements: Vec<CombatRequirement>,
}

#[cfg(not(target_arch = "wasm32"))]
impl AttackEffect {
    pub fn new(target: Option<GroupTarget>, effect: CombatEffect) -> Self {
        Self {
            target,
            effect,
            requirements: Vec::new(),
        }
    }

    pub fn with_requirement(mut self, requirement: CombatRequirement) -> Self {
        self.requirements.push(requirement);
        self
    }

    pub fn effect(&self) -> &CombatEffect { &self.effect }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum CombatEffect {
    Heal(f32),
    Buff(CombatBuff),
    Knockback(Knockback),
    EnergyReward(f32),
    Lifesteal(f32),
    Poise(f32),
    Combo(i32),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CombatRequirement {
    AnyDamage,
    Energy(f32),
    Combo(u32),
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageSource {
    Buff(BuffKind),
    Melee,
    Projectile,
    Explosion,
    Falling,
    Shockwave,
    Energy,
    Other,
}

/// DamageKind for the purpose of differentiating damage reduction
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageKind {
    /// Arrows/Sword dash
    Piercing,
    /// Swords/axes
    Slashing,
    /// Hammers
    Crushing,
    /// Staves/sceptres (TODO: differentiate further once there are more magic
    /// weapons)
    Energy,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Damage {
    pub source: DamageSource,
    pub kind: DamageKind,
    pub value: f32,
}

#[cfg(not(target_arch = "wasm32"))]
impl Damage {
    /// Returns the total damage reduction provided by all equipped items
    pub fn compute_damage_reduction(
        inventory: Option<&Inventory>,
        stats: Option<&Stats>,
        kind: Option<DamageKind>,
    ) -> f32 {
        let inventory_dr = if let Some(inventory) = inventory {
            let protection = inventory
                .equipped_items()
                .filter_map(|item| {
                    if let ItemKind::Armor(armor) = &item.kind() {
                        Some(armor.protection())
                    } else {
                        None
                    }
                })
                .map(|protection| match protection {
                    Protection::Normal(protection) => Some(protection),
                    Protection::Invincible => None,
                })
                .sum::<Option<f32>>();

            let kind_modifier = if matches!(kind, Some(DamageKind::Piercing)) {
                0.75
            } else {
                1.0
            };
            let protection = protection.map(|dr| dr * kind_modifier);

            const FIFTY_PERCENT_DR_THRESHOLD: f32 = 60.0;

            match protection {
                Some(dr) => dr / (FIFTY_PERCENT_DR_THRESHOLD + dr.abs()),
                None => 1.0,
            }
        } else {
            0.0
        };
        let stats_dr = if let Some(stats) = stats {
            stats.damage_reduction
        } else {
            0.0
        };
        1.0 - (1.0 - inventory_dr) * (1.0 - stats_dr)
    }

    pub fn calculate_health_change(
        self,
        damage_reduction: f32,
        uid: Option<Uid>,
        is_crit: bool,
        crit_mult: f32,
        damage_modifier: f32,
    ) -> HealthChange {
        let mut damage = self.value * damage_modifier;
        // Critical hit damage (to be applied post-armor for melee, and pre-armor for
        // other damage kinds
        let critdamage = if is_crit {
            damage * (crit_mult - 1.0)
        } else {
            0.0
        };
        match self.source {
            DamageSource::Melee => {
                // Armor
                damage *= 1.0 - damage_reduction;

                // Critical damage applies after armor for melee
                if (damage_reduction - 1.0).abs() > f32::EPSILON {
                    damage += critdamage;
                }

                HealthChange {
                    amount: -damage as i32,
                    cause: HealthSource::Damage {
                        kind: self.source,
                        by: uid,
                    },
                }
            },
            DamageSource::Projectile => {
                // Critical hit
                damage += critdamage;
                // Armor
                damage *= 1.0 - damage_reduction;

                HealthChange {
                    amount: -damage as i32,
                    cause: HealthSource::Damage {
                        kind: self.source,
                        by: uid,
                    },
                }
            },
            DamageSource::Explosion => {
                // Critical hit
                damage += critdamage;
                // Armor
                damage *= 1.0 - damage_reduction;

                HealthChange {
                    amount: -damage as i32,
                    cause: HealthSource::Damage {
                        kind: self.source,
                        by: uid,
                    },
                }
            },
            DamageSource::Shockwave => {
                // Critical hit
                damage += critdamage;
                // Armor
                damage *= 1.0 - damage_reduction;

                HealthChange {
                    amount: -damage as i32,
                    cause: HealthSource::Damage {
                        kind: self.source,
                        by: uid,
                    },
                }
            },
            DamageSource::Energy => {
                // Critical hit
                damage += critdamage;
                // Armor
                damage *= 1.0 - damage_reduction;

                HealthChange {
                    amount: -damage as i32,
                    cause: HealthSource::Damage {
                        kind: self.source,
                        by: uid,
                    },
                }
            },
            DamageSource::Falling => {
                // Armor
                if (damage_reduction - 1.0).abs() < f32::EPSILON {
                    damage = 0.0;
                }
                HealthChange {
                    amount: -damage as i32,
                    cause: HealthSource::World,
                }
            },
            DamageSource::Buff(_) => HealthChange {
                amount: -damage as i32,
                cause: HealthSource::Damage {
                    kind: self.source,
                    by: uid,
                },
            },
            DamageSource::Other => HealthChange {
                amount: -damage as i32,
                cause: HealthSource::Damage {
                    kind: self.source,
                    by: uid,
                },
            },
        }
    }

    pub fn interpolate_damage(&mut self, frac: f32, min: f32) {
        let new_damage = min + frac * (self.value - min);
        self.value = new_damage;
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Knockback {
    pub direction: KnockbackDir,
    pub strength: f32,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnockbackDir {
    Away,
    Towards,
    Up,
    TowardsUp,
}

#[cfg(not(target_arch = "wasm32"))]
impl Knockback {
    pub fn calculate_impulse(self, dir: Dir) -> Vec3<f32> {
        // TEMP until source knockback values have been updated
        50.0 * match self.direction {
            KnockbackDir::Away => self.strength * *Dir::slerp(dir, Dir::new(Vec3::unit_z()), 0.5),
            KnockbackDir::Towards => {
                self.strength * *Dir::slerp(-dir, Dir::new(Vec3::unit_z()), 0.5)
            },
            KnockbackDir::Up => self.strength * Vec3::unit_z(),
            KnockbackDir::TowardsUp => {
                self.strength * *Dir::slerp(-dir, Dir::new(Vec3::unit_z()), 0.85)
            },
        }
    }

    pub fn modify_strength(mut self, power: f32) -> Self {
        self.strength *= power;
        self
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatBuff {
    pub kind: BuffKind,
    pub dur_secs: f32,
    pub strength: CombatBuffStrength,
    pub chance: f32,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum CombatBuffStrength {
    DamageFraction(f32),
    Value(f32),
}

#[cfg(not(target_arch = "wasm32"))]
impl CombatBuffStrength {
    fn to_strength(self, damage: f32) -> f32 {
        match self {
            CombatBuffStrength::DamageFraction(f) => damage * f,
            CombatBuffStrength::Value(v) => v,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl CombatBuff {
    fn to_buff(self, uid: Option<Uid>, damage: f32) -> Buff {
        // TODO: Generate BufCategoryId vec (probably requires damage overhaul?)
        let source = if let Some(uid) = uid {
            BuffSource::Character { by: uid }
        } else {
            BuffSource::Unknown
        };
        Buff::new(
            self.kind,
            BuffData::new(
                self.strength.to_strength(damage),
                Some(Duration::from_secs_f32(self.dur_secs)),
            ),
            Vec::new(),
            source,
        )
    }

    pub fn default_physical() -> Self {
        Self {
            kind: BuffKind::Bleeding,
            dur_secs: 10.0,
            strength: CombatBuffStrength::DamageFraction(0.1),
            chance: 0.1,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn equipped_item_and_tool(inv: &Inventory, slot: EquipSlot) -> Option<(&Item, &Tool)> {
    inv.equipped(slot).and_then(|i| {
        if let ItemKind::Tool(tool) = &i.kind() {
            Some((i, tool))
        } else {
            None
        }
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_weapons(inv: &Inventory) -> (Option<ToolKind>, Option<ToolKind>) {
    (
        equipped_item_and_tool(inv, EquipSlot::ActiveMainhand).map(|(_, tool)| tool.kind),
        equipped_item_and_tool(inv, EquipSlot::ActiveOffhand).map(|(_, tool)| tool.kind),
    )
}

pub fn weapon_rating<T: ItemDesc>(item: &T, msm: &MaterialStatManifest) -> f32 {
    const DAMAGE_WEIGHT: f32 = 2.0;
    const POISE_WEIGHT: f32 = 1.0;

    if let ItemKind::Tool(tool) = item.kind() {
        let stats = tool::Stats::from((msm, item.components(), tool));

        // TODO: Look into changing the 0.5 to reflect armor later maybe?
        // Since it is only for weapon though, it probably makes sense to leave
        // independent for now
        let damage_rating = stats.power * stats.speed * (1.0 + stats.crit_chance * 0.5);
        let poise_rating = stats.poise_strength * stats.speed;

        (damage_rating * DAMAGE_WEIGHT + poise_rating * POISE_WEIGHT)
            / (DAMAGE_WEIGHT + POISE_WEIGHT)
    } else {
        0.0
    }
}

fn weapon_skills(inventory: &Inventory, skill_set: &SkillSet) -> f32 {
    let (mainhand, offhand) = get_weapons(inventory);
    let mainhand_skills = if let Some(tool) = mainhand {
        skill_set.earned_sp(SkillGroupKind::Weapon(tool)) as f32
    } else {
        0.0
    };
    let offhand_skills = if let Some(tool) = offhand {
        skill_set.earned_sp(SkillGroupKind::Weapon(tool)) as f32
    } else {
        0.0
    };
    mainhand_skills.max(offhand_skills)
}

fn get_weapon_rating(inventory: &Inventory, msm: &MaterialStatManifest) -> f32 {
    let mainhand_rating =
        if let Some((item, _)) = equipped_item_and_tool(inventory, EquipSlot::ActiveMainhand) {
            weapon_rating(item, msm)
        } else {
            0.0
        };

    let offhand_rating =
        if let Some((item, _)) = equipped_item_and_tool(inventory, EquipSlot::ActiveOffhand) {
            weapon_rating(item, msm)
        } else {
            0.0
        };

    mainhand_rating.max(offhand_rating)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn combat_rating(
    inventory: &Inventory,
    health: &Health,
    skill_set: &SkillSet,
    body: Body,
    msm: &MaterialStatManifest,
) -> f32 {
    const WEAPON_WEIGHT: f32 = 1.0;
    const HEALTH_WEIGHT: f32 = 1.0;
    const SKILLS_WEIGHT: f32 = 1.0;
    // Assumes a "standard" max health of 100
    let health_rating = health.base_max() as f32
        / 100.0
        / (1.0 - Damage::compute_damage_reduction(Some(inventory), None, None)).max(0.00001);

    // Assumes a standard person has earned 20 skill points in the general skill
    // tree and 10 skill points for the weapon skill tree
    let skills_rating = (skill_set.earned_sp(SkillGroupKind::General) as f32 / 20.0
        + weapon_skills(inventory, skill_set) / 10.0)
        / 2.0;

    let weapon_rating = get_weapon_rating(inventory, msm);

    let combined_rating = (health_rating * HEALTH_WEIGHT
        + skills_rating * SKILLS_WEIGHT
        + weapon_rating * WEAPON_WEIGHT)
        / (HEALTH_WEIGHT + SKILLS_WEIGHT + WEAPON_WEIGHT);

    // Body multiplier meant to account for an enemy being harder than equipment and
    // skills would account for. It should only not be 1.0 for non-humanoids
    combined_rating * body.combat_multiplier()
}

pub fn compute_crit_mult(inventory: Option<&Inventory>) -> f32 {
    // Starts with a value of 1.25 when summing the stats from each armor piece, and
    // defaults to a value of 1.25 if no inventory is equipped
    inventory.map_or(1.25, |inv| {
        inv.equipped_items()
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &item.kind() {
                    Some(armor.crit_power())
                } else {
                    None
                }
            })
            .fold(1.25, |a, b| a + b)
    })
}

/// Computes the energy reward modifer from worn armor
pub fn compute_energy_reward_mod(inventory: Option<&Inventory>) -> f32 {
    // Starts with a value of 1.0 when summing the stats from each armor piece, and
    // defaults to a value of 1.0 if no inventory is present
    inventory.map_or(1.0, |inv| {
        inv.equipped_items()
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &item.kind() {
                    Some(armor.energy_reward())
                } else {
                    None
                }
            })
            .fold(1.0, |a, b| a + b)
    })
}

/// Computes the modifier that should be applied to max energy from the
/// currently equipped items
pub fn compute_max_energy_mod(energy: &Energy, inventory: Option<&Inventory>) -> f32 {
    // Defaults to a value of 0 if no inventory is present
    let energy_increase = inventory.map_or(0, |inv| {
        inv.equipped_items()
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &item.kind() {
                    Some(armor.energy_max())
                } else {
                    None
                }
            })
            .sum()
    });
    // Returns the energy increase divided by base max of energy.
    // This value is then added to the max_energy_modifier field on stats component.
    // Adding is important here, as it ensures that a flat modifier is applied
    // correctly.
    energy_increase as f32 / energy.base_max() as f32
}
