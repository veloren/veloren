pub mod biped_large;
pub mod bird_medium;
pub mod bird_small;
pub mod dragon;
pub mod fish_medium;
pub mod fish_small;
pub mod golem;
pub mod humanoid;
pub mod object;
pub mod quadruped_low;
pub mod quadruped_medium;
pub mod quadruped_small;
pub mod theropod;

use crate::{
    assets::{self, Asset},
    make_case_elim,
    npc::NpcKind,
};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use vek::*;

make_case_elim!(
    body,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Body {
        Humanoid(body: humanoid::Body) = 0,
        QuadrupedSmall(body: quadruped_small::Body) = 1,
        QuadrupedMedium(body: quadruped_medium::Body) = 2,
        BirdMedium(body: bird_medium::Body) = 3,
        FishMedium(body: fish_medium::Body) = 4,
        Dragon(body: dragon::Body) = 5,
        BirdSmall(body: bird_small::Body) = 6,
        FishSmall(body: fish_small::Body) = 7,
        BipedLarge(body: biped_large::Body)= 8,
        Object(body: object::Body) = 9,
        Golem(body: golem::Body) = 10,
        Theropod(body: theropod::Body) = 11,
        QuadrupedLow(body: quadruped_low::Body) = 12,
    }
);

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
    pub fish_medium: BodyData<BodyMeta, fish_medium::AllSpecies<SpeciesMeta>>,
    pub dragon: BodyData<BodyMeta, dragon::AllSpecies<SpeciesMeta>>,
    pub bird_small: BodyData<BodyMeta, ()>,
    pub fish_small: BodyData<BodyMeta, fish_small::AllSpecies<SpeciesMeta>>,
    pub biped_large: BodyData<BodyMeta, biped_large::AllSpecies<SpeciesMeta>>,
    pub object: BodyData<BodyMeta, ()>,
    pub golem: BodyData<BodyMeta, golem::AllSpecies<SpeciesMeta>>,
    pub theropod: BodyData<BodyMeta, theropod::AllSpecies<SpeciesMeta>>,
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
            NpcKind::Marlin => &self.fish_medium.body,
            NpcKind::Clownfish => &self.fish_small.body,
            NpcKind::Ogre => &self.biped_large.body,
            NpcKind::StoneGolem => &self.golem.body,
            NpcKind::Archaeos => &self.theropod.body,
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
            Body::Theropod(_) => &self.theropod.body,
            Body::QuadrupedLow(_) => &self.quadruped_low.body,
        }
    }
}

impl<
    BodyMeta: Send + Sync + for<'de> serde::Deserialize<'de> + 'static,
    SpeciesMeta: Send + Sync + for<'de> serde::Deserialize<'de> + 'static,
> Asset for AllBodies<BodyMeta, SpeciesMeta>
{
    const EXTENSION: &'static str = "json";
    type Loader = assets::JsonLoader;
}

impl Body {
    pub fn is_humanoid(&self) -> bool { matches!(self, Body::Humanoid(_)) }

    // Note: this might need to be refined to something more complex for realistic
    // behavior with less cylindrical bodies (e.g. wolfs)
    #[allow(unreachable_patterns)]
    pub fn radius(&self) -> f32 {
        // TODO: Improve these values (some might be reliant on more info in inner type)
        match self {
            Body::Humanoid(humanoid) => match (humanoid.species, humanoid.body_type) {
                (humanoid::Species::Orc, humanoid::BodyType::Male) => 0.57,
                (humanoid::Species::Orc, humanoid::BodyType::Female) => 0.51,
                (humanoid::Species::Human, humanoid::BodyType::Male) => 0.51,
                (humanoid::Species::Human, humanoid::BodyType::Female) => 0.48,
                (humanoid::Species::Elf, humanoid::BodyType::Male) => 0.51,
                (humanoid::Species::Elf, humanoid::BodyType::Female) => 0.48,
                (humanoid::Species::Dwarf, humanoid::BodyType::Male) => 0.42,
                (humanoid::Species::Dwarf, humanoid::BodyType::Female) => 0.39,
                (humanoid::Species::Undead, humanoid::BodyType::Male) => 0.48,
                (humanoid::Species::Undead, humanoid::BodyType::Female) => 0.45,
                (humanoid::Species::Danari, humanoid::BodyType::Male) => 0.348,
                (humanoid::Species::Danari, humanoid::BodyType::Female) => 0.348,
                _ => 0.5,
            },
            Body::QuadrupedSmall(_) => 0.4,
            Body::QuadrupedMedium(body) => match body.species {
                quadruped_medium::Species::Grolgar => 1.9,
                quadruped_medium::Species::Tarasque => 1.8,
                quadruped_medium::Species::Lion => 1.9,
                quadruped_medium::Species::Saber => 1.8,
                quadruped_medium::Species::Catoblepas => 1.7,
                _ => 1.5,
            },
            Body::QuadrupedLow(body) => match body.species {
                quadruped_low::Species::Asp => 1.8,
                quadruped_low::Species::Monitor => 1.75,
                quadruped_low::Species::Crocodile => 2.1,
                quadruped_low::Species::Salamander => 1.9,
                quadruped_low::Species::Pangolin => 1.3,
                _ => 1.6,
            },
            Body::Theropod(body) => match body.species {
                theropod::Species::Snowraptor => 0.5,
                theropod::Species::Sandraptor => 0.5,
                theropod::Species::Woodraptor => 0.5,
                _ => 1.8,
            },
            Body::BirdMedium(_) => 0.35,
            Body::FishMedium(_) => 0.35,
            Body::Dragon(_) => 8.0,
            Body::BirdSmall(_) => 0.3,
            Body::FishSmall(_) => 0.3,
            Body::BipedLarge(_) => 0.75,
            Body::Golem(_) => 0.4,
            Body::Object(_) => 0.4,
        }
    }

    pub fn height(&self) -> f32 {
        match self {
            Body::Humanoid(humanoid) => match (humanoid.species, humanoid.body_type) {
                (humanoid::Species::Orc, humanoid::BodyType::Male) => 2.17,
                (humanoid::Species::Orc, humanoid::BodyType::Female) => 1.94,
                (humanoid::Species::Human, humanoid::BodyType::Male) => 1.94,
                (humanoid::Species::Human, humanoid::BodyType::Female) => 1.82,
                (humanoid::Species::Elf, humanoid::BodyType::Male) => 1.94,
                (humanoid::Species::Elf, humanoid::BodyType::Female) => 1.82,
                (humanoid::Species::Dwarf, humanoid::BodyType::Male) => 1.60,
                (humanoid::Species::Dwarf, humanoid::BodyType::Female) => 1.48,
                (humanoid::Species::Undead, humanoid::BodyType::Male) => 1.82,
                (humanoid::Species::Undead, humanoid::BodyType::Female) => 1.71,
                (humanoid::Species::Danari, humanoid::BodyType::Male) => 1.32,
                (humanoid::Species::Danari, humanoid::BodyType::Female) => 1.32,
            },
            Body::QuadrupedSmall(body) => match body.species {
                quadruped_small::Species::Dodarock => 1.5,
                quadruped_small::Species::Holladon => 1.5,
                quadruped_small::Species::Truffler => 2.0,
                _ => 1.0,
            },
            Body::QuadrupedMedium(body) => match body.species {
                quadruped_medium::Species::Tarasque => 2.5,
                quadruped_medium::Species::Lion => 1.8,
                quadruped_medium::Species::Saber => 1.8,
                quadruped_medium::Species::Catoblepas => 2.8,
                _ => 1.6,
            },
            Body::QuadrupedLow(body) => match body.species {
                quadruped_low::Species::Monitor => 1.5,
                quadruped_low::Species::Tortoise => 2.0,
                quadruped_low::Species::Rocksnapper => 2.9,
                quadruped_low::Species::Maneater => 4.0,
                _ => 1.3,
            },
            Body::Theropod(body) => match body.species {
                theropod::Species::Snowraptor => 2.5,
                theropod::Species::Sandraptor => 2.5,
                theropod::Species::Woodraptor => 2.5,
                _ => 8.0,
            },
            Body::BirdMedium(body) => match body.species {
                bird_medium::Species::Cockatrice => 1.8,
                _ => 1.1,
            },
            Body::FishMedium(_) => 0.8,
            Body::Dragon(_) => 16.0,
            Body::BirdSmall(_) => 1.1,
            Body::FishSmall(_) => 0.6,
            Body::BipedLarge(body) => match body.species {
                biped_large::Species::Slysaurok => 2.3,
                biped_large::Species::Occultsaurok => 2.8,
                biped_large::Species::Mightysaurok => 2.3,
                _ => 4.6,
            },
            Body::Golem(_) => 5.8,
            Body::Object(_) => 1.0,
        }
    }

    pub fn base_energy(&self) -> u32 {
        match self {
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Dullahan => 4000,
                _ => 3000,
            },
            _ => 1000,
        }
    }

    #[allow(unreachable_patterns)]
    pub fn base_health(&self) -> u32 {
        match self {
            Body::Humanoid(_) => 400,
            Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
                quadruped_small::Species::Boar => 360,
                quadruped_small::Species::Batfox => 200,
                quadruped_small::Species::Dodarock => 640,
                quadruped_small::Species::Holladon => 500,
                quadruped_small::Species::Hyena => 300,
                quadruped_small::Species::Truffler => 360,
                _ => 200,
            },
            Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                quadruped_medium::Species::Grolgar => 600,
                quadruped_medium::Species::Saber => 400,
                quadruped_medium::Species::Tiger => 400,
                quadruped_medium::Species::Tuskram => 600,
                quadruped_medium::Species::Lion => 800,
                quadruped_medium::Species::Tarasque => 1200,
                quadruped_medium::Species::Wolf => 400,
                quadruped_medium::Species::Frostfang => 400,
                quadruped_medium::Species::Mouflon => 500,
                quadruped_medium::Species::Catoblepas => 1000,
                quadruped_medium::Species::Bonerattler => 400,
                quadruped_medium::Species::Deer => 300,
                quadruped_medium::Species::Hirdrasil => 500,
                quadruped_medium::Species::Roshwalr => 600,
                quadruped_medium::Species::Donkey => 500,
                quadruped_medium::Species::Camel => 600,
                quadruped_medium::Species::Zebra => 500,
                quadruped_medium::Species::Antelope => 300,
                quadruped_medium::Species::Kelpie => 600,
                quadruped_medium::Species::Horse => 600,
                _ => 400,
            },
            Body::BirdMedium(bird_medium) => match bird_medium.species {
                bird_medium::Species::Chicken => 50,
                bird_medium::Species::Duck => 50,
                bird_medium::Species::Goose => 60,
                bird_medium::Species::Parrot => 60,
                bird_medium::Species::Peacock => 60,
                bird_medium::Species::Cockatrice => 400,
                bird_medium::Species::Eagle => 400,
                _ => 100,
            },
            Body::FishMedium(_) => 50,
            Body::Dragon(_) => 5000,
            Body::BirdSmall(_) => 50,
            Body::FishSmall(_) => 20,
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Ogre => 2500,
                biped_large::Species::Cyclops => 2000,
                biped_large::Species::Wendigo => 2000,
                biped_large::Species::Troll => 1500,
                biped_large::Species::Dullahan => 2000,
                biped_large::Species::Mindflayer => 8000,
                _ => 1000,
            },
            Body::Object(_) => 10000,
            Body::Golem(_) => 2740,
            Body::Theropod(theropod) => match theropod.species {
                theropod::Species::Archaeos => 3000,
                theropod::Species::Odonto => 2700,
                _ => 1100,
            },
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Crocodile => 600,
                quadruped_low::Species::Alligator => 600,
                quadruped_low::Species::Salamander => 400,
                quadruped_low::Species::Monitor => 150,
                quadruped_low::Species::Asp => 400,
                quadruped_low::Species::Tortoise => 600,
                quadruped_low::Species::Rocksnapper => 1000,
                quadruped_low::Species::Pangolin => 80,
                quadruped_low::Species::Maneater => 400,
                quadruped_low::Species::Sandshark => 600,
                quadruped_low::Species::Hakulaq => 400,
                quadruped_low::Species::Lavadrake => 900,

                _ => 200,
            },
        }
    }

    #[allow(unreachable_patterns)]
    pub fn base_health_increase(&self) -> u32 {
        match self {
            Body::Humanoid(_) => 50,
            Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
                quadruped_small::Species::Boar => 20,
                quadruped_small::Species::Batfox => 10,
                quadruped_small::Species::Dodarock => 30,
                quadruped_small::Species::Holladon => 30,
                quadruped_small::Species::Hyena => 20,
                quadruped_small::Species::Truffler => 20,
                _ => 10,
            },
            Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                quadruped_medium::Species::Grolgar => 30,
                quadruped_medium::Species::Saber => 20,
                quadruped_medium::Species::Tiger => 20,
                quadruped_medium::Species::Tuskram => 30,
                quadruped_medium::Species::Lion => 40,
                quadruped_medium::Species::Tarasque => 60,
                quadruped_medium::Species::Wolf => 20,
                quadruped_medium::Species::Frostfang => 40,
                quadruped_medium::Species::Mouflon => 30,
                quadruped_medium::Species::Catoblepas => 50,
                quadruped_medium::Species::Bonerattler => 30,
                quadruped_medium::Species::Deer => 20,
                quadruped_medium::Species::Hirdrasil => 30,
                quadruped_medium::Species::Roshwalr => 40,
                quadruped_medium::Species::Donkey => 30,
                quadruped_medium::Species::Camel => 30,
                quadruped_medium::Species::Zebra => 30,
                quadruped_medium::Species::Antelope => 20,
                quadruped_medium::Species::Kelpie => 30,
                quadruped_medium::Species::Horse => 30,
                _ => 20,
            },
            Body::BirdMedium(bird_medium) => match bird_medium.species {
                bird_medium::Species::Chicken => 10,
                bird_medium::Species::Duck => 10,
                bird_medium::Species::Goose => 10,
                bird_medium::Species::Parrot => 10,
                bird_medium::Species::Peacock => 10,
                bird_medium::Species::Cockatrice => 10,
                bird_medium::Species::Eagle => 10,
                _ => 20,
            },
            Body::FishMedium(_) => 10,
            Body::Dragon(_) => 500,
            Body::BirdSmall(_) => 10,
            Body::FishSmall(_) => 10,
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Ogre => 70,
                biped_large::Species::Cyclops => 80,
                biped_large::Species::Wendigo => 80,
                biped_large::Species::Troll => 60,
                biped_large::Species::Dullahan => 120,
                biped_large::Species::Mindflayer => 250,
                _ => 100,
            },
            Body::Object(_) => 10,
            Body::Golem(_) => 260,
            Body::Theropod(_) => 20,
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Crocodile => 20,
                quadruped_low::Species::Alligator => 20,
                quadruped_low::Species::Salamander => 10,
                quadruped_low::Species::Monitor => 10,
                quadruped_low::Species::Asp => 10,
                quadruped_low::Species::Tortoise => 20,
                quadruped_low::Species::Rocksnapper => 50,
                quadruped_low::Species::Pangolin => 10,
                quadruped_low::Species::Maneater => 30,
                quadruped_low::Species::Sandshark => 40,
                quadruped_low::Species::Hakulaq => 10,
                _ => 20,
            },
        }
    }

    #[allow(unreachable_patterns)]
    pub fn base_exp(&self) -> u32 {
        match self {
            Body::Humanoid(_) => 5,
            Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
                quadruped_small::Species::Boar => 6,
                quadruped_small::Species::Batfox => 2,
                quadruped_small::Species::Dodarock => 6,
                quadruped_small::Species::Holladon => 8,
                quadruped_small::Species::Hyena => 6,
                quadruped_small::Species::Truffler => 6,
                _ => 4,
            },
            Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                quadruped_medium::Species::Grolgar => 10,
                quadruped_medium::Species::Saber => 8,
                quadruped_medium::Species::Tiger => 8,
                quadruped_medium::Species::Tuskram => 9,
                quadruped_medium::Species::Lion => 10,
                quadruped_medium::Species::Tarasque => 16,
                quadruped_medium::Species::Wolf => 8,
                quadruped_medium::Species::Frostfang => 9,
                quadruped_medium::Species::Mouflon => 7,
                quadruped_medium::Species::Catoblepas => 10,
                quadruped_medium::Species::Bonerattler => 10,
                quadruped_medium::Species::Deer => 6,
                quadruped_medium::Species::Hirdrasil => 9,
                quadruped_medium::Species::Roshwalr => 10,
                quadruped_medium::Species::Donkey => 8,
                quadruped_medium::Species::Camel => 8,
                quadruped_medium::Species::Zebra => 8,
                quadruped_medium::Species::Antelope => 6,
                quadruped_medium::Species::Kelpie => 8,
                quadruped_medium::Species::Horse => 8,
                _ => 6,
            },
            Body::BirdMedium(bird_medium) => match bird_medium.species {
                bird_medium::Species::Chicken => 2,
                bird_medium::Species::Duck => 2,
                bird_medium::Species::Goose => 4,
                bird_medium::Species::Parrot => 4,
                bird_medium::Species::Peacock => 5,
                _ => 8,
            },
            Body::FishMedium(_) => 2,
            Body::Dragon(_) => 1000,
            Body::BirdSmall(_) => 2,
            Body::FishSmall(_) => 2,
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Ogre => 60,
                biped_large::Species::Cyclops => 70,
                biped_large::Species::Wendigo => 70,
                biped_large::Species::Troll => 50,
                biped_large::Species::Dullahan => 100,
                biped_large::Species::Mindflayer => 150,
                _ => 100,
            },
            Body::Object(_) => 1,
            Body::Golem(_) => 256,
            Body::Theropod(theropod) => match theropod.species {
                theropod::Species::Archaeos => 90,
                theropod::Species::Odonto => 80,
                _ => 50,
            },
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Crocodile => 10,
                quadruped_low::Species::Alligator => 10,
                quadruped_low::Species::Salamander => 6,
                quadruped_low::Species::Monitor => 4,
                quadruped_low::Species::Asp => 4,
                quadruped_low::Species::Tortoise => 6,
                quadruped_low::Species::Rocksnapper => 12,
                quadruped_low::Species::Pangolin => 3,
                quadruped_low::Species::Maneater => 14,
                quadruped_low::Species::Sandshark => 12,
                quadruped_low::Species::Hakulaq => 10,
                quadruped_low::Species::Lavadrake => 20,
                _ => 10,
            },
        }
    }

    #[allow(unreachable_patterns)]
    pub fn base_dmg(&self) -> u32 {
        match self {
            Body::Humanoid(_) => 50,
            Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
                quadruped_small::Species::Dodarock => 30,
                quadruped_small::Species::Hyena => 40,
                quadruped_small::Species::Holladon => 40,
                quadruped_small::Species::Porcupine => 30,
                _ => 20,
            },
            Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                quadruped_medium::Species::Grolgar => 50,
                quadruped_medium::Species::Lion => 60,
                quadruped_medium::Species::Tarasque => 70,
                quadruped_medium::Species::Mouflon => 30,
                quadruped_medium::Species::Catoblepas => 20,
                quadruped_medium::Species::Bonerattler => 50,
                quadruped_medium::Species::Deer => 30,
                quadruped_medium::Species::Hirdrasil => 50,
                quadruped_medium::Species::Roshwalr => 60,
                quadruped_medium::Species::Donkey => 40,
                quadruped_medium::Species::Camel => 40,
                quadruped_medium::Species::Zebra => 40,
                quadruped_medium::Species::Antelope => 6,
                quadruped_medium::Species::Kelpie => 60,
                quadruped_medium::Species::Horse => 50,
                _ => 40,
            },
            Body::BirdMedium(bird_medium) => match bird_medium.species {
                bird_medium::Species::Chicken => 10,
                bird_medium::Species::Duck => 10,
                bird_medium::Species::Goose => 10,
                bird_medium::Species::Parrot => 20,
                bird_medium::Species::Peacock => 40,
                bird_medium::Species::Cockatrice => 60,
                bird_medium::Species::Eagle => 60,
                _ => 30,
            },
            Body::FishMedium(_) => 10,
            Body::Dragon(_) => 5000,
            Body::BirdSmall(_) => 10,
            Body::FishSmall(_) => 10,
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Ogre => 60,
                biped_large::Species::Cyclops => 60,
                biped_large::Species::Wendigo => 60,
                biped_large::Species::Troll => 60,
                biped_large::Species::Dullahan => 80,
                biped_large::Species::Mindflayer => 200,
                _ => 60,
            },
            Body::Object(_) => 0,
            Body::Golem(_) => 250,
            Body::Theropod(theropod) => match theropod.species {
                theropod::Species::Archaeos => 150,
                theropod::Species::Odonto => 130,
                _ => 70,
            },
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Crocodile => 50,
                quadruped_low::Species::Alligator => 50,
                quadruped_low::Species::Salamander => 50,
                quadruped_low::Species::Monitor => 30,
                quadruped_low::Species::Asp => 35,
                quadruped_low::Species::Tortoise => 10,
                quadruped_low::Species::Rocksnapper => 80,
                quadruped_low::Species::Pangolin => 10,
                quadruped_low::Species::Maneater => 40,
                quadruped_low::Species::Sandshark => 60,
                quadruped_low::Species::Hakulaq => 40,
                _ => 20,
            },
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
            Body::Theropod(_) => 3.0,
            Body::QuadrupedLow(_) => 4.5,
        }
    }

    /// Returns the eye height for this humanoid.
    pub fn eye_height(&self) -> f32 { self.height() * 0.9 }

    pub fn default_light_offset(&self) -> Vec3<f32> {
        // TODO: Make this a manifest
        match self {
            Body::Object(_) => Vec3::unit_z() * 0.5,
            _ => Vec3::unit_z(),
        }
    }

    pub fn can_strafe(&self) -> bool { matches!(self, Body::Humanoid(_)) }
}

impl Component for Body {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
