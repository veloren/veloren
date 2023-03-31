// We'd like to not have this file in `common`, but sadly there are
// things in `common` that require it (currently, `ServerEvent` and
// `Agent`). When possible, this should be moved to the `rtsim`
// module in `server`.

use rand::{seq::IteratorRandom, Rng};
use serde::{Deserialize, Serialize};
use specs::Component;
use strum::{EnumIter, IntoEnumIterator};
use vek::*;

use crate::comp::dialogue::MoodState;

slotmap::new_key_type! { pub struct NpcId; }

slotmap::new_key_type! { pub struct VehicleId; }

slotmap::new_key_type! { pub struct SiteId; }

slotmap::new_key_type! { pub struct FactionId; }

#[derive(Copy, Clone, Debug)]
pub struct RtSimEntity(pub NpcId);

impl Component for RtSimEntity {
    type Storage = specs::VecStorage<Self>;
}

#[derive(Copy, Clone, Debug)]
pub struct RtSimVehicle(pub VehicleId);

impl Component for RtSimVehicle {
    type Storage = specs::VecStorage<Self>;
}

#[derive(Clone, Debug)]
pub enum RtSimEvent {
    AddMemory(Memory),
    SetMood(Memory),
    ForgetEnemy(String),
    PrintMemories,
}

#[derive(Clone, Debug)]
pub struct Memory {
    pub item: MemoryItem,
    pub time_to_forget: f64,
}

#[derive(Clone, Debug)]
pub enum MemoryItem {
    // These are structs to allow more data beyond name to be stored
    // such as clothing worn, weapon used, etc.
    CharacterInteraction { name: String },
    CharacterFight { name: String },
    Mood { state: MoodState },
}

#[derive(EnumIter, Clone, Copy)]
pub enum PersonalityTrait {
    Open,
    Adventurous,
    Closed,
    Conscientious,
    Busybody,
    Unconscientious,
    Extroverted,
    Introverted,
    Agreeable,
    Sociable,
    Disagreeable,
    Neurotic,
    Seeker,
    Worried,
    SadLoner,
    Stable,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Personality {
    openness: u8,
    conscientiousness: u8,
    extraversion: u8,
    agreeableness: u8,
    neuroticism: u8,
}

fn distributed(min: u8, max: u8, rng: &mut impl Rng) -> u8 {
    let l = max - min;
    min + rng.gen_range(0..=l / 3)
        + rng.gen_range(0..=l / 3 + l % 3 % 2)
        + rng.gen_range(0..=l / 3 + l % 3 / 2)
}

impl Personality {
    pub const HIGH_THRESHOLD: u8 = Self::MAX - Self::LOW_THRESHOLD;
    pub const LITTLE_HIGH: u8 = Self::MID + (Self::MAX - Self::MIN) / 20;
    pub const LITTLE_LOW: u8 = Self::MID - (Self::MAX - Self::MIN) / 20;
    pub const LOW_THRESHOLD: u8 = (Self::MAX - Self::MIN) / 5 * 2 + Self::MIN;
    const MAX: u8 = 255;
    pub const MID: u8 = (Self::MAX - Self::MIN) / 2;
    const MIN: u8 = 0;

    fn distributed_value(rng: &mut impl Rng) -> u8 { distributed(Self::MIN, Self::MAX, rng) }

    pub fn random(rng: &mut impl Rng) -> Self {
        Self {
            openness: Self::distributed_value(rng),
            conscientiousness: Self::distributed_value(rng),
            extraversion: Self::distributed_value(rng),
            agreeableness: Self::distributed_value(rng),
            neuroticism: Self::distributed_value(rng),
        }
    }

    pub fn random_evil(rng: &mut impl Rng) -> Self {
        Self {
            openness: Self::distributed_value(rng),
            extraversion: Self::distributed_value(rng),
            neuroticism: Self::distributed_value(rng),
            agreeableness: distributed(0, Self::LOW_THRESHOLD - 1, rng),
            conscientiousness: distributed(0, Self::LOW_THRESHOLD - 1, rng),
        }
    }

    pub fn random_good(rng: &mut impl Rng) -> Self {
        Self {
            openness: Self::distributed_value(rng),
            extraversion: Self::distributed_value(rng),
            neuroticism: Self::distributed_value(rng),
            agreeableness: Self::distributed_value(rng),
            conscientiousness: distributed(Self::LOW_THRESHOLD, Self::MAX, rng),
        }
    }

    pub fn is(&self, trait_: PersonalityTrait) -> bool {
        match trait_ {
            PersonalityTrait::Open => self.openness > Personality::HIGH_THRESHOLD,
            PersonalityTrait::Adventurous => {
                self.openness > Personality::HIGH_THRESHOLD && self.neuroticism < Personality::MID
            },
            PersonalityTrait::Closed => self.openness < Personality::LOW_THRESHOLD,
            PersonalityTrait::Conscientious => self.conscientiousness > Personality::HIGH_THRESHOLD,
            PersonalityTrait::Busybody => self.agreeableness < Personality::LOW_THRESHOLD,
            PersonalityTrait::Unconscientious => {
                self.conscientiousness < Personality::LOW_THRESHOLD
            },
            PersonalityTrait::Extroverted => self.extraversion > Personality::HIGH_THRESHOLD,
            PersonalityTrait::Introverted => self.extraversion < Personality::LOW_THRESHOLD,
            PersonalityTrait::Agreeable => self.agreeableness > Personality::HIGH_THRESHOLD,
            PersonalityTrait::Sociable => {
                self.agreeableness > Personality::HIGH_THRESHOLD
                    && self.extraversion > Personality::MID
            },
            PersonalityTrait::Disagreeable => self.agreeableness < Personality::LOW_THRESHOLD,
            PersonalityTrait::Neurotic => self.neuroticism > Personality::HIGH_THRESHOLD,
            PersonalityTrait::Seeker => {
                self.neuroticism > Personality::HIGH_THRESHOLD
                    && self.openness > Personality::LITTLE_HIGH
            },
            PersonalityTrait::Worried => {
                self.neuroticism > Personality::HIGH_THRESHOLD
                    && self.agreeableness > Personality::LITTLE_HIGH
            },
            PersonalityTrait::SadLoner => {
                self.neuroticism > Personality::HIGH_THRESHOLD
                    && self.extraversion < Personality::LITTLE_LOW
            },
            PersonalityTrait::Stable => self.neuroticism < Personality::LOW_THRESHOLD,
        }
    }

    pub fn chat_trait(&self, rng: &mut impl Rng) -> Option<PersonalityTrait> {
        PersonalityTrait::iter().filter(|t| self.is(*t)).choose(rng)
    }

    pub fn will_ambush(&self) -> bool {
        self.agreeableness < Self::LOW_THRESHOLD && self.conscientiousness < Self::LOW_THRESHOLD
    }
}

impl Default for Personality {
    fn default() -> Self {
        Self {
            openness: Personality::MID,
            conscientiousness: Personality::MID,
            extraversion: Personality::MID,
            agreeableness: Personality::MID,
            neuroticism: Personality::MID,
        }
    }
}

/// This type is the map route through which the rtsim (real-time simulation)
/// aspect of the game communicates with the rest of the game. It is analagous
/// to `comp::Controller` in that it provides a consistent interface for
/// simulation NPCs to control their actions. Unlike `comp::Controller`, it is
/// very abstract and is intended for consumption by both the agent code and the
/// internal rtsim simulation code (depending on whether the entity is loaded
/// into the game as a physical entity or not). Agent code should attempt to act
/// upon its instructions where reasonable although deviations for various
/// reasons (obstacle avoidance, counter-attacking, etc.) are expected.
#[derive(Clone, Debug)]
pub struct RtSimController {
    /// When this field is `Some(..)`, the agent should attempt to make progress
    /// toward the given location, accounting for obstacles and other
    /// high-priority situations like being attacked.
    pub travel_to: Option<Vec3<f32>>,
    pub personality: Personality,
    pub heading_to: Option<String>,
    /// Proportion of full speed to move
    pub speed_factor: f32,
    /// Events
    pub events: Vec<RtSimEvent>,
}

impl Default for RtSimController {
    fn default() -> Self {
        Self {
            travel_to: None,
            personality: Personality::default(),
            heading_to: None,
            speed_factor: 1.0,
            events: Vec::new(),
        }
    }
}

impl RtSimController {
    pub fn with_destination(pos: Vec3<f32>) -> Self {
        Self {
            travel_to: Some(pos),
            personality: Personality::default(),
            heading_to: None,
            speed_factor: 0.5,
            events: Vec::new(),
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, enum_map::Enum)]
pub enum ChunkResource {
    #[serde(rename = "0")]
    Grass,
    #[serde(rename = "1")]
    Flower,
    #[serde(rename = "2")]
    Fruit,
    #[serde(rename = "3")]
    Vegetable,
    #[serde(rename = "4")]
    Mushroom,
    #[serde(rename = "5")]
    Loot, // Chests, boxes, potions, etc.
    #[serde(rename = "6")]
    Plant, // Flax, cotton, wheat, corn, etc.
    #[serde(rename = "7")]
    Stone,
    #[serde(rename = "8")]
    Wood, // Twigs, logs, bamboo, etc.
    #[serde(rename = "9")]
    Gem, // Amethyst, diamond, etc.
    #[serde(rename = "a")]
    Ore, // Iron, copper, etc.
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Profession {
    #[serde(rename = "0")]
    Farmer,
    #[serde(rename = "1")]
    Hunter,
    #[serde(rename = "2")]
    Merchant,
    #[serde(rename = "3")]
    Guard,
    #[serde(rename = "4")]
    Adventurer(u32),
    #[serde(rename = "5")]
    Blacksmith,
    #[serde(rename = "6")]
    Chef,
    #[serde(rename = "7")]
    Alchemist,
    #[serde(rename = "8")]
    Pirate,
    #[serde(rename = "9")]
    Cultist,
    #[serde(rename = "10")]
    Herbalist,
    #[serde(rename = "11")]
    Captain,
}

impl Profession {
    pub fn to_name(&self) -> String {
        match self {
            Self::Farmer => "Farmer".to_string(),
            Self::Hunter => "Hunter".to_string(),
            Self::Merchant => "Merchant".to_string(),
            Self::Guard => "Guard".to_string(),
            Self::Adventurer(_) => "Adventurer".to_string(),
            Self::Blacksmith => "Blacksmith".to_string(),
            Self::Chef => "Chef".to_string(),
            Self::Alchemist => "Alchemist".to_string(),
            Self::Pirate => "Pirate".to_string(),
            Self::Cultist => "Cultist".to_string(),
            Self::Herbalist => "Herbalist".to_string(),
            Self::Captain => "Captain".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldSettings {
    pub start_time: f64,
}

impl Default for WorldSettings {
    fn default() -> Self {
        Self {
            start_time: 9.0 * 3600.0, // 9am
        }
    }
}
