use crate::{
    comp::{
        ability::Capability,
        buff::{Buff, BuffChange, BuffData, BuffKind, BuffSource},
        inventory::{
            item::{
                armor::Protection,
                tool::{self, ToolKind},
                ItemDesc, ItemKind, MaterialStatManifest,
            },
            slot::EquipSlot,
        },
        skillset::SkillGroupKind,
        Alignment, Body, Buffs, CharacterState, Combo, Energy, Group, Health, HealthChange,
        Inventory, Ori, Player, Poise, PoiseChange, SkillSet, Stats,
    },
    event::{
        BuffEvent, ComboChangeEvent, EmitExt, EnergyChangeEvent, EntityAttackedHookEvent,
        HealthChangeEvent, KnockbackEvent, ParryHookEvent, PoiseChangeEvent,
    },
    outcome::Outcome,
    resources::{Secs, Time},
    states::utils::StageSection,
    uid::{IdMaps, Uid},
    util::Dir,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use specs::{Entity as EcsEntity, ReadStorage};
use std::ops::{Mul, MulAssign};
use vek::*;

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
    GroundShockwave,
    AirShockwave,
    UndodgeableShockwave,
    Explosion,
}

pub const FULL_FLANK_ANGLE: f32 = std::f32::consts::PI / 4.0;
pub const PARTIAL_FLANK_ANGLE: f32 = std::f32::consts::PI * 3.0 / 4.0;
// NOTE: Do we want to change this to be a configurable parameter on body?
pub const PROJECTILE_HEADSHOT_PROPORTION: f32 = 0.1;
pub const BEAM_DURATION_PRECISION: f32 = 2.5;
pub const MAX_BACK_FLANK_PRECISION: f32 = 0.75;
pub const MAX_SIDE_FLANK_PRECISION: f32 = 0.25;
pub const MAX_HEADSHOT_PRECISION: f32 = 1.0;
pub const MAX_TOP_HEADSHOT_PRECISION: f32 = 0.5;
pub const MAX_BEAM_DUR_PRECISION: f32 = 0.25;
pub const MAX_MELEE_POISE_PRECISION: f32 = 0.5;

#[derive(Copy, Clone)]
pub struct AttackerInfo<'a> {
    pub entity: EcsEntity,
    pub uid: Uid,
    pub group: Option<&'a Group>,
    pub energy: Option<&'a Energy>,
    pub combo: Option<&'a Combo>,
    pub inventory: Option<&'a Inventory>,
    pub stats: Option<&'a Stats>,
}

pub struct TargetInfo<'a> {
    pub entity: EcsEntity,
    pub uid: Uid,
    pub inventory: Option<&'a Inventory>,
    pub stats: Option<&'a Stats>,
    pub health: Option<&'a Health>,
    pub pos: Vec3<f32>,
    pub ori: Option<&'a Ori>,
    pub char_state: Option<&'a CharacterState>,
    pub energy: Option<&'a Energy>,
    pub buffs: Option<&'a Buffs>,
}

#[derive(Clone, Copy)]
pub struct AttackOptions {
    pub target_dodging: bool,
    pub may_harm: bool,
    pub target_group: GroupTarget,
    pub precision_mult: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)] // TODO: Yeet clone derive
pub struct Attack {
    damages: Vec<AttackDamage>,
    effects: Vec<AttackEffect>,
    precision_multiplier: f32,
}

impl Default for Attack {
    fn default() -> Self {
        Self {
            damages: Vec::new(),
            effects: Vec::new(),
            precision_multiplier: 1.0,
        }
    }
}

impl Attack {
    #[must_use]
    pub fn with_damage(mut self, damage: AttackDamage) -> Self {
        self.damages.push(damage);
        self
    }

    #[must_use]
    pub fn with_effect(mut self, effect: AttackEffect) -> Self {
        self.effects.push(effect);
        self
    }

    #[must_use]
    pub fn with_precision(mut self, precision_multiplier: f32) -> Self {
        self.precision_multiplier = precision_multiplier;
        self
    }

    #[must_use]
    pub fn with_combo(self, combo: i32) -> Self {
        self.with_effect(
            AttackEffect::new(None, CombatEffect::Combo(combo))
                .with_requirement(CombatRequirement::AnyDamage),
        )
    }

    #[must_use]
    pub fn with_combo_increment(self) -> Self { self.with_combo(1) }

    pub fn effects(&self) -> impl Iterator<Item = &AttackEffect> { self.effects.iter() }

    pub fn compute_damage_reduction(
        attacker: Option<&AttackerInfo>,
        target: &TargetInfo,
        source: AttackSource,
        dir: Dir,
        damage: Damage,
        msm: &MaterialStatManifest,
        emitters: &mut impl EmitExt<ParryHookEvent>,
        mut emit_outcome: impl FnMut(Outcome),
    ) -> f32 {
        if damage.value > 0.0 {
            let attacker_penetration = attacker
                .and_then(|a| a.stats)
                .map_or(0.0, |s| s.mitigations_penetration)
                .clamp(0.0, 1.0);
            let raw_damage_reduction =
                Damage::compute_damage_reduction(Some(damage), target.inventory, target.stats, msm);
            let damage_reduction = (1.0 - attacker_penetration) * raw_damage_reduction;
            let block_reduction =
                if let (Some(char_state), Some(ori)) = (target.char_state, target.ori) {
                    if ori.look_vec().angle_between(-dir.with_z(0.0)) < char_state.block_angle() {
                        if char_state.is_parry(source) {
                            emit_outcome(Outcome::Block {
                                parry: true,
                                pos: target.pos,
                                uid: target.uid,
                            });
                            emitters.emit(ParryHookEvent {
                                defender: target.entity,
                                attacker: attacker.map(|a| a.entity),
                                source,
                            });
                            1.0
                        } else if let Some(block_strength) = char_state.block_strength(source) {
                            emit_outcome(Outcome::Block {
                                parry: false,
                                pos: target.pos,
                                uid: target.uid,
                            });
                            block_strength
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };
            1.0 - (1.0 - damage_reduction) * (1.0 - block_reduction)
        } else {
            0.0
        }
    }

    pub fn apply_attack(
        &self,
        attacker: Option<AttackerInfo>,
        target: &TargetInfo,
        dir: Dir,
        options: AttackOptions,
        // Currently strength_modifier just modifies damage,
        // maybe look into modifying strength of other effects?
        strength_modifier: f32,
        attack_source: AttackSource,
        time: Time,
        emitters: &mut (
                 impl EmitExt<HealthChangeEvent>
                 + EmitExt<EnergyChangeEvent>
                 + EmitExt<ParryHookEvent>
                 + EmitExt<KnockbackEvent>
                 + EmitExt<BuffEvent>
                 + EmitExt<PoiseChangeEvent>
                 + EmitExt<ComboChangeEvent>
                 + EmitExt<EntityAttackedHookEvent>
             ),
        mut emit_outcome: impl FnMut(Outcome),
        rng: &mut rand::rngs::ThreadRng,
        damage_instance_offset: u64,
    ) -> bool {
        // TODO: Maybe move this higher and pass it as argument into this function?
        let msm = &MaterialStatManifest::load().read();

        let AttackOptions {
            target_dodging,
            may_harm,
            target_group,
            precision_mult,
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
        let precision_mult = attacker
            .and_then(|a| a.stats)
            .and_then(|s| s.precision_multiplier_override)
            .or(precision_mult);
        let mut is_applied = false;
        let mut accumulated_damage = 0.0;
        let damage_modifier = attacker
            .and_then(|a| a.stats)
            .map_or(1.0, |s| s.attack_damage_modifier);
        for damage in self
            .damages
            .iter()
            .filter(|d| d.target.map_or(true, |t| t == target_group))
            .filter(|d| !avoid_damage(d))
        {
            let damage_instance = damage.instance + damage_instance_offset;
            is_applied = true;
            let damage_reduction = Attack::compute_damage_reduction(
                attacker.as_ref(),
                target,
                attack_source,
                dir,
                damage.damage,
                msm,
                emitters,
                &mut emit_outcome,
            );
            let change = damage.damage.calculate_health_change(
                damage_reduction,
                attacker.map(|x| x.into()),
                precision_mult,
                self.precision_multiplier,
                strength_modifier * damage_modifier,
                time,
                damage_instance,
            );
            let applied_damage = -change.amount;
            accumulated_damage += applied_damage;

            if change.amount.abs() > Health::HEALTH_EPSILON {
                emitters.emit(HealthChangeEvent {
                    entity: target.entity,
                    change,
                });
                match damage.damage.kind {
                    DamageKind::Slashing => {
                        // For slashing damage, reduce target energy by some fraction of applied
                        // damage. When target would lose more energy than they have, deal an
                        // equivalent amount of damage
                        if let Some(target_energy) = target.energy {
                            let energy_change = applied_damage * SLASHING_ENERGY_FRACTION;
                            if energy_change > target_energy.current() {
                                let health_damage = energy_change - target_energy.current();
                                accumulated_damage += health_damage;
                                let health_change = HealthChange {
                                    amount: -health_damage,
                                    by: attacker.map(|x| x.into()),
                                    cause: Some(damage.damage.source),
                                    time,
                                    precise: precision_mult.is_some(),
                                    instance: damage_instance,
                                };
                                emitters.emit(HealthChangeEvent {
                                    entity: target.entity,
                                    change: health_change,
                                });
                            }
                            emitters.emit(EnergyChangeEvent {
                                entity: target.entity,
                                change: -energy_change,
                            });
                        }
                    },
                    DamageKind::Crushing => {
                        // For crushing damage, reduce target poise by some fraction of the amount
                        // of damage that was reduced by target's protection
                        // Damage reduction should never equal 1 here as otherwise the check above
                        // that health change amount is greater than 0 would fail.
                        let reduced_damage =
                            applied_damage * damage_reduction / (1.0 - damage_reduction);
                        let poise = reduced_damage
                            * CRUSHING_POISE_FRACTION
                            * attacker
                                .and_then(|a| a.stats)
                                .map_or(1.0, |s| s.poise_damage_modifier);
                        let change = -Poise::apply_poise_reduction(
                            poise,
                            target.inventory,
                            msm,
                            target.char_state,
                            target.stats,
                        );
                        let poise_change = PoiseChange {
                            amount: change,
                            impulse: *dir,
                            by: attacker.map(|x| x.into()),
                            cause: Some(damage.damage.source),
                            time,
                        };
                        if change.abs() > Poise::POISE_EPSILON {
                            // If target is in a stunned state, apply extra poise damage as health
                            // damage instead
                            if let Some(CharacterState::Stunned(data)) = target.char_state {
                                let health_change =
                                    change * data.static_data.poise_state.damage_multiplier();
                                let health_change = HealthChange {
                                    amount: health_change,
                                    by: attacker.map(|x| x.into()),
                                    cause: Some(damage.damage.source),
                                    instance: damage_instance,
                                    precise: precision_mult.is_some(),
                                    time,
                                };
                                emitters.emit(HealthChangeEvent {
                                    entity: target.entity,
                                    change: health_change,
                                });
                            } else {
                                emitters.emit(PoiseChangeEvent {
                                    entity: target.entity,
                                    change: poise_change,
                                });
                            }
                        }
                    },
                    // Piercing damage ignores some penetration, and is handled when damage
                    // reduction is computed Energy is a placeholder damage type
                    DamageKind::Piercing | DamageKind::Energy => {},
                }
                for effect in damage.effects.iter() {
                    match effect {
                        CombatEffect::Knockback(kb) => {
                            let impulse =
                                kb.calculate_impulse(dir, target.char_state) * strength_modifier;
                            if !impulse.is_approx_zero() {
                                emitters.emit(KnockbackEvent {
                                    entity: target.entity,
                                    impulse,
                                });
                            }
                        },
                        CombatEffect::EnergyReward(ec) => {
                            if let Some(attacker) = attacker {
                                emitters.emit(EnergyChangeEvent {
                                    entity: attacker.entity,
                                    change: *ec
                                        * compute_energy_reward_mod(attacker.inventory, msm)
                                        * strength_modifier
                                        * attacker.stats.map_or(1.0, |s| s.energy_reward_modifier),
                                });
                            }
                        },
                        CombatEffect::Buff(b) => {
                            if rng.gen::<f32>() < b.chance {
                                emitters.emit(BuffEvent {
                                    entity: target.entity,
                                    buff_change: BuffChange::Add(b.to_buff(
                                        time,
                                        attacker.map(|a| a.uid),
                                        target.stats,
                                        applied_damage,
                                        strength_modifier,
                                    )),
                                });
                            }
                        },
                        CombatEffect::Lifesteal(l) => {
                            // Not modified by strength_modifier as damage already is
                            if let Some(attacker_entity) = attacker.map(|a| a.entity) {
                                let change = HealthChange {
                                    amount: applied_damage * l,
                                    by: attacker.map(|a| a.into()),
                                    cause: None,
                                    time,
                                    precise: false,
                                    instance: rand::random(),
                                };
                                if change.amount.abs() > Health::HEALTH_EPSILON {
                                    emitters.emit(HealthChangeEvent {
                                        entity: attacker_entity,
                                        change,
                                    });
                                }
                            }
                        },
                        CombatEffect::Poise(p) => {
                            let change = -Poise::apply_poise_reduction(
                                *p,
                                target.inventory,
                                msm,
                                target.char_state,
                                target.stats,
                            ) * strength_modifier
                                * attacker
                                    .and_then(|a| a.stats)
                                    .map_or(1.0, |s| s.poise_damage_modifier);
                            if change.abs() > Poise::POISE_EPSILON {
                                let poise_change = PoiseChange {
                                    amount: change,
                                    impulse: *dir,
                                    by: attacker.map(|x| x.into()),
                                    cause: Some(damage.damage.source),
                                    time,
                                };
                                emitters.emit(PoiseChangeEvent {
                                    entity: target.entity,
                                    change: poise_change,
                                });
                            }
                        },
                        CombatEffect::Heal(h) => {
                            let change = HealthChange {
                                amount: *h * strength_modifier,
                                by: attacker.map(|a| a.into()),
                                cause: None,
                                time,
                                precise: false,
                                instance: rand::random(),
                            };
                            if change.amount.abs() > Health::HEALTH_EPSILON {
                                emitters.emit(HealthChangeEvent {
                                    entity: target.entity,
                                    change,
                                });
                            }
                        },
                        CombatEffect::Combo(c) => {
                            // Not affected by strength modifier as integer
                            if let Some(attacker_entity) = attacker.map(|a| a.entity) {
                                emitters.emit(ComboChangeEvent {
                                    entity: attacker_entity,
                                    change: *c,
                                });
                            }
                        },
                        CombatEffect::StageVulnerable(damage, section) => {
                            if target
                                .char_state
                                .map_or(false, |cs| cs.stage_section() == Some(*section))
                            {
                                let change = {
                                    let mut change = change;
                                    change.amount *= damage;
                                    change
                                };
                                emitters.emit(HealthChangeEvent {
                                    entity: target.entity,
                                    change,
                                });
                            }
                        },
                        CombatEffect::RefreshBuff(chance, b) => {
                            if rng.gen::<f32>() < *chance {
                                emitters.emit(BuffEvent {
                                    entity: target.entity,
                                    buff_change: BuffChange::Refresh(*b),
                                });
                            }
                        },
                        CombatEffect::BuffsVulnerable(damage, buff) => {
                            if target.buffs.map_or(false, |b| b.contains(*buff)) {
                                let change = {
                                    let mut change = change;
                                    change.amount *= damage;
                                    change
                                };
                                emitters.emit(HealthChangeEvent {
                                    entity: target.entity,
                                    change,
                                });
                            }
                        },
                        CombatEffect::StunnedVulnerable(damage) => {
                            if target.char_state.map_or(false, |cs| cs.is_stunned()) {
                                let change = {
                                    let mut change = change;
                                    change.amount *= damage;
                                    change
                                };
                                emitters.emit(HealthChangeEvent {
                                    entity: target.entity,
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
            .chain(
                attacker
                    .and_then(|attacker| attacker.stats)
                    .iter()
                    .flat_map(|stats| stats.effects_on_attack.iter()),
            )
            .filter(|e| e.target.map_or(true, |t| t == target_group))
            .filter(|e| !avoid_effect(e))
        {
            let requirements_met = effect.requirements.iter().all(|req| match req {
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
                            emitters.emit(EnergyChangeEvent {
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
                            emitters.emit(ComboChangeEvent {
                                entity,
                                change: -(*r as i32),
                            });
                        }

                        sufficient_combo
                    } else {
                        false
                    }
                },
                CombatRequirement::TargetHasBuff(buff) => {
                    target.buffs.map_or(false, |buffs| buffs.contains(*buff))
                },
            });
            if requirements_met {
                is_applied = true;
                match effect.effect {
                    CombatEffect::Knockback(kb) => {
                        let impulse =
                            kb.calculate_impulse(dir, target.char_state) * strength_modifier;
                        if !impulse.is_approx_zero() {
                            emitters.emit(KnockbackEvent {
                                entity: target.entity,
                                impulse,
                            });
                        }
                    },
                    CombatEffect::EnergyReward(ec) => {
                        if let Some(attacker) = attacker {
                            emitters.emit(EnergyChangeEvent {
                                entity: attacker.entity,
                                change: ec
                                    * compute_energy_reward_mod(attacker.inventory, msm)
                                    * strength_modifier
                                    * attacker.stats.map_or(1.0, |s| s.energy_reward_modifier),
                            });
                        }
                    },
                    CombatEffect::Buff(b) => {
                        if rng.gen::<f32>() < b.chance {
                            emitters.emit(BuffEvent {
                                entity: target.entity,
                                buff_change: BuffChange::Add(b.to_buff(
                                    time,
                                    attacker.map(|a| a.uid),
                                    target.stats,
                                    accumulated_damage,
                                    strength_modifier,
                                )),
                            });
                        }
                    },
                    CombatEffect::Lifesteal(l) => {
                        // Not modified by strength_modifier as damage already is
                        if let Some(attacker_entity) = attacker.map(|a| a.entity) {
                            let change = HealthChange {
                                amount: accumulated_damage * l,
                                by: attacker.map(|a| a.into()),
                                cause: None,
                                time,
                                precise: false,
                                instance: rand::random(),
                            };
                            if change.amount.abs() > Health::HEALTH_EPSILON {
                                emitters.emit(HealthChangeEvent {
                                    entity: attacker_entity,
                                    change,
                                });
                            }
                        }
                    },
                    CombatEffect::Poise(p) => {
                        let change = -Poise::apply_poise_reduction(
                            p,
                            target.inventory,
                            msm,
                            target.char_state,
                            target.stats,
                        ) * strength_modifier
                            * attacker
                                .and_then(|a| a.stats)
                                .map_or(1.0, |s| s.poise_damage_modifier);
                        if change.abs() > Poise::POISE_EPSILON {
                            let poise_change = PoiseChange {
                                amount: change,
                                impulse: *dir,
                                by: attacker.map(|x| x.into()),
                                cause: Some(attack_source.into()),
                                time,
                            };
                            emitters.emit(PoiseChangeEvent {
                                entity: target.entity,
                                change: poise_change,
                            });
                        }
                    },
                    CombatEffect::Heal(h) => {
                        let change = HealthChange {
                            amount: h * strength_modifier,
                            by: attacker.map(|a| a.into()),
                            cause: None,
                            time,
                            precise: false,
                            instance: rand::random(),
                        };
                        if change.amount.abs() > Health::HEALTH_EPSILON {
                            emitters.emit(HealthChangeEvent {
                                entity: target.entity,
                                change,
                            });
                        }
                    },
                    CombatEffect::Combo(c) => {
                        // Not affected by strength modifier as integer
                        if let Some(attacker_entity) = attacker.map(|a| a.entity) {
                            emitters.emit(ComboChangeEvent {
                                entity: attacker_entity,
                                change: c,
                            });
                        }
                    },
                    // Only has an effect when attached to a damage
                    CombatEffect::StageVulnerable(_, _) => {},
                    CombatEffect::RefreshBuff(chance, b) => {
                        if rng.gen::<f32>() < chance {
                            emitters.emit(BuffEvent {
                                entity: target.entity,
                                buff_change: BuffChange::Refresh(b),
                            });
                        }
                    },
                    // Only has an effect when attached to a damage
                    CombatEffect::BuffsVulnerable(_, _) => {},
                    CombatEffect::StunnedVulnerable(_) => {},
                }
            }
        }
        // Emits event to handle things that should happen for any successful attack,
        // regardless of if the attack had any damages or effects in it
        if is_applied {
            emitters.emit(EntityAttackedHookEvent {
                entity: target.entity,
                attacker: attacker.map(|a| a.entity),
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
    id_maps: &IdMaps,
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
            id_maps.uid_entity(uid).unwrap_or(entity)
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttackDamage {
    damage: Damage,
    target: Option<GroupTarget>,
    effects: Vec<CombatEffect>,
    /// A random ID, used to group up attacks
    instance: u64,
}

impl AttackDamage {
    pub fn new(damage: Damage, target: Option<GroupTarget>, instance: u64) -> Self {
        Self {
            damage,
            target,
            effects: Vec::new(),
            instance,
        }
    }

    #[must_use]
    pub fn with_effect(mut self, effect: CombatEffect) -> Self {
        self.effects.push(effect);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AttackEffect {
    target: Option<GroupTarget>,
    effect: CombatEffect,
    requirements: Vec<CombatRequirement>,
}

impl AttackEffect {
    pub fn new(target: Option<GroupTarget>, effect: CombatEffect) -> Self {
        Self {
            target,
            effect,
            requirements: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_requirement(mut self, requirement: CombatRequirement) -> Self {
        self.requirements.push(requirement);
        self
    }

    pub fn effect(&self) -> &CombatEffect { &self.effect }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum CombatEffect {
    Heal(f32),
    Buff(CombatBuff),
    Knockback(Knockback),
    EnergyReward(f32),
    Lifesteal(f32),
    Poise(f32),
    Combo(i32),
    /// If the attack hits the target while they are in the buildup portion of a
    /// character state, deal increased damage
    /// Only has an effect when attached to a damage, otherwise does nothing if
    /// only attached to the attack
    // TODO: Maybe try to make it do something if tied to
    // attack, not sure if it should double count in that instance?
    StageVulnerable(f32, StageSection),
    /// Resets duration of all buffs of this buffkind, with some probability
    RefreshBuff(f32, BuffKind),
    /// If the target hit by an attack has this buff, they will take increased
    /// damage.
    /// Only has an effect when attached to a damage, otherwise does nothing if
    /// only attached to the attack
    // TODO: Maybe try to make it do something if tied to attack, not sure if it should double
    // count in that instance?
    BuffsVulnerable(f32, BuffKind),
    /// If the target hit by an attack is in a stunned state, they will take
    /// increased damage.
    /// Only has an effect when attached to a damage, otherwise does nothing if
    /// only attached to the attack
    // TODO: Maybe try to make it do something if tied to attack, not sure if it should double
    // count in that instance?
    StunnedVulnerable(f32),
}

impl CombatEffect {
    pub fn adjusted_by_stats(self, stats: tool::Stats) -> Self {
        match self {
            CombatEffect::Heal(h) => CombatEffect::Heal(h * stats.effect_power),
            CombatEffect::Buff(CombatBuff {
                kind,
                dur_secs,
                strength,
                chance,
            }) => CombatEffect::Buff(CombatBuff {
                kind,
                dur_secs,
                strength: strength * stats.buff_strength,
                chance,
            }),
            CombatEffect::Knockback(Knockback {
                direction,
                strength,
            }) => CombatEffect::Knockback(Knockback {
                direction,
                strength: strength * stats.effect_power,
            }),
            CombatEffect::EnergyReward(e) => CombatEffect::EnergyReward(e),
            CombatEffect::Lifesteal(l) => CombatEffect::Lifesteal(l * stats.effect_power),
            CombatEffect::Poise(p) => CombatEffect::Poise(p * stats.effect_power),
            CombatEffect::Combo(c) => CombatEffect::Combo(c),
            CombatEffect::StageVulnerable(v, s) => {
                CombatEffect::StageVulnerable(v * stats.effect_power, s)
            },
            CombatEffect::RefreshBuff(c, b) => CombatEffect::RefreshBuff(c, b),
            CombatEffect::BuffsVulnerable(v, b) => {
                CombatEffect::BuffsVulnerable(v * stats.effect_power, b)
            },
            CombatEffect::StunnedVulnerable(v) => {
                CombatEffect::StunnedVulnerable(v * stats.effect_power)
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CombatRequirement {
    AnyDamage,
    Energy(f32),
    Combo(u32),
    TargetHasBuff(BuffKind),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DamagedEffect {
    Combo(i32),
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum DamageContributor {
    Solo(Uid),
    Group { entity_uid: Uid, group: Group },
}

impl DamageContributor {
    pub fn new(uid: Uid, group: Option<Group>) -> Self {
        if let Some(group) = group {
            DamageContributor::Group {
                entity_uid: uid,
                group,
            }
        } else {
            DamageContributor::Solo(uid)
        }
    }

    pub fn uid(&self) -> Uid {
        match self {
            DamageContributor::Solo(uid) => *uid,
            DamageContributor::Group {
                entity_uid,
                group: _,
            } => *entity_uid,
        }
    }
}

impl From<AttackerInfo<'_>> for DamageContributor {
    fn from(attacker_info: AttackerInfo) -> Self {
        DamageContributor::new(attacker_info.uid, attacker_info.group.copied())
    }
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

impl From<AttackSource> for DamageSource {
    fn from(attack: AttackSource) -> Self {
        match attack {
            AttackSource::Melee => DamageSource::Melee,
            AttackSource::Projectile => DamageSource::Projectile,
            AttackSource::Explosion => DamageSource::Explosion,
            AttackSource::AirShockwave
            | AttackSource::GroundShockwave
            | AttackSource::UndodgeableShockwave => DamageSource::Shockwave,
            AttackSource::Beam => DamageSource::Energy,
        }
    }
}

/// DamageKind for the purpose of differentiating damage reduction
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageKind {
    /// Bypasses some protection from armor
    Piercing,
    /// Reduces energy of target, dealing additional damage when target energy
    /// is 0
    Slashing,
    /// Deals additional poise damage the more armored the target is
    Crushing,
    /// Catch all for remaining damage kinds (TODO: differentiate further with
    /// staff/sceptre reworks
    Energy,
}

const PIERCING_PENETRATION_FRACTION: f32 = 0.5;
const SLASHING_ENERGY_FRACTION: f32 = 0.5;
const CRUSHING_POISE_FRACTION: f32 = 1.0;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Damage {
    pub source: DamageSource,
    pub kind: DamageKind,
    pub value: f32,
}

impl Damage {
    /// Returns the total damage reduction provided by all equipped items
    pub fn compute_damage_reduction(
        damage: Option<Self>,
        inventory: Option<&Inventory>,
        stats: Option<&Stats>,
        msm: &MaterialStatManifest,
    ) -> f32 {
        let protection = compute_protection(inventory, msm);

        let penetration = if let Some(damage) = damage {
            if let DamageKind::Piercing = damage.kind {
                (damage.value * PIERCING_PENETRATION_FRACTION).clamp(0.0, protection.unwrap_or(0.0))
            } else {
                0.0
            }
        } else {
            0.0
        };

        let protection = protection.map(|p| p - penetration);

        const FIFTY_PERCENT_DR_THRESHOLD: f32 = 60.0;

        let inventory_dr = match protection {
            Some(dr) => dr / (FIFTY_PERCENT_DR_THRESHOLD + dr.abs()),
            None => 1.0,
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
        damage_contributor: Option<DamageContributor>,
        precision_mult: Option<f32>,
        precision_power: f32,
        damage_modifier: f32,
        time: Time,
        instance: u64,
    ) -> HealthChange {
        let mut damage = self.value * damage_modifier;
        let precise_damage = damage * precision_mult.unwrap_or(0.0) * (precision_power - 1.0);
        match self.source {
            DamageSource::Melee
            | DamageSource::Projectile
            | DamageSource::Explosion
            | DamageSource::Shockwave
            | DamageSource::Energy => {
                // Precise hit
                damage += precise_damage;
                // Armor
                damage *= 1.0 - damage_reduction;

                HealthChange {
                    amount: -damage,
                    by: damage_contributor,
                    cause: Some(self.source),
                    time,
                    precise: precision_mult.is_some(),
                    instance,
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
                    time,
                    precise: false,
                    instance,
                }
            },
            DamageSource::Buff(_) | DamageSource::Other => HealthChange {
                amount: -damage,
                by: None,
                cause: Some(self.source),
                time,
                precise: false,
                instance,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnockbackDir {
    Away,
    Towards,
    Up,
    TowardsUp,
}

impl Knockback {
    pub fn calculate_impulse(self, dir: Dir, char_state: Option<&CharacterState>) -> Vec3<f32> {
        let from_char = {
            let resistant = char_state
                .and_then(|cs| cs.ability_info())
                .map(|a| a.ability_meta)
                .map_or(false, |a| {
                    a.capabilities.contains(Capability::KNOCKBACK_RESISTANT)
                });
            if resistant { 0.5 } else { 1.0 }
        };
        // TEMP: 50.0 multiplication kept until source knockback values have been
        // updated
        50.0 * self.strength
            * from_char
            * match self.direction {
                KnockbackDir::Away => *Dir::slerp(dir, Dir::new(Vec3::unit_z()), 0.5),
                KnockbackDir::Towards => *Dir::slerp(-dir, Dir::new(Vec3::unit_z()), 0.5),
                KnockbackDir::Up => Vec3::unit_z(),
                KnockbackDir::TowardsUp => *Dir::slerp(-dir, Dir::new(Vec3::unit_z()), 0.85),
            }
    }

    #[must_use]
    pub fn modify_strength(mut self, power: f32) -> Self {
        self.strength *= power;
        self
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatBuff {
    pub kind: BuffKind,
    pub dur_secs: f32,
    pub strength: CombatBuffStrength,
    pub chance: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum CombatBuffStrength {
    DamageFraction(f32),
    Value(f32),
}

impl CombatBuffStrength {
    fn to_strength(self, damage: f32, strength_modifier: f32) -> f32 {
        match self {
            // Not affected by strength modifier as damage already is
            CombatBuffStrength::DamageFraction(f) => damage * f,
            CombatBuffStrength::Value(v) => v * strength_modifier,
        }
    }
}

impl MulAssign<f32> for CombatBuffStrength {
    fn mul_assign(&mut self, mul: f32) { *self = *self * mul; }
}

impl Mul<f32> for CombatBuffStrength {
    type Output = Self;

    fn mul(self, mult: f32) -> Self {
        match self {
            Self::DamageFraction(val) => Self::DamageFraction(val * mult),
            Self::Value(val) => Self::Value(val * mult),
        }
    }
}

impl CombatBuff {
    fn to_buff(
        self,
        time: Time,
        uid: Option<Uid>,
        tgt_stats: Option<&Stats>,
        damage: f32,
        strength_modifier: f32,
    ) -> Buff {
        // TODO: Generate BufCategoryId vec (probably requires damage overhaul?)
        let source = if let Some(uid) = uid {
            BuffSource::Character { by: uid }
        } else {
            BuffSource::Unknown
        };
        Buff::new(
            self.kind,
            BuffData::new(
                self.strength.to_strength(damage, strength_modifier),
                Some(Secs(self.dur_secs as f64)),
            ),
            Vec::new(),
            source,
            time,
            tgt_stats,
        )
    }
}

pub fn get_weapon_kinds(inv: &Inventory) -> (Option<ToolKind>, Option<ToolKind>) {
    (
        inv.equipped(EquipSlot::ActiveMainhand).and_then(|i| {
            if let ItemKind::Tool(tool) = &*i.kind() {
                Some(tool.kind)
            } else {
                None
            }
        }),
        inv.equipped(EquipSlot::ActiveOffhand).and_then(|i| {
            if let ItemKind::Tool(tool) = &*i.kind() {
                Some(tool.kind)
            } else {
                None
            }
        }),
    )
}

// TODO: Either remove msm or use it as argument in fn kind
fn weapon_rating<T: ItemDesc>(item: &T, _msm: &MaterialStatManifest) -> f32 {
    const POWER_WEIGHT: f32 = 2.0;
    const SPEED_WEIGHT: f32 = 3.0;
    const RANGE_WEIGHT: f32 = 0.8;
    const EFFECT_WEIGHT: f32 = 1.5;
    const EQUIP_TIME_WEIGHT: f32 = 0.0;
    const ENERGY_EFFICIENCY_WEIGHT: f32 = 1.5;
    const BUFF_STRENGTH_WEIGHT: f32 = 1.5;

    let rating = if let ItemKind::Tool(tool) = &*item.kind() {
        let stats = tool.stats(item.stats_durability_multiplier());

        // TODO: Look into changing the 0.5 to reflect armor later maybe?
        // Since it is only for weapon though, it probably makes sense to leave
        // independent for now

        let power_rating = stats.power;
        let speed_rating = stats.speed - 1.0;
        let range_rating = stats.range - 1.0;
        let effect_rating = stats.effect_power - 1.0;
        let equip_time_rating = 0.5 - stats.equip_time_secs;
        let energy_efficiency_rating = stats.energy_efficiency - 1.0;
        let buff_strength_rating = stats.buff_strength - 1.0;

        power_rating * POWER_WEIGHT
            + speed_rating * SPEED_WEIGHT
            + range_rating * RANGE_WEIGHT
            + effect_rating * EFFECT_WEIGHT
            + equip_time_rating * EQUIP_TIME_WEIGHT
            + energy_efficiency_rating * ENERGY_EFFICIENCY_WEIGHT
            + buff_strength_rating * BUFF_STRENGTH_WEIGHT
    } else {
        0.0
    };
    rating.max(0.0)
}

fn weapon_skills(inventory: &Inventory, skill_set: &SkillSet) -> f32 {
    let (mainhand, offhand) = get_weapon_kinds(inventory);
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
    let mainhand_rating = if let Some(item) = inventory.equipped(EquipSlot::ActiveMainhand) {
        weapon_rating(item, msm)
    } else {
        0.0
    };

    let offhand_rating = if let Some(item) = inventory.equipped(EquipSlot::ActiveOffhand) {
        weapon_rating(item, msm)
    } else {
        0.0
    };

    mainhand_rating.max(offhand_rating)
}

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
    const PRECISION_WEIGHT: f32 = 0.5;
    // Normalized with a standard max health of 100
    let health_rating = health.base_max()
        / 100.0
        / (1.0 - Damage::compute_damage_reduction(None, Some(inventory), None, msm)).max(0.00001);

    // Normalized with a standard max energy of 100 and energy reward multiplier of
    // x1
    let energy_rating = (energy.base_max() + compute_max_energy_mod(Some(inventory), msm)) / 100.0
        * compute_energy_reward_mod(Some(inventory), msm);

    // Normalized with a standard max poise of 100
    let poise_rating = poise.base_max()
        / 100.0
        / (1.0 - Poise::compute_poise_damage_reduction(Some(inventory), msm, None, None))
            .max(0.00001);

    // Normalized with a standard precision multiplier of 1.2
    let precision_rating = compute_precision_mult(Some(inventory), msm) / 1.2;

    // Assumes a standard person has earned 20 skill points in the general skill
    // tree and 10 skill points for the weapon skill tree
    let skills_rating = (skill_set.earned_sp(SkillGroupKind::General) as f32 / 20.0
        + weapon_skills(inventory, skill_set) / 10.0)
        / 2.0;

    let weapon_rating = get_weapon_rating(inventory, msm);

    let combined_rating = (health_rating * HEALTH_WEIGHT
        + energy_rating * ENERGY_WEIGHT
        + poise_rating * POISE_WEIGHT
        + precision_rating * PRECISION_WEIGHT
        + skills_rating * SKILLS_WEIGHT
        + weapon_rating * WEAPON_WEIGHT)
        / (HEALTH_WEIGHT
            + ENERGY_WEIGHT
            + POISE_WEIGHT
            + PRECISION_WEIGHT
            + SKILLS_WEIGHT
            + WEAPON_WEIGHT);

    // Body multiplier meant to account for an enemy being harder than equipment and
    // skills would account for. It should only not be 1.0 for non-humanoids
    combined_rating * body.combat_multiplier()
}

pub fn compute_precision_mult(inventory: Option<&Inventory>, msm: &MaterialStatManifest) -> f32 {
    // Starts with a value of 0.1 when summing the stats from each armor piece, and
    // defaults to a value of 0.1 if no inventory is equipped. Precision multiplier
    // cannot go below 1
    1.0 + inventory
        .map_or(0.1, |inv| {
            inv.equipped_items()
                .filter_map(|item| {
                    if let ItemKind::Armor(armor) = &*item.kind() {
                        armor
                            .stats(msm, item.stats_durability_multiplier())
                            .precision_power
                    } else {
                        None
                    }
                })
                .fold(0.1, |a, b| a + b)
        })
        .max(0.0)
}

/// Computes the energy reward modifier from worn armor
pub fn compute_energy_reward_mod(inventory: Option<&Inventory>, msm: &MaterialStatManifest) -> f32 {
    // Starts with a value of 1.0 when summing the stats from each armor piece, and
    // defaults to a value of 1.0 if no inventory is present
    inventory.map_or(1.0, |inv| {
        inv.equipped_items()
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &*item.kind() {
                    armor
                        .stats(msm, item.stats_durability_multiplier())
                        .energy_reward
                } else {
                    None
                }
            })
            .fold(1.0, |a, b| a + b)
    })
}

/// Computes the additive modifier that should be applied to max energy from the
/// currently equipped items
pub fn compute_max_energy_mod(inventory: Option<&Inventory>, msm: &MaterialStatManifest) -> f32 {
    // Defaults to a value of 0 if no inventory is present
    inventory.map_or(0.0, |inv| {
        inv.equipped_items()
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &*item.kind() {
                    armor
                        .stats(msm, item.stats_durability_multiplier())
                        .energy_max
                } else {
                    None
                }
            })
            .sum()
    })
}

/// Returns a value to be included as a multiplicative factor in perception
/// distance checks.
pub fn perception_dist_multiplier_from_stealth(
    inventory: Option<&Inventory>,
    character_state: Option<&CharacterState>,
    msm: &MaterialStatManifest,
) -> f32 {
    const SNEAK_MULTIPLIER: f32 = 0.7;

    let item_stealth_multiplier = stealth_multiplier_from_items(inventory, msm);
    let is_sneaking = character_state.map_or(false, |state| state.is_stealthy());

    let multiplier = item_stealth_multiplier * if is_sneaking { SNEAK_MULTIPLIER } else { 1.0 };

    multiplier.clamp(0.0, 1.0)
}

pub fn stealth_multiplier_from_items(
    inventory: Option<&Inventory>,
    msm: &MaterialStatManifest,
) -> f32 {
    let stealth_sum = inventory.map_or(0.0, |inv| {
        inv.equipped_items()
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &*item.kind() {
                    armor.stats(msm, item.stats_durability_multiplier()).stealth
                } else {
                    None
                }
            })
            .sum()
    });

    (1.0 / (1.0 + stealth_sum)).clamp(0.0, 1.0)
}

/// Computes the total protection provided from armor. Is used to determine the
/// damage reduction applied to damage received by an entity None indicates that
/// the armor equipped makes the entity invulnerable
pub fn compute_protection(
    inventory: Option<&Inventory>,
    msm: &MaterialStatManifest,
) -> Option<f32> {
    inventory.map_or(Some(0.0), |inv| {
        inv.equipped_items()
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &*item.kind() {
                    armor
                        .stats(msm, item.stats_durability_multiplier())
                        .protection
                } else {
                    None
                }
            })
            .map(|protection| match protection {
                Protection::Normal(protection) => Some(protection),
                Protection::Invincible => None,
            })
            .sum::<Option<f32>>()
    })
}

/// Used to compute the precision multiplier achieved by flanking a target
pub fn precision_mult_from_flank(attack_dir: Vec3<f32>, target_ori: Option<&Ori>) -> Option<f32> {
    let angle = target_ori.map(|t_ori| t_ori.look_dir().angle_between(attack_dir));
    match angle {
        Some(angle) if angle < FULL_FLANK_ANGLE => Some(MAX_BACK_FLANK_PRECISION),
        Some(angle) if angle < PARTIAL_FLANK_ANGLE => Some(MAX_SIDE_FLANK_PRECISION),
        Some(_) | None => None,
    }
}
