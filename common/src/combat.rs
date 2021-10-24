use crate::comp::buff::{Buff, BuffChange, BuffData, BuffKind, BuffSource};
#[cfg(not(target_arch = "wasm32"))]
use crate::{
    comp::{
        inventory::{
            item::{
                armor::Protection,
                tool::{self, Tool, ToolKind},
                Item, ItemDesc, ItemKind, MaterialStatManifest,
            },
            slot::EquipSlot,
        },
        skills::SkillGroupKind,
        Alignment, Body, CharacterState, Combo, Energy, Health, HealthChange, Inventory, Ori,
        Player, Poise, SkillSet, Stats,
    },
    event::ServerEvent,
    outcome::Outcome,
    states::utils::StageSection,
    uid::{Uid, UidAllocator},
    util::Dir,
};

#[cfg(not(target_arch = "wasm32"))]
use rand::{thread_rng, Rng};

use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use specs::{saveload::MarkerAllocator, Entity as EcsEntity, ReadStorage};
#[cfg(not(target_arch = "wasm32"))]
use std::{ops::MulAssign, time::Duration};
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

#[derive(Clone, Copy)]
pub struct AttackOptions {
    pub target_dodging: bool,
    pub may_harm: bool,
    pub target_group: GroupTarget,
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
        mut emit: impl FnMut(ServerEvent),
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
                        emit(ServerEvent::Parry {
                            entity: target.entity,
                            energy_cost: data.static_data.energy_cost,
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
        attacker: Option<AttackerInfo>,
        target: TargetInfo,
        dir: Dir,
        options: AttackOptions,
        // Currently strength_modifier just modifies damage,
        // maybe look into modifying strength of other effects?
        strength_modifier: f32,
        attack_source: AttackSource,
        mut emit: impl FnMut(ServerEvent),
        mut emit_outcome: impl FnMut(Outcome),
    ) -> bool {
        let AttackOptions {
            target_dodging,
            may_harm,
            target_group,
        } = options;

        // target == OutOfGroup is basic heuristic that this
        // "attack" has negative effects.
        //
        // so if target dodges this "attack" or we don't want to harm target,
        // it should avoid such "damage" or effect
        let avoid_damage = |attack_damage: &AttackDamage| {
            matches!(attack_damage.target, Some(GroupTarget::OutOfGroup))
                && (target_dodging || !may_harm)
        };
        let avoid_effect = |attack_effect: &AttackEffect| {
            matches!(attack_effect.target, Some(GroupTarget::OutOfGroup))
                && (target_dodging || !may_harm)
        };
        let is_crit = thread_rng().gen::<f32>() < self.crit_chance;
        let mut is_applied = false;
        let mut accumulated_damage = 0.0;
        for damage in self
            .damages
            .iter()
            .filter(|d| d.target.map_or(true, |t| t == target_group))
            .filter(|d| !avoid_damage(d))
        {
            is_applied = true;
            let damage_reduction = Attack::compute_damage_reduction(
                &target,
                attack_source,
                dir,
                damage.damage.kind,
                |e| emit(e),
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
            if change.amount.abs() > Health::HEALTH_EPSILON {
                emit(ServerEvent::HealthChange {
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
                                    change: *ec * compute_energy_reward_mod(attacker.inventory),
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
                                    amount: applied_damage * l,
                                    by: attacker.map(|a| a.uid),
                                    cause: None,
                                };
                                if change.amount.abs() > Health::HEALTH_EPSILON {
                                    emit(ServerEvent::HealthChange {
                                        entity: attacker_entity,
                                        change,
                                    });
                                }
                            }
                        },
                        CombatEffect::Poise(p) => {
                            let change = -Poise::apply_poise_reduction(*p, target.inventory);
                            if change.abs() > Poise::POISE_EPSILON {
                                emit(ServerEvent::PoiseChange {
                                    entity: target.entity,
                                    change,
                                    kb_dir: *dir,
                                });
                            }
                        },
                        CombatEffect::Heal(h) => {
                            let change = HealthChange {
                                amount: *h,
                                by: attacker.map(|a| a.uid),
                                cause: None,
                            };
                            if change.amount.abs() > Health::HEALTH_EPSILON {
                                emit(ServerEvent::HealthChange {
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
            .filter(|e| !avoid_effect(e))
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
                        let sufficient_energy = e.current() >= *r;
                        if sufficient_energy {
                            emit(ServerEvent::EnergyChange {
                                entity,
                                change: -*r,
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
                is_applied = true;
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
                                change: ec * compute_energy_reward_mod(attacker.inventory),
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
                                amount: accumulated_damage * l,
                                by: attacker.map(|a| a.uid),
                                cause: None,
                            };
                            if change.amount.abs() > Health::HEALTH_EPSILON {
                                emit(ServerEvent::HealthChange {
                                    entity: attacker_entity,
                                    change,
                                });
                            }
                        }
                    },
                    CombatEffect::Poise(p) => {
                        let change = -Poise::apply_poise_reduction(p, target.inventory);
                        if change.abs() > Poise::POISE_EPSILON {
                            emit(ServerEvent::PoiseChange {
                                entity: target.entity,
                                change,
                                kb_dir: *dir,
                            });
                        }
                    },
                    CombatEffect::Heal(h) => {
                        let change = HealthChange {
                            amount: h,
                            by: attacker.map(|a| a.uid),
                            cause: None,
                        };
                        if change.amount.abs() > Health::HEALTH_EPSILON {
                            emit(ServerEvent::HealthChange {
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
        // Emits event to handle things that should happen for any successful attack,
        // regardless of if the attack had any damages or effects in it
        if is_applied {
            emit(ServerEvent::EntityAttackedHook {
                entity: target.entity,
            });
        }
        is_applied
    }
}

/// Function that checks for unintentional PvP between players.
///
/// Returns `false` if attack will create unintentional conflict,
/// e.g. if player with PvE mode will harm pets of other players
/// or other players will do the same to such player.
///
/// If both players have PvP mode enabled, interact with NPC and
/// in any other case, this function will return `true`
// TODO: add parameter for doing self-harm?
pub fn may_harm(
    alignments: &ReadStorage<Alignment>,
    players: &ReadStorage<Player>,
    uid_allocator: &UidAllocator,
    attacker: Option<EcsEntity>,
    target: EcsEntity,
) -> bool {
    // Return owner entity if pet,
    // or just return entity back otherwise
    let owner_if_pet = |entity| {
        let alignment = alignments.get(entity).copied();
        if let Some(Alignment::Owned(uid)) = alignment {
            // return original entity
            // if can't get owner
            uid_allocator
                .retrieve_entity_internal(uid.into())
                .unwrap_or(entity)
        } else {
            entity
        }
    };

    // Just return ok if attacker is unknown, it's probably
    // environment or command.
    let attacker = match attacker {
        Some(attacker) => attacker,
        None => return true,
    };

    // "Dereference" to owner if this is a pet.
    let attacker = owner_if_pet(attacker);
    let target = owner_if_pet(target);

    // Get player components
    let attacker_info = players.get(attacker);
    let target_info = players.get(target);

    // Return `true` if not players.
    attacker_info
        .zip(target_info)
        .map_or(true, |(a, t)| a.may_harm(t))
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
        let critdamage = if is_crit {
            damage * (crit_mult - 1.0)
        } else {
            0.0
        };
        match self.source {
            DamageSource::Melee
            | DamageSource::Projectile
            | DamageSource::Explosion
            | DamageSource::Shockwave
            | DamageSource::Energy => {
                // Critical hit
                damage += critdamage;
                // Armor
                damage *= 1.0 - damage_reduction;

                HealthChange {
                    amount: -damage,
                    by: uid,
                    cause: Some(self.source),
                }
            },
            DamageSource::Falling => {
                // Armor
                if (damage_reduction - 1.0).abs() < f32::EPSILON {
                    damage = 0.0;
                }
                HealthChange {
                    amount: -damage,
                    by: None,
                    cause: Some(self.source),
                }
            },
            DamageSource::Buff(_) | DamageSource::Other => HealthChange {
                amount: -damage,
                by: None,
                cause: Some(self.source),
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

impl MulAssign<f32> for CombatBuffStrength {
    fn mul_assign(&mut self, mul: f32) {
        match self {
            Self::DamageFraction(ref mut val) | Self::Value(ref mut val) => {
                *val *= mul;
            },
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

#[cfg(not(target_arch = "wasm32"))]
pub fn weapon_rating<T: ItemDesc>(item: &T, msm: &MaterialStatManifest) -> f32 {
    const DAMAGE_WEIGHT: f32 = 2.0;
    const EFFECT_WEIGHT: f32 = 1.0;

    if let ItemKind::Tool(tool) = item.kind() {
        let stats = tool::Stats::from((msm, item.components(), tool));

        // TODO: Look into changing the 0.5 to reflect armor later maybe?
        // Since it is only for weapon though, it probably makes sense to leave
        // independent for now
        let damage_rating = stats.power * stats.speed * (1.0 + stats.crit_chance * 0.5);
        let effect_rating = stats.effect_power * stats.speed;

        (damage_rating * DAMAGE_WEIGHT + effect_rating * EFFECT_WEIGHT)
            / (DAMAGE_WEIGHT + EFFECT_WEIGHT)
    } else {
        0.0
    }
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
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
    energy: &Energy,
    poise: &Poise,
    skill_set: &SkillSet,
    body: Body,
    msm: &MaterialStatManifest,
) -> f32 {
    const WEAPON_WEIGHT: f32 = 1.0;
    const HEALTH_WEIGHT: f32 = 1.5;
    const ENERGY_WEIGHT: f32 = 0.5;
    const SKILLS_WEIGHT: f32 = 1.0;
    const POISE_WEIGHT: f32 = 0.5;
    const CRIT_WEIGHT: f32 = 0.5;
    // Normalzied with a standard max health of 100
    let health_rating = health.base_max()
        / 100.0
        / (1.0 - Damage::compute_damage_reduction(Some(inventory), None, None)).max(0.00001);

    // Normalzied with a standard max energy of 100 and energy reward multiplier of
    // x1
    let energy_rating = (energy.base_max() + compute_max_energy_mod(Some(inventory))) / 100.0
        * compute_energy_reward_mod(Some(inventory));

    // Normalzied with a standard max poise of 100
    let poise_rating = poise.base_max() as f32
        / 100.0
        / (1.0 - Poise::compute_poise_damage_reduction(inventory)).max(0.00001);

    // Normalzied with a standard crit multiplier of 1.2
    let crit_rating = compute_crit_mult(Some(inventory)) / 1.2;

    // Assumes a standard person has earned 20 skill points in the general skill
    // tree and 10 skill points for the weapon skill tree
    let skills_rating = (skill_set.earned_sp(SkillGroupKind::General) as f32 / 20.0
        + weapon_skills(inventory, skill_set) / 10.0)
        / 2.0;

    let weapon_rating = get_weapon_rating(inventory, msm);

    let combined_rating = (health_rating * HEALTH_WEIGHT
        + energy_rating * ENERGY_WEIGHT
        + poise_rating * POISE_WEIGHT
        + crit_rating * CRIT_WEIGHT
        + skills_rating * SKILLS_WEIGHT
        + weapon_rating * WEAPON_WEIGHT)
        / (HEALTH_WEIGHT
            + ENERGY_WEIGHT
            + POISE_WEIGHT
            + CRIT_WEIGHT
            + SKILLS_WEIGHT
            + WEAPON_WEIGHT);

    // Body multiplier meant to account for an enemy being harder than equipment and
    // skills would account for. It should only not be 1.0 for non-humanoids
    combined_rating * body.combat_multiplier()
}

#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
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

/// Computes the additive modifier that should be applied to max energy from the
/// currently equipped items
#[cfg(not(target_arch = "wasm32"))]
pub fn compute_max_energy_mod(inventory: Option<&Inventory>) -> f32 {
    // Defaults to a value of 0 if no inventory is present
    inventory.map_or(0.0, |inv| {
        inv.equipped_items()
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &item.kind() {
                    Some(armor.energy_max())
                } else {
                    None
                }
            })
            .sum()
    })
}

/// Computes the sneak coefficient from armor. Agent perception distances are
/// divided by the resulting f32.
#[cfg(not(target_arch = "wasm32"))]
pub fn compute_stealth_coefficient(inventory: Option<&Inventory>) -> f32 {
    // Starts with a value of 2.0 when summing the stats from each armor piece, and
    // defaults to a value of 2.0 if no inventory is equipped
    inventory.map_or(2.0, |inv| {
        inv.equipped_items()
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &item.kind() {
                    Some(armor.stealth())
                } else {
                    None
                }
            })
            .fold(2.0, |a, b| a + b.max(0.0))
    })
}
