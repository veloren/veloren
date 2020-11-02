use crate::{
    comp::{HealthChange, HealthSource, Loadout},
    sync::Uid,
    util::Dir,
};
use serde::{Deserialize, Serialize};
use vek::*;

pub const BLOCK_EFFICIENCY: f32 = 0.9;

/// Each section of this struct determines what damage is applied to a
/// particular target, using some identifier
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Damages {
    /// Targets enemies, and all other creatures not in your group
    pub enemy: Option<Damage>,
    /// Targets people in the same group as you, and any pets you have
    pub group: Option<Damage>,
}

impl Damages {
    pub fn new(enemy: Option<Damage>, group: Option<Damage>) -> Self { Damages { enemy, group } }

    pub fn get_damage(self, group_target: GroupTarget) -> Option<Damage> {
        match group_target {
            GroupTarget::InGroup => self.group,
            GroupTarget::OutOfGroup => self.enemy,
        }
    }

    pub fn contains_damage(self, source: DamageSource) -> bool {
        self.enemy.map_or(false, |e| e.source == source)
            || self.group.map_or(false, |g| g.source == source)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GroupTarget {
    InGroup,
    OutOfGroup,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DamageSource {
    Melee,
    Healing,
    Projectile,
    Explosion,
    Falling,
    Shockwave,
    Energy,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Damage {
    pub source: DamageSource,
    pub value: f32,
}

impl Damage {
    pub fn modify_damage(
        self,
        block: bool,
        loadout: Option<&Loadout>,
        uid: Option<Uid>,
    ) -> HealthChange {
        let mut damage = self.value;
        match self.source {
            DamageSource::Melee => {
                // Critical hit
                let mut critdamage = 0.0;
                if rand::random() {
                    critdamage = damage * 0.3;
                }
                // Block
                if block {
                    damage *= 1.0 - BLOCK_EFFICIENCY
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
                    cause: HealthSource::Attack { by: uid.unwrap() },
                }
            },
            DamageSource::Projectile => {
                // Critical hit
                if rand::random() {
                    damage *= 1.2;
                }
                // Block
                if block {
                    damage *= 1.0 - BLOCK_EFFICIENCY
                }
                // Armor
                let damage_reduction = loadout.map_or(0.0, |l| l.get_damage_reduction());
                damage *= 1.0 - damage_reduction;

                HealthChange {
                    amount: -damage as i32,
                    cause: HealthSource::Projectile { owner: uid },
                }
            },
            DamageSource::Explosion => {
                // Block
                if block {
                    damage *= 1.0 - BLOCK_EFFICIENCY
                }
                // Armor
                let damage_reduction = loadout.map_or(0.0, |l| l.get_damage_reduction());
                damage *= 1.0 - damage_reduction;

                HealthChange {
                    amount: -damage as i32,
                    cause: HealthSource::Explosion { owner: uid },
                }
            },
            DamageSource::Shockwave => {
                // Armor
                let damage_reduction = loadout.map_or(0.0, |l| l.get_damage_reduction());
                damage *= 1.0 - damage_reduction;

                HealthChange {
                    amount: -damage as i32,
                    cause: HealthSource::Attack { by: uid.unwrap() },
                }
            },
            DamageSource::Energy => {
                // Armor
                let damage_reduction = loadout.map_or(0.0, |l| l.get_damage_reduction());
                damage *= 1.0 - damage_reduction;

                HealthChange {
                    amount: -damage as i32,
                    cause: HealthSource::Energy { owner: uid },
                }
            },
            DamageSource::Healing => HealthChange {
                amount: damage as i32,
                cause: HealthSource::Healing { by: uid },
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
