use crate::{
    combat::DamageContributor, comp, consts::HP_PER_LEVEL, resources::Time, uid::Uid, DamageSource,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::{convert::TryFrom, ops::Mul};

/// Specifies what and how much changed current health
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HealthChange {
    /// The amount of the health change, negative is damage, positive is healing
    pub amount: f32,
    /// The individual or group who caused the health change (None if the
    /// damage wasn't caused by an entity)
    pub by: Option<DamageContributor>,
    /// The category of action that resulted in the health change
    pub cause: Option<DamageSource>,
    /// The time that the health change occurred at
    pub time: Time,
    /// A boolean that tells you if the change was a precsie hit
    pub precise: bool,
    /// A random ID, used to group up health changes from the same attack
    pub instance: u64,
}

impl HealthChange {
    pub fn damage_by(&self) -> Option<DamageContributor> {
        self.cause.is_some().then_some(self.by).flatten()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Health is represented by u32s within the module, but treated as a float by
/// the rest of the game.
// As a general rule, all input and output values to public functions should be
// floats rather than integers.
pub struct Health {
    // Current and base_max are scaled by 256 within this module compared to what is visible to
    // outside this module. The scaling is done to allow health to function as a fixed point while
    // still having the advantages of being an integer. The scaling of 256 was chosen so that max
    // health could be u16::MAX - 1, and then the scaled health could fit inside an f32 with no
    // precision loss
    /// Current health is how much health the entity currently has. Current
    /// health *must* be lower than or equal to maximum health.
    current: u32,
    /// Base max is the amount of health the entity has without considering
    /// temporary modifiers such as buffs
    base_max: u32,
    /// Maximum is the amount of health the entity has after temporary modifiers
    /// are considered
    maximum: u32,
    /// The last change to health
    pub last_change: HealthChange,
    pub is_dead: bool,

    /// Keeps track of damage per DamageContributor and the last time they
    /// caused damage, used for EXP sharing
    #[serde(skip)]
    damage_contributors: HashMap<DamageContributor, (u64, Time)>,
}

impl Health {
    /// Used when comparisons to health are needed outside this module.
    // This value is chosen as anything smaller than this is more precise than our
    // units of health.
    pub const HEALTH_EPSILON: f32 = 0.5 / Self::MAX_SCALED_HEALTH as f32;
    /// Maximum value allowed for health before scaling
    const MAX_HEALTH: u16 = u16::MAX - 1;
    /// The maximum value allowed for current and maximum health
    /// Maximum value is (u16:MAX - 1) * 256, which only requires 24 bits. This
    /// can fit into an f32 with no loss to precision
    // Cast to u32 done as u32::from cannot be called inside constant
    const MAX_SCALED_HEALTH: u32 = Self::MAX_HEALTH as u32 * Self::SCALING_FACTOR_INT;
    /// The amount health is scaled by within this module
    const SCALING_FACTOR_FLOAT: f32 = 256.;
    const SCALING_FACTOR_INT: u32 = Self::SCALING_FACTOR_FLOAT as u32;

    /// Returns the current value of health casted to a float
    pub fn current(&self) -> f32 { self.current as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the base maximum value of health casted to a float
    pub fn base_max(&self) -> f32 { self.base_max as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the maximum value of health casted to a float
    pub fn maximum(&self) -> f32 { self.maximum as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the fraction of health an entity has remaining
    pub fn fraction(&self) -> f32 { self.current() / self.maximum().max(1.0) }

    /// Calculates a new maximum value and returns it if the value differs from
    /// the current maximum.
    ///
    /// Note: The returned value uses an internal format so don't expect it to
    /// be useful for anything other than a parameter to
    /// [`Self::update_maximum`].
    pub fn needs_maximum_update(&self, modifiers: comp::stats::StatsModifier) -> Option<u32> {
        let maximum = modifiers
            .compute_maximum(self.base_max())
            .mul(Self::SCALING_FACTOR_FLOAT)
            // NaN does not need to be handled here as rust will automatically change to 0 when casting to u32
            .clamp(0.0, Self::MAX_SCALED_HEALTH as f32) as u32;

        (maximum != self.maximum).then_some(maximum)
    }

    /// Updates the maximum value for health.
    ///
    /// Note: The accepted `u32` value is in the internal format of this type.
    /// So attempting to pass values that weren't returned from
    /// [`Self::needs_maximum_update`] can produce strange or unexpected
    /// results.
    pub fn update_internal_integer_maximum(&mut self, maximum: u32) {
        self.maximum = maximum;
        // Clamp the current health to enforce the current <= maximum invariant.
        self.current = self.current.min(self.maximum);
    }

    pub fn new(body: comp::Body, level: u16) -> Self {
        let health = u32::from(
            body.base_health()
                .saturating_add(HP_PER_LEVEL.saturating_mul(level)),
        ) * Self::SCALING_FACTOR_INT;
        Health {
            current: health,
            base_max: health,
            maximum: health,
            last_change: HealthChange {
                amount: 0.0,
                by: None,
                cause: None,
                precise: false,
                time: Time(0.0),
                instance: rand::random(),
            },
            is_dead: false,
            damage_contributors: HashMap::new(),
        }
    }

    // TODO: Delete this once stat points will be a thing
    pub fn update_max_hp(&mut self, body: comp::Body, level: u16) {
        let old_max = self.base_max;
        self.base_max = u32::from(
            body.base_health()
                .saturating_add(HP_PER_LEVEL.saturating_mul(level)),
        ) * Self::SCALING_FACTOR_INT;
        self.current = (self.current + self.base_max - old_max).min(self.maximum);
    }

    /// Returns a boolean if the delta was not zero.
    pub fn change_by(&mut self, change: HealthChange) -> bool {
        let prev_health = i64::from(self.current);
        self.current = (((self.current() + change.amount).clamp(0.0, f32::from(Self::MAX_HEALTH))
            * Self::SCALING_FACTOR_FLOAT) as u32)
            .min(self.maximum);
        let delta = i64::from(self.current) - prev_health;

        self.last_change = change;

        // If damage is applied by an entity, update the damage contributors
        if delta < 0 {
            if let Some(attacker) = change.by {
                let entry = self
                    .damage_contributors
                    .entry(attacker)
                    .or_insert((0, change.time));
                entry.0 += u64::try_from(-delta).unwrap_or(0);
                entry.1 = change.time
            }

            // Prune any damage contributors who haven't contributed damage for over the
            // threshold - this enforces a maximum period that an entity will receive EXP
            // for a kill after they last damaged the killed entity.
            const DAMAGE_CONTRIB_PRUNE_SECS: f64 = 600.0;
            self.damage_contributors.retain(|_, (_, last_damage_time)| {
                (change.time.0 - last_damage_time.0) < DAMAGE_CONTRIB_PRUNE_SECS
            });
        }
        delta != 0
    }

    pub fn damage_contributions(&self) -> impl Iterator<Item = (&DamageContributor, &u64)> {
        self.damage_contributors
            .iter()
            .map(|(damage_contrib, (damage, _))| (damage_contrib, damage))
    }

    pub fn recent_damagers(&self) -> impl Iterator<Item = (Uid, Time)> + '_ {
        self.damage_contributors
            .iter()
            .map(|(contrib, (_, time))| (contrib.uid(), *time))
    }

    pub fn should_die(&self) -> bool { self.current == 0 }

    pub fn kill(&mut self) { self.current = 0; }

    pub fn revive(&mut self) {
        self.current = self.maximum;
        self.is_dead = false;
    }

    #[cfg(test)]
    pub fn empty() -> Self {
        Health {
            current: 0,
            base_max: 0,
            maximum: 0,
            last_change: HealthChange {
                amount: 0.0,
                by: None,
                cause: None,
                precise: false,
                time: Time(0.0),
                instance: rand::random(),
            },
            is_dead: false,
            damage_contributors: HashMap::new(),
        }
    }
}

impl Component for Health {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}

#[cfg(test)]
mod tests {
    use crate::{
        combat::DamageContributor,
        comp::{Health, HealthChange},
        resources::Time,
        uid::Uid,
    };

    #[test]
    fn test_change_by_negative_health_change_adds_to_damage_contributors() {
        let mut health = Health::empty();
        health.current = 100 * Health::SCALING_FACTOR_INT;
        health.maximum = health.current;

        let damage_contrib = DamageContributor::Solo(Uid(0));
        let health_change = HealthChange {
            amount: -5.0,
            time: Time(123.0),
            by: Some(damage_contrib),
            cause: None,
            precise: false,
            instance: rand::random(),
        };

        health.change_by(health_change);

        let (damage, time) = health.damage_contributors.get(&damage_contrib).unwrap();

        assert_eq!(
            health_change.amount.abs() as u64 * Health::SCALING_FACTOR_INT as u64,
            *damage
        );
        assert_eq!(health_change.time, *time);
    }

    #[test]
    fn test_change_by_positive_health_change_does_not_add_damage_contributor() {
        let mut health = Health::empty();
        health.maximum = 100 * Health::SCALING_FACTOR_INT;
        health.current = (health.maximum as f32 * 0.5) as u32;

        let damage_contrib = DamageContributor::Solo(Uid(0));
        let health_change = HealthChange {
            amount: 20.0,
            time: Time(123.0),
            by: Some(damage_contrib),
            cause: None,
            precise: false,
            instance: rand::random(),
        };

        health.change_by(health_change);

        assert!(health.damage_contributors.is_empty());
    }

    #[test]
    fn test_change_by_multiple_damage_from_same_damage_contributor() {
        let mut health = Health::empty();
        health.current = 100 * Health::SCALING_FACTOR_INT;
        health.maximum = health.current;

        let damage_contrib = DamageContributor::Solo(Uid(0));
        let health_change = HealthChange {
            amount: -5.0,
            time: Time(123.0),
            by: Some(damage_contrib),
            cause: None,
            precise: false,
            instance: rand::random(),
        };
        health.change_by(health_change);
        health.change_by(health_change);

        let (damage, _) = health.damage_contributors.get(&damage_contrib).unwrap();

        assert_eq!(
            (health_change.amount.abs() * 2.0) as u64 * Health::SCALING_FACTOR_INT as u64,
            *damage
        );
        assert_eq!(1, health.damage_contributors.len());
    }

    #[test]
    fn test_change_by_damage_contributor_pruning() {
        let mut health = Health::empty();
        health.current = 100 * Health::SCALING_FACTOR_INT;
        health.maximum = health.current;

        let damage_contrib1 = DamageContributor::Solo(Uid(0));
        let health_change = HealthChange {
            amount: -5.0,
            time: Time(10.0),
            by: Some(damage_contrib1),
            cause: None,
            precise: false,
            instance: rand::random(),
        };
        health.change_by(health_change);

        let damage_contrib2 = DamageContributor::Solo(Uid(1));
        let health_change = HealthChange {
            amount: -5.0,
            time: Time(100.0),
            by: Some(damage_contrib2),
            cause: None,
            precise: false,
            instance: rand::random(),
        };
        health.change_by(health_change);

        assert!(health.damage_contributors.contains_key(&damage_contrib1));
        assert!(health.damage_contributors.contains_key(&damage_contrib2));

        // Apply damage 610 seconds after the damage from damage_contrib1 - this should
        // result in the damage from damage_contrib1 being pruned.
        let health_change = HealthChange {
            amount: -5.0,
            time: Time(620.0),
            by: Some(damage_contrib2),
            cause: None,
            precise: false,
            instance: rand::random(),
        };
        health.change_by(health_change);

        assert!(!health.damage_contributors.contains_key(&damage_contrib1));
        assert!(health.damage_contributors.contains_key(&damage_contrib2));
    }
}
