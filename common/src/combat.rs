use crate::{
    comp::{HealthChange, HealthSource, Loadout},
    sync::Uid,
    util::Dir,
};
use serde::{Deserialize, Serialize};
use vek::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GroupTarget {
    InGroup,
    OutOfGroup,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DamageSource {
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
    pub fn modify_damage(self, loadout: Option<&Loadout>, uid: Option<Uid>) -> HealthChange {
        let mut damage = self.value;
        match self.source {
            DamageSource::Melee => {
                // Critical hit
                let mut critdamage = 0.0;
                if rand::random() {
                    critdamage = damage * 0.3;
                }
                // Armor
                let damage_reduction = loadout.map_or(0.0, |l| l.get_damage_reduction());
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
                let damage_reduction = loadout.map_or(0.0, |l| l.get_damage_reduction());
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
                let damage_reduction = loadout.map_or(0.0, |l| l.get_damage_reduction());
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
                let damage_reduction = loadout.map_or(0.0, |l| l.get_damage_reduction());
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
                let damage_reduction = loadout.map_or(0.0, |l| l.get_damage_reduction());
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
                let damage_reduction = loadout.map_or(0.0, |l| l.get_damage_reduction());
                if (damage_reduction - 1.0).abs() < f32::EPSILON {
                    damage = 0.0;
                }
                HealthChange {
                    amount: -damage as i32,
                    cause: HealthSource::World,
                }
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
pub enum Knockback {
    Away(f32),
    Towards(f32),
    Up(f32),
    TowardsUp(f32),
}

impl Knockback {
    pub fn calculate_impulse(self, dir: Dir) -> Vec3<f32> {
        match self {
            Knockback::Away(strength) => strength * *Dir::slerp(dir, Dir::new(Vec3::unit_z()), 0.5),
            Knockback::Towards(strength) => {
                strength * *Dir::slerp(-dir, Dir::new(Vec3::unit_z()), 0.5)
            },
            Knockback::Up(strength) => strength * Vec3::unit_z(),
            Knockback::TowardsUp(strength) => {
                strength * *Dir::slerp(-dir, Dir::new(Vec3::unit_z()), 0.85)
            },
        }
    }
}
