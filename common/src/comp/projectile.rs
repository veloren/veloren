use crate::{
    comp::buff::{BuffCategory, BuffData, BuffKind},
    effect::{BuffEffect, Effect as EffectB},
    sync::Uid,
    Damage, DamageSource, Explosion, GroupTarget, Knockback, RadiusEffect,
};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    Damage(Option<GroupTarget>, Damage),
    Knockback(Knockback),
    RewardEnergy(u32),
    Explode(Explosion),
    Vanish,
    Stick,
    Possess,
    Buff {
        buff: BuffEffect,
        chance: Option<f32>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
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
                let buff = BuffEffect {
                    kind: BuffKind::Bleeding,
                    data: BuffData {
                        strength: damage / 2.0,
                        duration: Some(Duration::from_secs(5)),
                    },
                    cat_ids: vec![BuffCategory::Physical],
                };
                Projectile {
                    hit_solid: vec![Effect::Stick],
                    hit_entity: vec![
                        Effect::Damage(Some(GroupTarget::OutOfGroup), Damage {
                            source: DamageSource::Projectile,
                            value: damage,
                        }),
                        Effect::Knockback(Knockback::Away(knockback)),
                        Effect::RewardEnergy(energy_regen),
                        Effect::Vanish,
                        Effect::Buff {
                            buff,
                            chance: Some(0.10),
                        },
                    ],
                    time_left: Duration::from_secs(15),
                    owner,
                    ignore_group: true,
                }
            },
            Fireball {
                damage,
                radius,
                energy_regen,
            } => Projectile {
                hit_solid: vec![
                    Effect::Explode(Explosion {
                        effects: vec![RadiusEffect::Entity(
                            Some(GroupTarget::OutOfGroup),
                            EffectB::Damage(Damage {
                                source: DamageSource::Explosion,
                                value: damage,
                            }),
                        )],
                        radius,
                        energy_regen,
                    }),
                    Effect::Vanish,
                ],
                hit_entity: vec![
                    Effect::Explode(Explosion {
                        effects: vec![RadiusEffect::Entity(
                            Some(GroupTarget::OutOfGroup),
                            EffectB::Damage(Damage {
                                source: DamageSource::Explosion,
                                value: damage,
                            }),
                        )],
                        radius,
                        energy_regen,
                    }),
                    Effect::Vanish,
                ],
                time_left: Duration::from_secs(20),
                owner,
                ignore_group: true,
            },
            Heal {
                heal,
                damage,
                radius,
            } => Projectile {
                hit_solid: vec![
                    Effect::Explode(Explosion {
                        effects: vec![
                            RadiusEffect::Entity(
                                Some(GroupTarget::OutOfGroup),
                                EffectB::Damage(Damage {
                                    source: DamageSource::Explosion,
                                    value: damage,
                                }),
                            ),
                            RadiusEffect::Entity(
                                Some(GroupTarget::InGroup),
                                EffectB::Damage(Damage {
                                    source: DamageSource::Healing,
                                    value: heal,
                                }),
                            ),
                        ],
                        radius,
                        energy_regen: 0,
                    }),
                    Effect::Vanish,
                ],
                hit_entity: vec![
                    Effect::Explode(Explosion {
                        effects: vec![
                            RadiusEffect::Entity(
                                Some(GroupTarget::OutOfGroup),
                                EffectB::Damage(Damage {
                                    source: DamageSource::Explosion,
                                    value: damage,
                                }),
                            ),
                            RadiusEffect::Entity(
                                Some(GroupTarget::InGroup),
                                EffectB::Damage(Damage {
                                    source: DamageSource::Healing,
                                    value: heal,
                                }),
                            ),
                        ],
                        radius,
                        energy_regen: 0,
                    }),
                    Effect::Vanish,
                ],
                time_left: Duration::from_secs(20),
                owner,
                ignore_group: true,
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
}
