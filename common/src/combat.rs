use crate::comp::buff::BuffKind;
#[cfg(not(target_arch = "wasm32"))]
use crate::{
    comp::{
        buff::{Buff, BuffChange, BuffData, BuffSource},
        inventory::{
            item::{
                armor::Protection,
                tool::{Tool, ToolKind},
                Item, ItemKind,
            },
            slot::EquipSlot,
        },
        poise::PoiseChange,
        skills::{SkillGroupKind, SkillSet},
        Body, Energy, EnergyChange, EnergySource, Health, HealthChange, HealthSource, Inventory,
        Stats,
    },
    event::ServerEvent,
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

#[cfg(not(target_arch = "wasm32"))]
#[derive(Copy, Clone)]
pub struct AttackerInfo<'a> {
    pub entity: EcsEntity,
    pub uid: Uid,
    pub energy: Option<&'a Energy>,
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

    pub fn effects(&self) -> impl Iterator<Item = &AttackEffect> { self.effects.iter() }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_attack(
        &self,
        target_group: GroupTarget,
        attacker_info: Option<AttackerInfo>,
        target_entity: EcsEntity,
        target_inventory: Option<&Inventory>,
        dir: Dir,
        target_dodging: bool,
        // Currently just modifies damage, maybe look into modifying strength of other effects?
        strength_modifier: f32,
        mut emit: impl FnMut(ServerEvent),
    ) {
        let is_crit = thread_rng().gen::<f32>() < self.crit_chance;
        let mut accumulated_damage = 0.0;
        for damage in self
            .damages
            .iter()
            .filter(|d| d.target.map_or(true, |t| t == target_group))
            .filter(|d| !(matches!(d.target, Some(GroupTarget::OutOfGroup)) && target_dodging))
        {
            let change = damage.damage.calculate_health_change(
                target_inventory,
                attacker_info.map(|a| a.uid),
                is_crit,
                self.crit_multiplier,
                strength_modifier,
            );
            let applied_damage = -change.amount as f32;
            accumulated_damage += applied_damage;
            if change.amount != 0 {
                emit(ServerEvent::Damage {
                    entity: target_entity,
                    change,
                });
                for effect in damage.effects.iter() {
                    match effect {
                        CombatEffect::Knockback(kb) => {
                            let impulse = kb.calculate_impulse(dir);
                            if !impulse.is_approx_zero() {
                                emit(ServerEvent::Knockback {
                                    entity: target_entity,
                                    impulse,
                                });
                            }
                        },
                        CombatEffect::EnergyReward(ec) => {
                            if let Some(attacker_entity) = attacker_info.map(|a| a.entity) {
                                emit(ServerEvent::EnergyChange {
                                    entity: attacker_entity,
                                    change: EnergyChange {
                                        amount: *ec as i32,
                                        source: EnergySource::HitEnemy,
                                    },
                                });
                            }
                        },
                        CombatEffect::Buff(b) => {
                            if thread_rng().gen::<f32>() < b.chance {
                                emit(ServerEvent::Buff {
                                    entity: target_entity,
                                    buff_change: BuffChange::Add(
                                        b.to_buff(attacker_info.map(|a| a.uid), applied_damage),
                                    ),
                                });
                            }
                        },
                        CombatEffect::Lifesteal(l) => {
                            if let Some(attacker_entity) = attacker_info.map(|a| a.entity) {
                                let change = HealthChange {
                                    amount: (applied_damage * l) as i32,
                                    cause: HealthSource::Heal {
                                        by: attacker_info.map(|a| a.uid),
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
                            let change = PoiseChange::from_value(*p, target_inventory);
                            if change.amount != 0 {
                                emit(ServerEvent::PoiseChange {
                                    entity: target_entity,
                                    change,
                                    kb_dir: *dir,
                                });
                            }
                        },
                        CombatEffect::Heal(h) => {
                            let change = HealthChange {
                                amount: *h as i32,
                                cause: HealthSource::Heal {
                                    by: attacker_info.map(|a| a.uid),
                                },
                            };
                            if change.amount != 0 {
                                emit(ServerEvent::Damage {
                                    entity: target_entity,
                                    change,
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
            if match &effect.requirement {
                Some(CombatRequirement::AnyDamage) => accumulated_damage > 0.0,
                Some(CombatRequirement::SufficientEnergy(r)) => {
                    if let Some(AttackerInfo {
                        entity,
                        energy: Some(e),
                        ..
                    }) = attacker_info
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
                None => true,
            } {
                match effect.effect {
                    CombatEffect::Knockback(kb) => {
                        let impulse = kb.calculate_impulse(dir);
                        if !impulse.is_approx_zero() {
                            emit(ServerEvent::Knockback {
                                entity: target_entity,
                                impulse,
                            });
                        }
                    },
                    CombatEffect::EnergyReward(ec) => {
                        if let Some(attacker_entity) = attacker_info.map(|a| a.entity) {
                            emit(ServerEvent::EnergyChange {
                                entity: attacker_entity,
                                change: EnergyChange {
                                    amount: ec as i32,
                                    source: EnergySource::HitEnemy,
                                },
                            });
                        }
                    },
                    CombatEffect::Buff(b) => {
                        if thread_rng().gen::<f32>() < b.chance {
                            emit(ServerEvent::Buff {
                                entity: target_entity,
                                buff_change: BuffChange::Add(
                                    b.to_buff(attacker_info.map(|a| a.uid), accumulated_damage),
                                ),
                            });
                        }
                    },
                    CombatEffect::Lifesteal(l) => {
                        if let Some(attacker_entity) = attacker_info.map(|a| a.entity) {
                            let change = HealthChange {
                                amount: (accumulated_damage * l) as i32,
                                cause: HealthSource::Heal {
                                    by: attacker_info.map(|a| a.uid),
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
                        let change = PoiseChange::from_value(p, target_inventory);
                        if change.amount != 0 {
                            emit(ServerEvent::PoiseChange {
                                entity: target_entity,
                                change,
                                kb_dir: *dir,
                            });
                        }
                    },
                    CombatEffect::Heal(h) => {
                        let change = HealthChange {
                            amount: h as i32,
                            cause: HealthSource::Heal {
                                by: attacker_info.map(|a| a.uid),
                            },
                        };
                        if change.amount != 0 {
                            emit(ServerEvent::Damage {
                                entity: target_entity,
                                change,
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
    requirement: Option<CombatRequirement>,
}

#[cfg(not(target_arch = "wasm32"))]
impl AttackEffect {
    pub fn new(target: Option<GroupTarget>, effect: CombatEffect) -> Self {
        Self {
            target,
            effect,
            requirement: None,
        }
    }

    pub fn with_requirement(mut self, requirement: CombatRequirement) -> Self {
        self.requirement = Some(requirement);
        self
    }

    pub fn effect(&self) -> &CombatEffect { &self.effect }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CombatEffect {
    Heal(f32),
    Buff(CombatBuff),
    Knockback(Knockback),
    EnergyReward(f32),
    Lifesteal(f32),
    Poise(f32),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CombatRequirement {
    AnyDamage,
    SufficientEnergy(f32),
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

#[cfg(not(target_arch = "wasm32"))]
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Damage {
    pub source: DamageSource,
    pub value: f32,
}

#[cfg(not(target_arch = "wasm32"))]
impl Damage {
    /// Returns the total damage reduction provided by all equipped items
    pub fn compute_damage_reduction(inventory: &Inventory) -> f32 {
        let protection = inventory
            .equipped_items()
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &item.kind() {
                    Some(armor.get_protection())
                } else {
                    None
                }
            })
            .map(|protection| match protection {
                Protection::Normal(protection) => Some(protection),
                Protection::Invincible => None,
            })
            .sum::<Option<f32>>();

        const FIFTY_PERCENT_DR_THRESHOLD: f32 = 60.0;

        match protection {
            Some(dr) => dr / (FIFTY_PERCENT_DR_THRESHOLD + dr.abs()),
            None => 1.0,
        }
    }

    pub fn calculate_health_change(
        self,
        inventory: Option<&Inventory>,
        uid: Option<Uid>,
        is_crit: bool,
        crit_mult: f32,
        damage_modifier: f32,
    ) -> HealthChange {
        let mut damage = self.value * damage_modifier;
        let damage_reduction = inventory.map_or(0.0, Damage::compute_damage_reduction);
        match self.source {
            DamageSource::Melee => {
                // Critical hit
                let mut critdamage = 0.0;
                if is_crit {
                    critdamage = damage * (crit_mult - 1.0);
                }
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
                if is_crit {
                    damage *= crit_mult;
                }
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
        match self.direction {
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
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CombatBuff {
    pub kind: BuffKind,
    pub dur_secs: f32,
    pub strength: CombatBuffStrength,
    pub chance: f32,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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
        equipped_item_and_tool(inv, EquipSlot::Mainhand).map(|(_, tool)| tool.kind),
        equipped_item_and_tool(inv, EquipSlot::Offhand).map(|(_, tool)| tool.kind),
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn offensive_rating(inv: &Inventory, skillset: &SkillSet) -> f32 {
    let active_damage =
        equipped_item_and_tool(inv, EquipSlot::Mainhand).map_or(0.0, |(item, tool)| {
            tool.base_power(item.components())
                * tool.base_speed(item.components())
                * (1.0 + 0.05 * skillset.earned_sp(SkillGroupKind::Weapon(tool.kind)) as f32)
        });
    let second_damage =
        equipped_item_and_tool(inv, EquipSlot::Offhand).map_or(0.0, |(item, tool)| {
            tool.base_power(item.components())
                * tool.base_speed(item.components())
                * (1.0 + 0.05 * skillset.earned_sp(SkillGroupKind::Weapon(tool.kind)) as f32)
        });
    active_damage.max(second_damage)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn combat_rating(inventory: &Inventory, health: &Health, stats: &Stats, body: Body) -> f32 {
    let defensive_weighting = 1.0;
    let offensive_weighting = 1.0;
    let defensive_rating = health.maximum() as f32
        / (1.0 - Damage::compute_damage_reduction(inventory)).max(0.00001)
        / 100.0;
    let offensive_rating = offensive_rating(inventory, &stats.skill_set).max(0.1)
        + 0.05 * stats.skill_set.earned_sp(SkillGroupKind::General) as f32;
    let combined_rating = (offensive_rating * offensive_weighting
        + defensive_rating * defensive_weighting)
        / (offensive_weighting + defensive_weighting);
    combined_rating * body.combat_multiplier()
}
