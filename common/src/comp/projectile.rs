use crate::{
    combat::{
        Attack, AttackEffect, CombatBuff, CombatRequirement, Damage, DamageComponent, DamageSource,
        EffectComponent, GroupTarget, Knockback, KnockbackDir,
    },
    uid::Uid,
    Explosion, RadiusEffect,
};
use serde::{Deserialize, Serialize};
use specs::Component;
use specs_idvs::IdvStorage;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub enum Effect {
    Attack(Attack),
    //Knockback(Knockback),
    //RewardEnergy(u32),
    Explode(Explosion),
    Vanish,
    Stick,
    Possess,
    /* Buff {
     *     buff: BuffEffect,
     *     chance: Option<f32>,
     * }, */
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Projectile {
    // TODO: use SmallVec for these effects
    pub hit_solid: Vec<Effect>,
    pub hit_entity: Vec<Effect>,
    /// Time left until the projectile will despawn
    pub time_left: Duration,
    pub owner: Option<Uid>,
    /// Whether projectile collides with entities in the same group as its
    /// owner
    pub ignore_group: bool,
}

impl Component for Projectile {
    type Storage = IdvStorage<Self>;
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProjectileConstructor {
    Arrow {
        damage: f32,
        knockback: f32,
        energy_regen: u32,
    },
    Fireball {
        damage: f32,
        radius: f32,
        energy_regen: u32,
    },
    Firebolt {
        damage: f32,
        energy_regen: u32,
    },
    Heal {
        heal: f32,
        damage: f32,
        radius: f32,
    },
    Possess,
}

impl ProjectileConstructor {
    pub fn create_projectile(self, owner: Option<Uid>) -> Projectile {
        use ProjectileConstructor::*;
        match self {
            Arrow {
                damage,
                knockback,
                energy_regen,
            } => {
                let damage = Damage {
                    source: DamageSource::Projectile,
                    value: damage,
                };
                let knockback = AttackEffect::Knockback(Knockback {
                    strength: knockback,
                    direction: KnockbackDir::Away,
                });
                let energy = AttackEffect::EnergyReward(energy_regen);
                let energy = EffectComponent::new(None, energy)
                    .with_requirement(CombatRequirement::AnyDamage);
                let buff = AttackEffect::Buff(CombatBuff::default_physical());
                let damage = DamageComponent::new(damage, Some(GroupTarget::OutOfGroup))
                    .with_effect(knockback)
                    .with_effect(buff);
                let attack = Attack::default()
                    .with_damage(damage)
                    .with_crit(0.5, 1.2)
                    .with_effect(energy);

                Projectile {
                    hit_solid: vec![Effect::Stick],
                    hit_entity: vec![Effect::Attack(attack), Effect::Vanish],
                    time_left: Duration::from_secs(15),
                    owner,
                    ignore_group: true,
                }
            },
            Fireball {
                damage,
                radius,
                energy_regen,
            } => {
                let damage = Damage {
                    source: DamageSource::Explosion,
                    value: damage,
                };
                let energy = AttackEffect::EnergyReward(energy_regen);
                let energy = EffectComponent::new(None, energy)
                    .with_requirement(CombatRequirement::AnyDamage);
                let damage = DamageComponent::new(damage, Some(GroupTarget::OutOfGroup));
                let attack = Attack::default().with_damage(damage).with_effect(energy);
                let explosion = Explosion {
                    effects: vec![
                        RadiusEffect::Attack(attack),
                        RadiusEffect::TerrainDestruction(2.0),
                    ],
                    radius,
                };
                Projectile {
                    hit_solid: vec![Effect::Explode(explosion.clone()), Effect::Vanish],
                    hit_entity: vec![Effect::Explode(explosion), Effect::Vanish],
                    time_left: Duration::from_secs(10),
                    owner,
                    ignore_group: true,
                }
            },
            Firebolt {
                damage,
                energy_regen,
            } => {
                let damage = Damage {
                    source: DamageSource::Energy,
                    value: damage,
                };
                let energy = AttackEffect::EnergyReward(energy_regen);
                let energy = EffectComponent::new(None, energy)
                    .with_requirement(CombatRequirement::AnyDamage);
                let damage = DamageComponent::new(damage, Some(GroupTarget::OutOfGroup));
                let attack = Attack::default().with_damage(damage).with_effect(energy);

                Projectile {
                    hit_solid: vec![Effect::Vanish],
                    hit_entity: vec![Effect::Attack(attack), Effect::Vanish],
                    time_left: Duration::from_secs(10),
                    owner,
                    ignore_group: true,
                }
            },
            Heal {
                heal,
                damage,
                radius,
            } => {
                let damage = Damage {
                    source: DamageSource::Explosion,
                    value: damage,
                };
                let damage = DamageComponent::new(damage, Some(GroupTarget::OutOfGroup));
                let heal = AttackEffect::Heal(heal);
                let heal = EffectComponent::new(Some(GroupTarget::InGroup), heal);
                let attack = Attack::default().with_damage(damage).with_effect(heal);
                let explosion = Explosion {
                    effects: vec![RadiusEffect::Attack(attack)],
                    radius,
                };
                Projectile {
                    hit_solid: vec![Effect::Explode(explosion.clone()), Effect::Vanish],
                    hit_entity: vec![Effect::Explode(explosion), Effect::Vanish],
                    time_left: Duration::from_secs(10),
                    owner,
                    ignore_group: false,
                }
            },
            Possess => Projectile {
                hit_solid: vec![Effect::Stick],
                hit_entity: vec![Effect::Stick, Effect::Possess],
                time_left: Duration::from_secs(10),
                owner,
                ignore_group: false,
            },
        }
    }

    pub fn modified_projectile(
        mut self,
        power: f32,
        regen: f32,
        range: f32,
        heal_power: f32,
    ) -> Self {
        use ProjectileConstructor::*;
        match self {
            Arrow {
                ref mut damage,
                ref mut energy_regen,
                ..
            } => {
                *damage *= power;
                *energy_regen = (*energy_regen as f32 * regen) as u32;
            },
            Fireball {
                ref mut damage,
                ref mut energy_regen,
                ref mut radius,
                ..
            } => {
                *damage *= power;
                *energy_regen = (*energy_regen as f32 * regen) as u32;
                *radius *= range;
            },
            Firebolt {
                ref mut damage,
                ref mut energy_regen,
                ..
            } => {
                *damage *= power;
                *energy_regen = (*energy_regen as f32 * regen) as u32;
            },
            Heal {
                ref mut damage,
                ref mut heal,
                ref mut radius,
                ..
            } => {
                *damage *= power;
                *heal *= heal_power;
                *radius *= range;
            },
            Possess => {},
        }
        self
    }

    pub fn fireball_to_firebolt(self) -> Self {
        if let ProjectileConstructor::Fireball {
            damage,
            energy_regen,
            ..
        } = self
        {
            ProjectileConstructor::Firebolt {
                damage,
                energy_regen: energy_regen * 2,
            }
        } else {
            self
        }
    }
}
