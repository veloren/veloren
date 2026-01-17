use crate::{
    combat::{
        Attack, AttackDamage, AttackEffect, CombatBuff, CombatEffect, CombatRequirement, Damage,
        DamageKind, GroupTarget, Knockback, KnockbackDir,
    },
    comp::{
        ArcProperties, CapsulePrism,
        ability::Dodgeable,
        item::{Reagent, tool},
    },
    consts::GRAVITY,
    explosion::{ColorPreset, Explosion, RadiusEffect},
    resources::{Secs, Time},
    states::utils::AbilityInfo,
    uid::Uid,
    util::Dir,
};
use common_base::dev_panic;
use serde::{Deserialize, Serialize};
use specs::Component;
use std::time::Duration;
use vek::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Effect {
    Attack(Attack),
    Explode(Explosion),
    Vanish,
    Stick,
    Possess,
    Bonk, // Knock/dislodge/change objects on hit
    Firework(Reagent),
    SurpriseEgg,
    TrainingDummy,
    Arc(ArcProperties),
    Split(SplitOptions),
}

#[derive(Clone, Debug)]
pub struct Projectile {
    // TODO: use SmallVec for these effects
    pub hit_solid: Vec<Effect>,
    pub hit_entity: Vec<Effect>,
    pub timeout: Vec<Effect>,
    /// Time left until the projectile will despawn
    pub time_left: Duration,
    /// Max duration of projectile (should be equal to time_left when projectile
    /// is created)
    pub init_time: Secs,
    pub owner: Option<Uid>,
    /// Whether projectile collides with entities in the same group as its
    /// owner
    pub ignore_group: bool,
    /// Whether the projectile is sticky
    pub is_sticky: bool,
    /// Whether the projectile should use a point collider
    pub is_point: bool,
    /// Whether the projectile should home towards a target entity and at what
    /// rate (in deg/s)
    pub homing: Option<(Uid, f32)>,
    /// Whether the projectile should hit and apply its effects to multiple
    /// entities
    pub pierce_entities: bool,
    /// Entities that the projectile has hit (only relevant for projectiles that
    /// can pierce entities)
    pub hit_entities: Vec<Uid>,
    /// Whether to limit the number of projectiles from from an ability can
    /// damage the target in a short duration
    pub limit_per_ability: bool,
    /// Whether to override the collider used by the projectile for detecting
    /// hit entities
    pub override_collider: Option<CapsulePrism>,
}

impl Component for Projectile {
    type Storage = specs::DenseVecStorage<Self>;
}

impl Projectile {
    pub fn is_blockable(&self) -> bool {
        !self.hit_entity.iter().any(|effect| {
            matches!(
                effect,
                Effect::Attack(Attack {
                    blockable: false,
                    ..
                })
            )
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectileConstructor {
    pub kind: ProjectileConstructorKind,
    pub attack: Option<ProjectileAttack>,
    pub scaled: Option<Scaled>,
    /// In degrees per second
    pub homing_rate: Option<f32>,
    pub split: Option<SplitOptions>,
    pub lifetime_override: Option<Secs>,
    #[serde(default)]
    pub limit_per_ability: bool,
    pub override_collider: Option<CapsulePrism>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SplitOptions {
    pub amount: u32,
    pub spread: f32,
    pub new_lifetime: Secs,
    /// If this is used, it will only apply to projectiles created after the
    /// split, and will also override this option on the parent projectile
    pub override_collider: Option<CapsulePrism>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scaled {
    damage: f32,
    poise: Option<f32>,
    knockback: Option<f32>,
    energy: Option<f32>,
    damage_effect: Option<f32>,
}

fn default_true() -> bool { true }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectileAttack {
    pub damage: f32,
    pub poise: Option<f32>,
    pub knockback: Option<f32>,
    pub energy: Option<f32>,
    pub buff: Option<CombatBuff>,
    #[serde(default)]
    pub friendly_fire: bool,
    #[serde(default = "default_true")]
    pub blockable: bool,
    pub damage_effect: Option<CombatEffect>,
    pub attack_effect: Option<(CombatEffect, CombatRequirement)>,
    #[serde(default)]
    pub without_combo: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ProjectileConstructorKind {
    // I want a better name for 'Pointed' and 'Blunt'
    Pointed,
    Blunt,
    Penetrating,
    Explosive {
        radius: f32,
        min_falloff: f32,
        reagent: Option<Reagent>,
        terrain: Option<(f32, ColorPreset)>,
    },
    Arcing {
        distance: f32,
        arcs: u32,
        min_delay: Secs,
        max_delay: Secs,
        #[serde(default)]
        targets_owner: bool,
    },
    Possess,
    Hazard {
        is_sticky: bool,
        duration: Secs,
    },
    ExplosiveHazard {
        radius: f32,
        min_falloff: f32,
        reagent: Option<Reagent>,
        terrain: Option<(f32, ColorPreset)>,
        is_sticky: bool,
        duration: Secs,
    },
    Firework(Reagent),
    SurpriseEgg,
    TrainingDummy,
}

impl ProjectileConstructor {
    pub fn create_projectile(
        self,
        owner: Option<Uid>,
        precision_mult: f32,
        ability_info: Option<AbilityInfo>,
    ) -> Projectile {
        if self.scaled.is_some() {
            dev_panic!(
                "Attempted to create a projectile that had a provided scaled value without \
                 scaling the projectile."
            )
        }

        let instance = rand::random();
        let attack = self.attack.map(|a| {
            let target = if a.friendly_fire {
                Some(GroupTarget::All)
            } else {
                Some(GroupTarget::OutOfGroup)
            };

            let poise = a.poise.map(|poise| {
                AttackEffect::new(target, CombatEffect::Poise(poise))
                    .with_requirement(CombatRequirement::AnyDamage)
            });

            let knockback = a.knockback.map(|kb| {
                AttackEffect::new(
                    target,
                    CombatEffect::Knockback(Knockback {
                        strength: kb,
                        direction: KnockbackDir::Away,
                    }),
                )
                .with_requirement(CombatRequirement::AnyDamage)
            });

            let energy = a.energy.map(|energy| {
                AttackEffect::new(None, CombatEffect::EnergyReward(energy))
                    .with_requirement(CombatRequirement::AnyDamage)
            });

            let buff = a.buff.map(CombatEffect::Buff);

            let damage_kind = match self.kind {
                ProjectileConstructorKind::Pointed
                | ProjectileConstructorKind::Hazard { .. }
                | ProjectileConstructorKind::Penetrating => DamageKind::Piercing,
                ProjectileConstructorKind::Blunt => DamageKind::Crushing,
                ProjectileConstructorKind::Explosive { .. }
                | ProjectileConstructorKind::ExplosiveHazard { .. }
                | ProjectileConstructorKind::Firework(_) => DamageKind::Energy,
                ProjectileConstructorKind::Possess
                | ProjectileConstructorKind::SurpriseEgg
                | ProjectileConstructorKind::TrainingDummy => {
                    dev_panic!("This should be unreachable");
                    DamageKind::Piercing
                },
                ProjectileConstructorKind::Arcing { .. } => DamageKind::Energy,
            };

            let mut damage = AttackDamage::new(
                Damage {
                    kind: damage_kind,
                    value: a.damage,
                },
                target,
                instance,
            );

            if let Some(buff) = buff {
                damage = damage.with_effect(buff);
            }

            if let Some(damage_effect) = a.damage_effect {
                damage = damage.with_effect(damage_effect);
            }

            let mut attack = Attack::new(ability_info)
                .with_damage(damage)
                .with_precision(
                    precision_mult
                        * ability_info
                            .and_then(|ai| ai.ability_meta.precision_power_mult)
                            .unwrap_or(1.0),
                )
                .with_blockable(a.blockable);

            if !a.without_combo {
                attack = attack.with_combo_increment();
            }

            if let Some(poise) = poise {
                attack = attack.with_effect(poise);
            }

            if let Some(knockback) = knockback {
                attack = attack.with_effect(knockback);
            }

            if let Some(energy) = energy {
                attack = attack.with_effect(energy);
            }

            if let Some((effect, requirement)) = a.attack_effect {
                let effect = AttackEffect::new(Some(GroupTarget::OutOfGroup), effect)
                    .with_requirement(requirement);
                attack = attack.with_effect(effect);
            }

            attack
        });

        let homing = ability_info
            .and_then(|a| a.input_attr)
            .and_then(|i| i.target_entity)
            .zip(self.homing_rate);

        let mut timeout = Vec::new();
        let mut hit_solid = Vec::new();

        if let Some(split) = self.split {
            timeout.push(Effect::Split(split));
            hit_solid.push(Effect::Split(split));
        }

        match self.kind {
            ProjectileConstructorKind::Pointed | ProjectileConstructorKind::Blunt => {
                hit_solid.push(Effect::Stick);
                hit_solid.push(Effect::Bonk);

                let mut hit_entity = vec![Effect::Vanish];

                if let Some(attack) = attack {
                    hit_entity.push(Effect::Attack(attack));
                }

                let lifetime = self.lifetime_override.unwrap_or(Secs(15.0));

                Projectile {
                    hit_solid,
                    hit_entity,
                    timeout,
                    time_left: Duration::from_secs_f64(lifetime.0),
                    init_time: lifetime,
                    owner,
                    ignore_group: true,
                    is_sticky: true,
                    is_point: true,
                    homing,
                    pierce_entities: false,
                    hit_entities: Vec::new(),
                    limit_per_ability: self.limit_per_ability,
                    override_collider: self.override_collider,
                }
            },
            ProjectileConstructorKind::Penetrating => {
                hit_solid.push(Effect::Stick);
                hit_solid.push(Effect::Bonk);

                let mut hit_entity = Vec::new();

                if let Some(attack) = attack {
                    hit_entity.push(Effect::Attack(attack));
                }

                let lifetime = self.lifetime_override.unwrap_or(Secs(15.0));

                Projectile {
                    hit_solid,
                    hit_entity,
                    timeout,
                    time_left: Duration::from_secs_f64(lifetime.0),
                    init_time: lifetime,
                    owner,
                    ignore_group: true,
                    is_sticky: true,
                    is_point: true,
                    homing,
                    pierce_entities: true,
                    hit_entities: Vec::new(),
                    limit_per_ability: self.limit_per_ability,
                    override_collider: self.override_collider,
                }
            },
            ProjectileConstructorKind::Hazard {
                is_sticky,
                duration,
            } => {
                hit_solid.push(Effect::Stick);
                hit_solid.push(Effect::Bonk);

                let mut hit_entity = vec![Effect::Vanish];

                if let Some(attack) = attack {
                    hit_entity.push(Effect::Attack(attack));
                }

                let lifetime = self.lifetime_override.unwrap_or(duration);

                Projectile {
                    hit_solid,
                    hit_entity,
                    timeout,
                    time_left: Duration::from_secs_f64(lifetime.0),
                    init_time: lifetime,
                    owner,
                    ignore_group: true,
                    is_sticky,
                    is_point: false,
                    homing,
                    pierce_entities: false,
                    hit_entities: Vec::new(),
                    limit_per_ability: self.limit_per_ability,
                    override_collider: self.override_collider,
                }
            },
            ProjectileConstructorKind::Explosive {
                radius,
                min_falloff,
                reagent,
                terrain,
            } => {
                let terrain =
                    terrain.map(|(pow, col)| RadiusEffect::TerrainDestruction(pow, col.to_rgb()));

                let mut effects = Vec::new();

                if let Some(attack) = attack {
                    effects.push(RadiusEffect::Attack {
                        attack,
                        dodgeable: Dodgeable::Roll,
                    });
                }

                if let Some(terrain) = terrain {
                    effects.push(terrain);
                }

                let explosion = Explosion {
                    effects,
                    radius,
                    reagent,
                    min_falloff,
                };

                hit_solid.push(Effect::Explode(explosion.clone()));
                hit_solid.push(Effect::Vanish);

                let lifetime = self.lifetime_override.unwrap_or(Secs(10.0));

                Projectile {
                    hit_solid,
                    hit_entity: vec![Effect::Explode(explosion), Effect::Vanish],
                    timeout,
                    time_left: Duration::from_secs_f64(lifetime.0),
                    init_time: lifetime,
                    owner,
                    ignore_group: true,
                    is_sticky: true,
                    is_point: true,
                    homing,
                    pierce_entities: false,
                    hit_entities: Vec::new(),
                    limit_per_ability: self.limit_per_ability,
                    override_collider: self.override_collider,
                }
            },
            ProjectileConstructorKind::Arcing {
                distance,
                arcs,
                min_delay,
                max_delay,
                targets_owner,
            } => {
                let mut hit_entity = vec![Effect::Vanish];

                if let Some(attack) = attack {
                    hit_entity.push(Effect::Attack(attack.clone()));

                    let arc = ArcProperties {
                        attack,
                        distance,
                        arcs,
                        min_delay,
                        max_delay,
                        targets_owner,
                    };

                    hit_entity.push(Effect::Arc(arc));
                }

                let lifetime = self.lifetime_override.unwrap_or(Secs(10.0));

                Projectile {
                    hit_solid,
                    hit_entity,
                    timeout,
                    time_left: Duration::from_secs_f64(lifetime.0),
                    init_time: lifetime,
                    owner,
                    ignore_group: true,
                    is_sticky: true,
                    is_point: true,
                    homing,
                    pierce_entities: false,
                    hit_entities: Vec::new(),
                    limit_per_ability: self.limit_per_ability,
                    override_collider: self.override_collider,
                }
            },
            ProjectileConstructorKind::ExplosiveHazard {
                radius,
                min_falloff,
                reagent,
                terrain,
                is_sticky,
                duration,
            } => {
                let terrain =
                    terrain.map(|(pow, col)| RadiusEffect::TerrainDestruction(pow, col.to_rgb()));

                let mut effects = Vec::new();

                if let Some(attack) = attack {
                    effects.push(RadiusEffect::Attack {
                        attack,
                        dodgeable: Dodgeable::Roll,
                    });
                }

                if let Some(terrain) = terrain {
                    effects.push(terrain);
                }

                let explosion = Explosion {
                    effects,
                    radius,
                    reagent,
                    min_falloff,
                };

                let lifetime = self.lifetime_override.unwrap_or(duration);

                Projectile {
                    hit_solid,
                    hit_entity: vec![Effect::Explode(explosion), Effect::Vanish],
                    timeout,
                    time_left: Duration::from_secs_f64(lifetime.0),
                    init_time: lifetime,
                    owner,
                    ignore_group: true,
                    is_sticky,
                    is_point: false,
                    homing,
                    pierce_entities: false,
                    hit_entities: Vec::new(),
                    limit_per_ability: self.limit_per_ability,
                    override_collider: self.override_collider,
                }
            },
            ProjectileConstructorKind::Possess => {
                hit_solid.push(Effect::Stick);

                let lifetime = self.lifetime_override.unwrap_or(Secs(10.0));

                Projectile {
                    hit_solid,
                    hit_entity: vec![Effect::Stick, Effect::Possess],
                    timeout,
                    time_left: Duration::from_secs_f64(lifetime.0),
                    init_time: lifetime,
                    owner,
                    ignore_group: false,
                    is_sticky: true,
                    is_point: true,
                    homing,
                    pierce_entities: false,
                    hit_entities: Vec::new(),
                    limit_per_ability: self.limit_per_ability,
                    override_collider: self.override_collider,
                }
            },
            ProjectileConstructorKind::Firework(reagent) => {
                timeout.push(Effect::Firework(reagent));

                let lifetime = self.lifetime_override.unwrap_or(Secs(3.0));

                Projectile {
                    hit_solid,
                    hit_entity: Vec::new(),
                    timeout,
                    time_left: Duration::from_secs_f64(lifetime.0),
                    init_time: lifetime,
                    owner,
                    ignore_group: true,
                    is_sticky: true,
                    is_point: true,
                    homing,
                    pierce_entities: false,
                    hit_entities: Vec::new(),
                    limit_per_ability: self.limit_per_ability,
                    override_collider: self.override_collider,
                }
            },
            ProjectileConstructorKind::SurpriseEgg => {
                hit_solid.push(Effect::SurpriseEgg);
                hit_solid.push(Effect::Vanish);

                let lifetime = self.lifetime_override.unwrap_or(Secs(15.0));

                Projectile {
                    hit_solid,
                    hit_entity: vec![Effect::SurpriseEgg, Effect::Vanish],
                    timeout,
                    time_left: Duration::from_secs_f64(lifetime.0),
                    init_time: lifetime,
                    owner,
                    ignore_group: true,
                    is_sticky: true,
                    is_point: true,
                    homing,
                    pierce_entities: false,
                    hit_entities: Vec::new(),
                    limit_per_ability: self.limit_per_ability,
                    override_collider: self.override_collider,
                }
            },
            ProjectileConstructorKind::TrainingDummy => {
                hit_solid.push(Effect::TrainingDummy);
                hit_solid.push(Effect::Vanish);

                timeout.push(Effect::TrainingDummy);

                let lifetime = self.lifetime_override.unwrap_or(Secs(15.0));

                Projectile {
                    hit_solid,
                    hit_entity: vec![Effect::TrainingDummy, Effect::Vanish],
                    timeout,
                    time_left: Duration::from_secs_f64(lifetime.0),
                    init_time: lifetime,
                    owner,
                    ignore_group: true,
                    is_sticky: true,
                    is_point: false,
                    homing,
                    pierce_entities: false,
                    hit_entities: Vec::new(),
                    limit_per_ability: self.limit_per_ability,
                    override_collider: self.override_collider,
                }
            },
        }
    }

    pub fn handle_scaling(mut self, scaling: f32) -> Self {
        let scale_values = |a, b| a + b * scaling;

        if let Some(scaled) = self.scaled {
            if let Some(ref mut attack) = self.attack {
                attack.damage = scale_values(attack.damage, scaled.damage);
                if let Some(s_poise) = scaled.poise {
                    attack.poise = Some(scale_values(attack.poise.unwrap_or(0.0), s_poise));
                }
                if let Some(s_kb) = scaled.knockback {
                    attack.knockback = Some(scale_values(attack.knockback.unwrap_or(0.0), s_kb));
                }
                if let Some(s_energy) = scaled.energy {
                    attack.energy = Some(scale_values(attack.energy.unwrap_or(0.0), s_energy));
                }
                if let Some(s_dmg_eff) = scaled.damage_effect {
                    if attack.damage_effect.is_some() {
                        attack.damage_effect =
                            attack.damage_effect.as_ref().cloned().map(|dmg_eff| {
                                dmg_eff.apply_multiplier(scale_values(1.0, s_dmg_eff))
                            });
                    } else {
                        dev_panic!(
                            "Attempted to scale damage effect on a projectile that doesn't have a \
                             damage effect."
                        )
                    }
                }
            } else {
                dev_panic!("Attempted to scale on a projectile that has no attack to scale.")
            }
        } else {
            dev_panic!("Attempted to scale on a projectile that has no provided scaling value.")
        }

        self.scaled = None;

        self
    }

    pub fn adjusted_by_stats(mut self, stats: tool::Stats) -> Self {
        self.attack = self.attack.map(|mut a| {
            a.damage *= stats.power;
            a.poise = a.poise.map(|poise| poise * stats.effect_power);
            a.knockback = a.knockback.map(|kb| kb * stats.effect_power);
            a.buff = a.buff.map(|mut b| {
                b.strength *= stats.buff_strength;
                b
            });
            a.damage_effect = a.damage_effect.map(|de| de.adjusted_by_stats(stats));
            a.attack_effect = a
                .attack_effect
                .map(|(e, r)| (e.adjusted_by_stats(stats), r));
            a
        });

        self.scaled = self.scaled.map(|mut s| {
            s.damage *= stats.power;
            s.poise = s.poise.map(|poise| poise * stats.effect_power);
            s.knockback = s.knockback.map(|kb| kb * stats.effect_power);
            s
        });

        match self.kind {
            ProjectileConstructorKind::Pointed
            | ProjectileConstructorKind::Blunt
            | ProjectileConstructorKind::Penetrating
            | ProjectileConstructorKind::Possess
            | ProjectileConstructorKind::Hazard { .. }
            | ProjectileConstructorKind::Firework(_)
            | ProjectileConstructorKind::SurpriseEgg
            | ProjectileConstructorKind::TrainingDummy => {},
            ProjectileConstructorKind::Explosive { ref mut radius, .. }
            | ProjectileConstructorKind::ExplosiveHazard { ref mut radius, .. } => {
                *radius *= stats.range;
            },
            ProjectileConstructorKind::Arcing {
                ref mut distance, ..
            } => {
                *distance *= stats.range;
            },
        }

        self.split = self.split.map(|mut s| {
            s.amount = (s.amount as f32 * stats.effect_power).ceil().max(0.0) as u32;
            s
        });

        self
    }

    // Remove this function after skill tree overhaul completed for bow and fire
    // staff
    pub fn legacy_modified_by_skills(
        mut self,
        power: f32,
        regen: f32,
        range: f32,
        kb: f32,
    ) -> Self {
        self.attack = self.attack.map(|mut a| {
            a.damage *= power;
            a.knockback = a.knockback.map(|k| k * kb);
            a.energy = a.energy.map(|e| e * regen);
            a
        });
        self.scaled = self.scaled.map(|mut s| {
            s.damage *= power;
            s.knockback = s.knockback.map(|k| k * kb);
            s.energy = s.energy.map(|e| e * regen);
            s
        });
        if let ProjectileConstructorKind::Explosive { ref mut radius, .. } = self.kind {
            *radius *= range;
        }
        self
    }

    pub fn is_explosive(&self) -> bool {
        match self.kind {
            ProjectileConstructorKind::Pointed
            | ProjectileConstructorKind::Blunt
            | ProjectileConstructorKind::Penetrating
            | ProjectileConstructorKind::Possess
            | ProjectileConstructorKind::Hazard { .. }
            | ProjectileConstructorKind::Firework(_)
            | ProjectileConstructorKind::SurpriseEgg
            | ProjectileConstructorKind::TrainingDummy
            | ProjectileConstructorKind::Arcing { .. } => false,
            ProjectileConstructorKind::Explosive { .. }
            | ProjectileConstructorKind::ExplosiveHazard { .. } => true,
        }
    }
}

/// Projectile motion: Returns the direction to aim for the projectile to reach
/// target position. Does not take any forces but gravity into account.
pub fn aim_projectile(speed: f32, pos: Vec3<f32>, tgt: Vec3<f32>, high_arc: bool) -> Option<Dir> {
    let mut to_tgt = tgt - pos;
    let dist_sqrd = to_tgt.xy().magnitude_squared();
    let u_sqrd = speed.powi(2);
    if high_arc {
        to_tgt.z = (u_sqrd
            + (u_sqrd.powi(2) - GRAVITY * (GRAVITY * dist_sqrd + 2.0 * to_tgt.z * u_sqrd))
                .sqrt()
                .max(0.0))
            / GRAVITY;
    } else {
        to_tgt.z = (u_sqrd
            - (u_sqrd.powi(2) - GRAVITY * (GRAVITY * dist_sqrd + 2.0 * to_tgt.z * u_sqrd))
                .sqrt()
                .max(0.0))
            / GRAVITY;
    }
    Dir::from_unnormalized(to_tgt)
}

#[derive(Clone, Debug, Default)]
pub struct ProjectileHitEntities {
    pub hit_entities: Vec<(Uid, Time)>,
}

impl Component for ProjectileHitEntities {
    type Storage = specs::DenseVecStorage<Self>;
}
