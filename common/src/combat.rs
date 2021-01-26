use crate::{
    comp::{
        inventory::{
            item::{
                armor::Protection,
                tool::{Tool, ToolKind},
                ItemKind,
            },
            slot::EquipSlot,
        },
        skills::{SkillGroupKind, SkillSet},
        Body, BuffKind, Health, HealthChange, HealthSource, Inventory, Stats,
    },
    effect,
    uid::Uid,
    util::Dir,
};
use serde::{Deserialize, Serialize};
use vek::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GroupTarget {
    InGroup,
    OutOfGroup,
}

pub struct Attack {
    damages: Vec<DamageComponent>,
    effects: Vec<EffectComponent>,
    crit_chance: f32,
    crit_multiplier: f32,
}

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

impl Attack {
    pub fn with_damage(mut self, damage: DamageComponent) -> Self {
        self.damages.push(damage);
        self
    }

    pub fn with_effect(mut self, effect: EffectComponent) -> Self {
        self.effects.push(effect);
        self
    }

    pub fn with_crit(mut self, cc: f32, cm: f32) -> Self {
        self.crit_chance = cc;
        self.crit_multiplier = cm;
        self
    }
}

pub struct DamageComponent {
    damage: Damage,
    target: Option<GroupTarget>,
    effects: Vec<AttackEffect>,
}

impl DamageComponent {
    pub fn new(damage: Damage, target: Option<GroupTarget>) -> Self {
        Self {
            damage,
            target,
            effects: Vec::new(),
        }
    }

    pub fn with_effect(mut self, effect: AttackEffect) -> Self {
        self.effects.push(effect);
        self
    }
}

pub struct EffectComponent {
    target: Option<GroupTarget>,
    effect: AttackEffect,
}

impl EffectComponent {
    pub fn new(target: Option<GroupTarget>, effect: AttackEffect) -> Self {
        Self { target, effect }
    }
}

pub enum AttackEffect {
    Heal(f32),
    Buff(effect::BuffEffect),
    Knockback(Knockback),
    EnergyChange(f32),
    Lifesteal(f32),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DamageSource {
    Buff(BuffKind),
    Melee,
    Healing,
    Projectile,
    Explosion,
    Falling,
    Shockwave,
    Energy,
    Other,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Damage {
    pub source: DamageSource,
    pub value: f32,
}

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
        match protection {
            Some(dr) => dr / (60.0 + dr.abs()),
            None => 1.0,
        }
    }

    pub fn modify_damage(self, inventory: Option<&Inventory>, uid: Option<Uid>) -> HealthChange {
        let mut damage = self.value;
        let damage_reduction = inventory.map_or(0.0, |inv| Damage::compute_damage_reduction(inv));
        match self.source {
            DamageSource::Melee => {
                // Critical hit
                let mut critdamage = 0.0;
                if rand::random() {
                    critdamage = damage * 0.3;
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
                if rand::random() {
                    damage *= 1.2;
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
            DamageSource::Healing => HealthChange {
                amount: damage as i32,
                cause: HealthSource::Heal { by: uid },
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

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Knockback {
    pub direction: KnockbackDir,
    pub strength: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnockbackDir {
    Away,
    Towards,
    Up,
    TowardsUp,
}

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

fn equipped_tool(inv: &Inventory, slot: EquipSlot) -> Option<&Tool> {
    inv.equipped(slot).and_then(|i| {
        if let ItemKind::Tool(tool) = &i.kind() {
            Some(tool)
        } else {
            None
        }
    })
}

pub fn get_weapons(inv: &Inventory) -> (Option<ToolKind>, Option<ToolKind>) {
    (
        equipped_tool(inv, EquipSlot::Mainhand).map(|tool| tool.kind),
        equipped_tool(inv, EquipSlot::Offhand).map(|tool| tool.kind),
    )
}

fn offensive_rating(inv: &Inventory, skillset: &SkillSet) -> f32 {
    let active_damage = equipped_tool(inv, EquipSlot::Mainhand).map_or(0.0, |tool| {
        tool.base_power()
            * tool.base_speed()
            * (1.0 + 0.05 * skillset.earned_sp(SkillGroupKind::Weapon(tool.kind)) as f32)
    });
    let second_damage = equipped_tool(inv, EquipSlot::Offhand).map_or(0.0, |tool| {
        tool.base_power()
            * tool.base_speed()
            * (1.0 + 0.05 * skillset.earned_sp(SkillGroupKind::Weapon(tool.kind)) as f32)
    });
    active_damage.max(second_damage)
}

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
