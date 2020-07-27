pub mod biped_large;
pub mod bird_medium;
pub mod bird_small;
pub mod critter;
pub mod dragon;
pub mod fish_medium;
pub mod fish_small;
pub mod golem;
pub mod humanoid;
pub mod object;
pub mod quadruped_low;
pub mod quadruped_medium;
pub mod quadruped_small;

use crate::{
    assets::{self, Asset},
    npc::NpcKind,
};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::{fs::File, io::BufReader};

/* pub trait PerBody {
    type Humanoid;
    type QuadrupedSmall;
    type QuadrupedMedium;
    type BirdMedium;
    type FishMedium;
    type Dragon;
    type BirdSmall;
    type FishSmall;
    type BipedLarge;
    type Object;
    type Golem;
    type Critter;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Body<Meta = BodyType, BodyMeta = ()>
    where Meta: PerBody,
{
    Humanoid(Meta::Humanoid) = 0,
    QuadrupedSmall(Meta::QuadrupedSmall) = 1,
    QuadrupedMedium(Meta::QuadrupedMedium) = 2,
    BirdMedium(Meta::BirdMedium) = 3,
    FishMedium(Meta::FishMedium) = 4,
    Dragon(Meta::Dragon) = 5,
    BirdSmall(Meta::BirdSmall) = 6,
    FishSmall(Meta::FishSmall) = 7,
    BipedLarge(Meta::BipedLarge) = 8,
    Object(Meta::Object) = 9,
    Golem(Meta::Golem) = 10,
    Critter(Meta::Critter) = 11,
}

/// Metadata intended to be stored per-body, together with data intended to be
/// stored for each species for each body.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct AllBodies<BodyMeta, PerBodyMeta: Meta> {
    pub humanoid: BodyData<BodyMeta, humanoid::AllSpecies<SpeciesMeta>>,
    pub quadruped_small: BodyData<BodyMeta, quadruped_small::AllSpecies<SpeciesMeta>>,
    pub quadruped_medium: BodyData<BodyMeta, quadruped_medium::AllSpecies<SpeciesMeta>>,
    pub bird_medium: BodyData<BodyMeta, bird_medium::AllSpecies<SpeciesMeta>>,
    pub fish_medium: BodyData<BodyMeta, ()>,
    pub dragon: BodyData<BodyMeta, dragon::AllSpecies<SpeciesMeta>>,
    pub bird_small: BodyData<BodyMeta, ()>,
    pub fish_small: BodyData<BodyMeta, ()>
    pub biped_large: BodyData<BodyMeta, biped_large::AllSpecies<SpeciesMeta>>,
    pub object: BodyData<BodyMeta, ()>,
    pub golem: BodyData<BodyMeta, golem::AllSpecies<SpeciesMeta>>,
    pub critter: BodyData<BodyMeta, critter::AllSpecies<SpeciesMeta>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BodyType;

impl PerBody for BodyType {
    type Humanoid = humanoid::Body;
    type QuadrupedSmall = quadruped_small::Body;
    type QuadrupedMedium = quadruped_medium::Body;
    type BirdMedium = bird_medium::Body;
    type FishMedium = fish_medium::Body;
    type Dragon = dragon::Body;
    type BirdSmall = bird_small::Body;
    type FishSmall = fish_small::Body;
    type BipedLarge = biped_large::Body;
    type Object = object::Body;
    type Golem = golem::Body;
    type Critter = critter::Body;
}
*/

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Body {
    Humanoid(humanoid::Body) = 0,
    QuadrupedSmall(quadruped_small::Body) = 1,
    QuadrupedMedium(quadruped_medium::Body) = 2,
    BirdMedium(bird_medium::Body) = 3,
    FishMedium(fish_medium::Body) = 4,
    Dragon(dragon::Body) = 5,
    BirdSmall(bird_small::Body) = 6,
    FishSmall(fish_small::Body) = 7,
    BipedLarge(biped_large::Body) = 8,
    Object(object::Body) = 9,
    Golem(golem::Body) = 10,
    Critter(critter::Body) = 11,
    QuadrupedLow(quadruped_low::Body) = 12,
}

/// Data representing data generic to the body together with per-species data.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct BodyData<BodyMeta, SpeciesData> {
    /// Shared metadata for this whole body type.
    pub body: BodyMeta,
    /// All the metadata for species with this body type.
    pub species: SpeciesData,
}

/// Metadata intended to be stored per-body, together with data intended to be
/// stored for each species for each body.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct AllBodies<BodyMeta, SpeciesMeta> {
    pub humanoid: BodyData<BodyMeta, humanoid::AllSpecies<SpeciesMeta>>,
    pub quadruped_small: BodyData<BodyMeta, quadruped_small::AllSpecies<SpeciesMeta>>,
    pub quadruped_medium: BodyData<BodyMeta, quadruped_medium::AllSpecies<SpeciesMeta>>,
    pub bird_medium: BodyData<BodyMeta, bird_medium::AllSpecies<SpeciesMeta>>,
    pub fish_medium: BodyData<BodyMeta, ()>,
    pub dragon: BodyData<BodyMeta, dragon::AllSpecies<SpeciesMeta>>,
    pub bird_small: BodyData<BodyMeta, ()>,
    pub fish_small: BodyData<BodyMeta, ()>,
    pub biped_large: BodyData<BodyMeta, biped_large::AllSpecies<SpeciesMeta>>,
    pub object: BodyData<BodyMeta, ()>,
    pub golem: BodyData<BodyMeta, golem::AllSpecies<SpeciesMeta>>,
    pub critter: BodyData<BodyMeta, critter::AllSpecies<SpeciesMeta>>,
    pub quadruped_low: BodyData<BodyMeta, quadruped_low::AllSpecies<SpeciesMeta>>,
}

/// Can only retrieve body metadata by direct index.
impl<BodyMeta, SpeciesMeta> core::ops::Index<NpcKind> for AllBodies<BodyMeta, SpeciesMeta> {
    type Output = BodyMeta;

    #[inline]
    fn index(&self, index: NpcKind) -> &Self::Output {
        match index {
            NpcKind::Humanoid => &self.humanoid.body,
            NpcKind::Pig => &self.quadruped_small.body,
            NpcKind::Wolf => &self.quadruped_medium.body,
            NpcKind::Duck => &self.bird_medium.body,
            NpcKind::Ogre => &self.biped_large.body,
            NpcKind::StoneGolem => &self.golem.body,
            NpcKind::Rat => &self.critter.body,
            NpcKind::Reddragon => &self.dragon.body,
            NpcKind::Crocodile => &self.quadruped_low.body,
        }
    }
}

/// Can only retrieve body metadata by direct index.
impl<'a, BodyMeta, SpeciesMeta> core::ops::Index<&'a Body> for AllBodies<BodyMeta, SpeciesMeta> {
    type Output = BodyMeta;

    #[inline]
    fn index(&self, index: &Body) -> &Self::Output {
        match index {
            Body::Humanoid(_) => &self.humanoid.body,
            Body::QuadrupedSmall(_) => &self.quadruped_small.body,
            Body::QuadrupedMedium(_) => &self.quadruped_medium.body,
            Body::BirdMedium(_) => &self.bird_medium.body,
            Body::FishMedium(_) => &self.fish_medium.body,
            Body::Dragon(_) => &self.dragon.body,
            Body::BirdSmall(_) => &self.bird_small.body,
            Body::FishSmall(_) => &self.fish_small.body,
            Body::BipedLarge(_) => &self.biped_large.body,
            Body::Object(_) => &self.object.body,
            Body::Golem(_) => &self.golem.body,
            Body::Critter(_) => &self.critter.body,
            Body::QuadrupedLow(_) => &self.quadruped_low.body,
        }
    }
}

impl<
    BodyMeta: Send + Sync + for<'de> serde::Deserialize<'de>,
    SpeciesMeta: Send + Sync + for<'de> serde::Deserialize<'de>,
> Asset for AllBodies<BodyMeta, SpeciesMeta>
{
    const ENDINGS: &'static [&'static str] = &["json"];

    fn parse(buf_reader: BufReader<File>) -> Result<Self, assets::Error> {
        serde_json::de::from_reader(buf_reader).map_err(assets::Error::parse_error)
    }
}

impl Body {
    pub fn is_humanoid(&self) -> bool {
        match self {
            Body::Humanoid(_) => true,
            _ => false,
        }
    }

    // Note: this might need to be refined to something more complex for realistic
    // behavior with less cylindrical bodies (e.g. wolfs)
    pub fn radius(&self) -> f32 {
        // TODO: Improve these values (some might be reliant on more info in inner type)
        match self {
            Body::Humanoid(_) => 0.5,
            Body::QuadrupedSmall(_) => 0.3,
            Body::QuadrupedMedium(_) => 0.9,
            Body::Critter(_) => 0.2,
            Body::BirdMedium(_) => 0.5,
            Body::FishMedium(_) => 0.5,
            Body::Dragon(_) => 2.5,
            Body::BirdSmall(_) => 0.2,
            Body::FishSmall(_) => 0.2,
            Body::BipedLarge(_) => 2.0,
            Body::Golem(_) => 2.5,
            Body::QuadrupedLow(_) => 1.0,
            Body::Object(_) => 0.3,
        }
    }

    // Note: currently assumes sphericality
    pub fn height(&self) -> f32 { self.radius() * 2.0 }

    pub fn base_health(&self) -> u32 {
        match self {
            Body::Humanoid(_) => 52,
            Body::QuadrupedSmall(_) => 44,
            Body::QuadrupedMedium(_) => 72,
            Body::BirdMedium(_) => 36,
            Body::FishMedium(_) => 32,
            Body::Dragon(_) => 256,
            Body::BirdSmall(_) => 24,
            Body::FishSmall(_) => 20,
            Body::BipedLarge(_) => 144,
            Body::Object(_) => 100,
            Body::Golem(_) => 168,
            Body::Critter(_) => 32,
            Body::QuadrupedLow(_) => 64,
        }
    }

    pub fn base_health_increase(&self) -> u32 {
        match self {
            Body::Humanoid(_) => 5,
            Body::QuadrupedSmall(_) => 4,
            Body::QuadrupedMedium(_) => 7,
            Body::BirdMedium(_) => 4,
            Body::FishMedium(_) => 3,
            Body::Dragon(_) => 26,
            Body::BirdSmall(_) => 2,
            Body::FishSmall(_) => 2,
            Body::BipedLarge(_) => 14,
            Body::Object(_) => 0,
            Body::Golem(_) => 17,
            Body::Critter(_) => 3,
            Body::QuadrupedLow(_) => 6,
        }
    }

    pub fn base_exp(&self) -> u32 {
        match self {
            Body::Humanoid(_) => 15,
            Body::QuadrupedSmall(_) => 12,
            Body::QuadrupedMedium(_) => 28,
            Body::BirdMedium(_) => 10,
            Body::FishMedium(_) => 8,
            Body::Dragon(_) => 160,
            Body::BirdSmall(_) => 5,
            Body::FishSmall(_) => 4,
            Body::BipedLarge(_) => 75,
            Body::Object(_) => 0,
            Body::Golem(_) => 75,
            Body::Critter(_) => 8,
            Body::QuadrupedLow(_) => 24,
        }
    }

    pub fn base_exp_increase(&self) -> u32 {
        match self {
            Body::Humanoid(_) => 3,
            Body::QuadrupedSmall(_) => 2,
            Body::QuadrupedMedium(_) => 6,
            Body::BirdMedium(_) => 2,
            Body::FishMedium(_) => 2,
            Body::Dragon(_) => 32,
            Body::BirdSmall(_) => 1,
            Body::FishSmall(_) => 1,
            Body::BipedLarge(_) => 15,
            Body::Object(_) => 0,
            Body::Golem(_) => 15,
            Body::Critter(_) => 2,
            Body::QuadrupedLow(_) => 5,
        }
    }

    pub fn base_dmg(&self) -> u32 {
        match self {
            Body::Humanoid(_) => 6,
            Body::QuadrupedSmall(_) => 8,
            Body::QuadrupedMedium(_) => 12,
            Body::BirdMedium(_) => 7,
            Body::FishMedium(_) => 6,
            Body::Dragon(_) => 90,
            Body::BirdSmall(_) => 5,
            Body::FishSmall(_) => 3,
            Body::BipedLarge(_) => 36,
            Body::Object(_) => 0,
            Body::Golem(_) => 36,
            Body::Critter(_) => 7,
            Body::QuadrupedLow(_) => 11,
        }
    }

    pub fn base_range(&self) -> f32 {
        match self {
            Body::Humanoid(_) => 5.0,
            Body::QuadrupedSmall(_) => 4.5,
            Body::QuadrupedMedium(_) => 5.5,
            Body::BirdMedium(_) => 3.5,
            Body::FishMedium(_) => 3.5,
            Body::Dragon(_) => 12.5,
            Body::BirdSmall(_) => 3.0,
            Body::FishSmall(_) => 3.0,
            Body::BipedLarge(_) => 10.0,
            Body::Object(_) => 3.0,
            Body::Golem(_) => 7.5,
            Body::Critter(_) => 3.0,
            Body::QuadrupedLow(_) => 4.5,
        }
    }
}

impl Component for Body {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
