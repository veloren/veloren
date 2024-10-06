use crate::{
    combat::{
        Attack, AttackDamage, AttackEffect, CombatBuff, CombatEffect, CombatRequirement, Damage,
        DamageKind, DamageSource, GroupTarget, Knockback, KnockbackDir,
    },
    comp::item::{Reagent, tool},
    explosion::{ColorPreset, Explosion, RadiusEffect},
    resources::Secs,
    uid::Uid,
};
use common_base::dev_panic;
use serde::{Deserialize, Serialize};
use specs::Component;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Effect {
    Attack(Attack),
    Explode(Explosion),
    Vanish,
    Stick,
    Possess,
    Bonk, // Knock/dislodge/change objects on hit
    DropItem,
    Firework(Reagent),
    SurpriseEgg,
    TrainingDummy,
}

#[derive(Clone, Debug)]
pub struct Projectile {
    // TODO: use SmallVec for these effects
    pub hit_solid: Vec<Effect>,
    pub hit_entity: Vec<Effect>,
    pub timeout: Vec<Effect>,
    /// Time left until the projectile will despawn
    pub time_left: Duration,
    pub owner: Option<Uid>,
    /// Whether projectile collides with entities in the same group as its
    /// owner
    pub ignore_group: bool,
    /// Whether the projectile is sticky
    pub is_sticky: bool,
    /// Whether the projectile should use a point collider
    pub is_point: bool,
}

impl Component for Projectile {
    type Storage = specs::DenseVecStorage<Self>;
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectileConstructor {
    pub kind: ProjectileConstructorKind,
    pub attack: Option<ProjectileAttack>,
    pub scaled: Option<Scaled>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scaled {
    damage: f32,
    poise: Option<f32>,
    knockback: Option<f32>,
    energy: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectileAttack {
    pub damage: f32,
    pub poise: Option<f32>,
    pub knockback: Option<f32>,
    pub energy: Option<f32>,
    pub buff: Option<CombatBuff>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProjectileConstructorKind {
    // I want a better name for 'Pointed' and 'Blunt'
    Pointed,
    Blunt,
    Explosive {
        radius: f32,
        min_falloff: f32,
        reagent: Option<Reagent>,
        terrain: Option<(f32, ColorPreset)>,
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
    ThrownWeapon,
    Firework(Reagent),
    SurpriseEgg,
    TrainingDummy,
}

impl ProjectileConstructor {
    pub fn create_projectile(
        self,
        owner: Option<Uid>,
        precision_mult: f32,
        damage_effect: Option<CombatEffect>,
    ) -> Projectile {
        if self.scaled.is_some() {
            dev_panic!(
                "Attempted to create a projectile that had a provided scaled value without \
                 scaling the projectile."
            )
        }

        let instance = rand::random();
        let attack = self.attack.map(|a| {
            let poise = a.poise.map(|poise| {
                AttackEffect::new(Some(GroupTarget::OutOfGroup), CombatEffect::Poise(poise))
                    .with_requirement(CombatRequirement::AnyDamage)
            });

            let knockback = a.knockback.map(|kb| {
                AttackEffect::new(
                    Some(GroupTarget::OutOfGroup),
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

            let (damage_source, damage_kind) = match self.kind {
                ProjectileConstructorKind::Pointed
                | ProjectileConstructorKind::Hazard { .. }
                | ProjectileConstructorKind::ThrownWeapon => {
                    (DamageSource::Projectile, DamageKind::Piercing)
                },
                ProjectileConstructorKind::Blunt => {
                    (DamageSource::Projectile, DamageKind::Crushing)
                },
                ProjectileConstructorKind::Explosive { .. }
                | ProjectileConstructorKind::ExplosiveHazard { .. }
                | ProjectileConstructorKind::Firework(_) => {
                    (DamageSource::Explosion, DamageKind::Energy)
                },
                ProjectileConstructorKind::Possess
                | ProjectileConstructorKind::SurpriseEgg
                | ProjectileConstructorKind::TrainingDummy => {
                    dev_panic!("This should be unreachable");
                    (DamageSource::Projectile, DamageKind::Piercing)
                },
            };

            let mut damage = AttackDamage::new(
                Damage {
                    source: damage_source,
                    kind: damage_kind,
                    value: a.damage,
                },
                Some(GroupTarget::OutOfGroup),
                instance,
            );

            if let Some(buff) = buff {
                damage = damage.with_effect(buff);
            }

            if let Some(damage_effect) = damage_effect {
                damage = damage.with_effect(damage_effect);
            }

            let mut attack = Attack::default()
                .with_damage(damage)
                .with_precision(precision_mult)
                .with_combo_increment();

            if let Some(poise) = poise {
                attack = attack.with_effect(poise);
            }

            if let Some(knockback) = knockback {
                attack = attack.with_effect(knockback);
            }

            if let Some(energy) = energy {
                attack = attack.with_effect(energy);
            }

            attack
        });

        match self.kind {
            ProjectileConstructorKind::Pointed | ProjectileConstructorKind::Blunt => {
                let mut hit_entity = vec![Effect::Vanish];

                if let Some(attack) = attack {
                    hit_entity.push(Effect::Attack(attack));
                }

                Projectile {
                    hit_solid: vec![Effect::Stick, Effect::Bonk],
                    hit_entity,
                    timeout: Vec::new(),
                    time_left: Duration::from_secs(15),
                    owner,
                    ignore_group: true,
                    is_sticky: true,
                    is_point: true,
                }
            },
            ProjectileConstructorKind::Hazard {
                is_sticky,
                duration,
            } => {
                let mut hit_entity = vec![Effect::Vanish];

                if let Some(attack) = attack {
                    hit_entity.push(Effect::Attack(attack));
                }

                Projectile {
                    hit_solid: vec![Effect::Stick, Effect::Bonk],
                    hit_entity,
                    timeout: Vec::new(),
                    time_left: Duration::from_secs_f64(duration.0),
                    owner,
                    ignore_group: true,
                    is_sticky,
                    is_point: false,
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
                    effects.push(RadiusEffect::Attack(attack));
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

                Projectile {
                    hit_solid: vec![Effect::Explode(explosion.clone()), Effect::Vanish],
                    hit_entity: vec![Effect::Explode(explosion), Effect::Vanish],
                    timeout: Vec::new(),
                    time_left: Duration::from_secs(10),
                    owner,
                    ignore_group: true,
                    is_sticky: true,
                    is_point: true,
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
                    effects.push(RadiusEffect::Attack(attack));
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

                Projectile {
                    hit_solid: Vec::new(),
                    hit_entity: vec![Effect::Explode(explosion), Effect::Vanish],
                    timeout: Vec::new(),
                    time_left: Duration::from_secs_f64(duration.0),
                    owner,
                    ignore_group: true,
                    is_sticky,
                    is_point: false,
                }
            },
            ProjectileConstructorKind::Possess => Projectile {
                hit_solid: vec![Effect::Stick],
                hit_entity: vec![Effect::Stick, Effect::Possess],
                timeout: Vec::new(),
                time_left: Duration::from_secs(10),
                owner,
                ignore_group: false,
                is_sticky: true,
                is_point: true,
            },
            ProjectileConstructorKind::ThrownWeapon => {
                let effects = vec![Effect::DropItem, Effect::Vanish];

                let mut hit_entity = effects.clone();
                if let Some(attack) = attack {
                    hit_entity.push(Effect::Attack(attack));
                }

                Projectile {
                    hit_solid: effects.clone(),
                    hit_entity,
                    timeout: effects,
                    time_left: Duration::from_secs(10),
                    owner,
                    ignore_group: true,
                    is_sticky: true,
                    is_point: true,
                }
            },
            ProjectileConstructorKind::Firework(reagent) => Projectile {
                hit_solid: Vec::new(),
                hit_entity: Vec::new(),
                timeout: vec![Effect::Firework(reagent)],
                time_left: Duration::from_secs(3),
                owner,
                ignore_group: true,
                is_sticky: true,
                is_point: true,
            },
            ProjectileConstructorKind::SurpriseEgg => Projectile {
                hit_solid: vec![Effect::SurpriseEgg, Effect::Vanish],
                hit_entity: vec![Effect::SurpriseEgg, Effect::Vanish],
                timeout: Vec::new(),
                time_left: Duration::from_secs(15),
                owner,
                ignore_group: true,
                is_sticky: true,
                is_point: true,
            },
            ProjectileConstructorKind::TrainingDummy => Projectile {
                hit_solid: vec![Effect::TrainingDummy, Effect::Vanish],
                hit_entity: vec![Effect::TrainingDummy, Effect::Vanish],
                timeout: vec![Effect::TrainingDummy],
                time_left: Duration::from_secs(15),
                owner,
                ignore_group: true,
                is_sticky: true,
                is_point: false,
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
            | ProjectileConstructorKind::Possess
            | ProjectileConstructorKind::Hazard { .. }
            | ProjectileConstructorKind::ThrownWeapon
            | ProjectileConstructorKind::Firework(_)
            | ProjectileConstructorKind::SurpriseEgg
            | ProjectileConstructorKind::TrainingDummy => {},
            ProjectileConstructorKind::Explosive { ref mut radius, .. }
            | ProjectileConstructorKind::ExplosiveHazard { ref mut radius, .. } => {
                *radius *= stats.range;
            },
        }

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
            | ProjectileConstructorKind::Possess
            | ProjectileConstructorKind::Hazard { .. }
            | ProjectileConstructorKind::ThrownWeapon
            | ProjectileConstructorKind::Firework(_)
            | ProjectileConstructorKind::SurpriseEgg
            | ProjectileConstructorKind::TrainingDummy => false,
            ProjectileConstructorKind::Explosive { .. }
            | ProjectileConstructorKind::ExplosiveHazard { .. } => true,
        }
    }
}
