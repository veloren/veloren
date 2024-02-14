use crate::{
    combat::{DamageContributor, DamageSource},
    comp::{
        self,
        ability::Capability,
        inventory::item::{armor::Protection, ItemKind, MaterialStatManifest},
        CharacterState, Inventory, Stats,
    },
    resources::Time,
    states,
    util::Dir,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, VecStorage};
use std::{ops::Mul, time::Duration};
use vek::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PoiseChange {
    /// The amount of the poise change
    pub amount: f32,
    /// The direction that the poise change came from, used for when the target
    /// is knocked down
    pub impulse: Vec3<f32>,
    /// The individual or group who caused the poise change (None if the
    /// damage wasn't caused by an entity)
    pub by: Option<DamageContributor>,
    /// The category of action that resulted in the poise change
    pub cause: Option<DamageSource>,
    /// The time that the poise change occurred at
    pub time: Time,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
/// Poise is represented by u32s within the module, but treated as a float by
/// the rest of the game.
// As a general rule, all input and output values to public functions should be
// floats rather than integers.
pub struct Poise {
    // Current and base_max are scaled by 256 within this module compared to what is visible to
    // outside this module. The scaling is done to allow poise to function as a fixed point while
    // still having the advantages of being an integer. The scaling of 256 was chosen so that max
    // poise could be u16::MAX - 1, and then the scaled poise could fit inside an f32 with no
    // precision loss
    /// Current poise is how much poise the entity currently has
    current: u32,
    /// Base max is the amount of poise the entity has without considering
    /// temporary modifiers such as buffs
    base_max: u32,
    /// Maximum is the amount of poise the entity has after temporary modifiers
    /// are considered
    maximum: u32,
    /// Direction that the last poise change came from
    pub last_change: Dir,
    /// Rate of regeneration per tick. Starts at zero and accelerates.
    regen_rate: f32,
    /// Time that entity was last in a poise state
    last_stun_time: Option<Time>,
    /// The previous poise state
    pub previous_state: PoiseState,
}

/// States to define effects of a poise change
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Eq, Hash)]
pub enum PoiseState {
    /// No effect applied
    Normal,
    /// Poise reset, and target briefly stunned
    Interrupted,
    /// Poise reset, target stunned and knocked back horizontally
    Stunned,
    /// Poise reset, target staggered
    Dazed,
    /// Poise reset, target staggered and knocked back further
    KnockedDown,
}

impl PoiseState {
    /// Returns the optional stunned character state and duration of stun, and
    /// optional impulse strength corresponding to a particular poise state
    pub fn poise_effect(&self, was_wielded: bool) -> (Option<(CharacterState, f64)>, Option<f32>) {
        use states::{
            stunned::{Data, StaticData},
            utils::StageSection,
        };
        // charstate_parameters is Option<(buildup_duration, recover_duration,
        // movement_speed)>
        let (charstate_parameters, impulse) = match self {
            PoiseState::Normal => (None, None),
            PoiseState::Interrupted => (
                Some((Duration::from_millis(200), Duration::from_millis(200), 0.8)),
                None,
            ),
            PoiseState::Stunned => (
                Some((Duration::from_millis(350), Duration::from_millis(350), 0.5)),
                None,
            ),
            PoiseState::Dazed => (
                Some((Duration::from_millis(750), Duration::from_millis(750), 0.2)),
                None,
            ),
            PoiseState::KnockedDown => (
                Some((
                    Duration::from_millis(1500),
                    Duration::from_millis(1500),
                    0.0,
                )),
                Some(10.0),
            ),
        };
        (
            charstate_parameters.map(|(buildup_duration, recover_duration, movement_speed)| {
                (
                    CharacterState::Stunned(Data {
                        static_data: StaticData {
                            buildup_duration,
                            recover_duration,
                            movement_speed,
                            poise_state: *self,
                        },
                        timer: Duration::default(),
                        stage_section: StageSection::Buildup,
                        was_wielded,
                    }),
                    buildup_duration.as_secs_f64() + recover_duration.as_secs_f64(),
                )
            }),
            impulse,
        )
    }

    /// Returns the multiplier on poise damage to health damage for when the
    /// target is in a poise state, also is used for precision
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            Self::Interrupted => 0.1,
            Self::Stunned => 0.25,
            Self::Dazed => 0.5,
            Self::KnockedDown => 1.0,
            // Should never be reached
            Self::Normal => 0.0,
        }
    }
}

impl Poise {
    /// Maximum value allowed for poise before scaling
    const MAX_POISE: u16 = u16::MAX - 1;
    /// The maximum value allowed for current and maximum poise
    /// Maximum value is (u16:MAX - 1) * 256, which only requires 24 bits. This
    /// can fit into an f32 with no loss to precision
    // Cast to u32 done as u32::from cannot be called inside constant
    const MAX_SCALED_POISE: u32 = Self::MAX_POISE as u32 * Self::SCALING_FACTOR_INT;
    /// The amount of time after being in a poise state before you can take
    /// poise damage again
    pub const POISE_BUFFER_TIME: f64 = 1.0;
    /// Used when comparisons to poise are needed outside this module.
    // This value is chosen as anything smaller than this is more precise than our
    // units of poise.
    pub const POISE_EPSILON: f32 = 0.5 / Self::MAX_SCALED_POISE as f32;
    /// The thresholds where poise changes to a different state
    pub const POISE_THRESHOLDS: [f32; 4] = [50.0, 30.0, 15.0, 5.0];
    /// The amount poise is scaled by within this module
    const SCALING_FACTOR_FLOAT: f32 = 256.;
    const SCALING_FACTOR_INT: u32 = Self::SCALING_FACTOR_FLOAT as u32;

    /// Returns the current value of poise casted to a float
    pub fn current(&self) -> f32 { self.current as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the base maximum value of poise casted to a float
    pub fn base_max(&self) -> f32 { self.base_max as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the maximum value of poise casted to a float
    pub fn maximum(&self) -> f32 { self.maximum as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the fraction of poise an entity has remaining
    pub fn fraction(&self) -> f32 { self.current() / self.maximum().max(1.0) }

    /// Updates the maximum value for poise
    pub fn update_maximum(&mut self, modifiers: comp::stats::StatsModifier) {
        let maximum = modifiers
            .compute_maximum(self.base_max())
            .mul(Self::SCALING_FACTOR_FLOAT)
            // NaN does not need to be handled here as rust will automatically change to 0 when casting to u32
            .clamp(0.0, Self::MAX_SCALED_POISE as f32) as u32;
        self.maximum = maximum;
        self.current = self.current.min(self.maximum);
    }

    pub fn new(body: comp::Body) -> Self {
        let poise = u32::from(body.base_poise()) * Self::SCALING_FACTOR_INT;
        Poise {
            current: poise,
            base_max: poise,
            maximum: poise,
            last_change: Dir::default(),
            regen_rate: 0.0,
            last_stun_time: None,
            previous_state: PoiseState::Normal,
        }
    }

    pub fn change(&mut self, change: PoiseChange) {
        match self.last_stun_time {
            Some(last_time) if last_time.0 + Poise::POISE_BUFFER_TIME > change.time.0 => {},
            _ => {
                // if self.previous_state != self.poise_state() {
                self.previous_state = self.poise_state();
                // };
                self.current = (((self.current() + change.amount)
                    .clamp(0.0, f32::from(Self::MAX_POISE))
                    * Self::SCALING_FACTOR_FLOAT) as u32)
                    .min(self.maximum);
                self.last_change = Dir::from_unnormalized(change.impulse).unwrap_or_default();
            },
        }
    }

    /// Returns `true` if the current value is less than the maximum
    pub fn needs_regen(&self) -> bool { self.current < self.maximum }

    /// Regenerates poise based on a provided acceleration
    pub fn regen(&mut self, accel: f32, dt: f32, now: Time) {
        if self.current < self.maximum {
            let poise_change = PoiseChange {
                amount: self.regen_rate * dt,
                impulse: Vec3::zero(),
                by: None,
                cause: None,
                time: now,
            };
            self.change(poise_change);
            self.regen_rate = (self.regen_rate + accel * dt).min(10.0);
        }
    }

    pub fn reset(&mut self, time: Time, poise_state_time: f64) {
        self.current = self.maximum;
        self.last_stun_time = Some(Time(time.0 + poise_state_time));
    }

    /// Returns knockback as a Dir
    /// Kept as helper function should additional fields ever be added to last
    /// change
    pub fn knockback(&self) -> Dir { self.last_change }

    /// Defines the poise states based on current poise value
    pub fn poise_state(&self) -> PoiseState {
        match self.current() {
            x if x > Self::POISE_THRESHOLDS[0] => PoiseState::Normal,
            x if x > Self::POISE_THRESHOLDS[1] => PoiseState::Interrupted,
            x if x > Self::POISE_THRESHOLDS[2] => PoiseState::Stunned,
            x if x > Self::POISE_THRESHOLDS[3] => PoiseState::Dazed,
            _ => PoiseState::KnockedDown,
        }
    }

    /// Returns the total poise damage reduction provided by all equipped items
    pub fn compute_poise_damage_reduction(
        inventory: Option<&Inventory>,
        msm: &MaterialStatManifest,
        char_state: Option<&CharacterState>,
        stats: Option<&Stats>,
    ) -> f32 {
        let protection = inventory.map_or(Some(0.0), |inv| {
            inv.equipped_items()
                .filter_map(|item| {
                    if let ItemKind::Armor(armor) = &*item.kind() {
                        armor
                            .stats(msm, item.stats_durability_multiplier())
                            .poise_resilience
                    } else {
                        None
                    }
                })
                .map(|protection| match protection {
                    Protection::Normal(protection) => Some(protection),
                    Protection::Invincible => None,
                })
                .sum::<Option<f32>>()
        });
        let from_inventory = match protection {
            Some(dr) => dr / (60.0 + dr.abs()),
            None => 1.0,
        };
        let from_char = {
            let resistant = char_state
                .and_then(|cs| cs.ability_info())
                .map(|a| a.ability_meta)
                .map_or(false, |a| {
                    a.capabilities.contains(Capability::POISE_RESISTANT)
                });
            if resistant { 0.5 } else { 0.0 }
        };
        let from_stats = if let Some(stats) = stats {
            stats.poise_reduction
        } else {
            0.0
        };
        1.0 - (1.0 - from_inventory) * (1.0 - from_char) * (1.0 - from_stats)
    }

    /// Modifies a poise change when optionally given an inventory and character
    /// state to aid in calculation of poise damage reduction
    pub fn apply_poise_reduction(
        value: f32,
        inventory: Option<&Inventory>,
        msm: &MaterialStatManifest,
        char_state: Option<&CharacterState>,
        stats: Option<&Stats>,
    ) -> f32 {
        value * (1.0 - Poise::compute_poise_damage_reduction(inventory, msm, char_state, stats))
    }
}

impl Component for Poise {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}
