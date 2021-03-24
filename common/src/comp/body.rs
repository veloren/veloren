pub mod biped_large;
pub mod biped_small;
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
pub mod ship;
pub mod theropod;

use crate::{
    assets::{self, Asset},
    make_case_elim,
    npc::NpcKind,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use vek::*;

use super::BuffKind;

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
        BipedSmall(body: biped_small::Body)= 9,
        Object(body: object::Body) = 10,
        Golem(body: golem::Body) = 11,
        Theropod(body: theropod::Body) = 12,
        QuadrupedLow(body: quadruped_low::Body) = 13,
        Ship(body: ship::Body) = 14,
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
    pub biped_small: BodyData<BodyMeta, biped_small::AllSpecies<SpeciesMeta>>,
    pub object: BodyData<BodyMeta, ()>,
    pub golem: BodyData<BodyMeta, golem::AllSpecies<SpeciesMeta>>,
    pub theropod: BodyData<BodyMeta, theropod::AllSpecies<SpeciesMeta>>,
    pub quadruped_low: BodyData<BodyMeta, quadruped_low::AllSpecies<SpeciesMeta>>,
    pub ship: BodyData<BodyMeta, ()>,
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
            NpcKind::Gnome => &self.biped_small.body,
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
            Body::BipedSmall(_) => &self.biped_small.body,
            Body::Object(_) => &self.object.body,
            Body::Golem(_) => &self.golem.body,
            Body::Theropod(_) => &self.theropod.body,
            Body::QuadrupedLow(_) => &self.quadruped_low.body,
            Body::Ship(_) => &self.ship.body,
        }
    }
}

impl<
    BodyMeta: Send + Sync + for<'de> serde::Deserialize<'de> + 'static,
    SpeciesMeta: Send + Sync + for<'de> serde::Deserialize<'de> + 'static,
> Asset for AllBodies<BodyMeta, SpeciesMeta>
{
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
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
                (humanoid::Species::Orc, humanoid::BodyType::Male) => 0.75,
                (humanoid::Species::Orc, humanoid::BodyType::Female) => 0.75,
                (humanoid::Species::Human, humanoid::BodyType::Male) => 0.75,
                (humanoid::Species::Human, humanoid::BodyType::Female) => 0.75,
                (humanoid::Species::Elf, humanoid::BodyType::Male) => 0.75,
                (humanoid::Species::Elf, humanoid::BodyType::Female) => 0.75,
                (humanoid::Species::Dwarf, humanoid::BodyType::Male) => 0.75,
                (humanoid::Species::Dwarf, humanoid::BodyType::Female) => 0.75,
                (humanoid::Species::Undead, humanoid::BodyType::Male) => 0.75,
                (humanoid::Species::Undead, humanoid::BodyType::Female) => 0.75,
                (humanoid::Species::Danari, humanoid::BodyType::Male) => 0.75,
                (humanoid::Species::Danari, humanoid::BodyType::Female) => 0.75,
                _ => 0.75,
            },
            Body::QuadrupedSmall(_) => 0.6,
            Body::QuadrupedMedium(body) => match body.species {
                quadruped_medium::Species::Grolgar => 2.0,
                quadruped_medium::Species::Tarasque => 2.0,
                quadruped_medium::Species::Lion => 2.0,
                quadruped_medium::Species::Saber => 2.0,
                quadruped_medium::Species::Catoblepas => 2.0,
                quadruped_medium::Species::Horse => 1.5,
                quadruped_medium::Species::Deer => 1.5,
                quadruped_medium::Species::Donkey => 1.5,
                quadruped_medium::Species::Kelpie => 1.5,
                quadruped_medium::Species::Barghest => 1.8,
                quadruped_medium::Species::Cattle => 1.8,
                quadruped_medium::Species::Highland => 1.8,
                quadruped_medium::Species::Yak => 1.8,
                quadruped_medium::Species::Panda => 1.8,
                quadruped_medium::Species::Bear => 1.8,
                _ => 1.5,
            },
            Body::QuadrupedLow(body) => match body.species {
                quadruped_low::Species::Asp => 2.5,
                quadruped_low::Species::Monitor => 2.3,
                quadruped_low::Species::Crocodile => 2.4,
                quadruped_low::Species::Salamander => 2.4,
                quadruped_low::Species::Pangolin => 2.0,
                quadruped_low::Species::Lavadrake => 2.5,
                quadruped_low::Species::Deadwood => 0.5,
                _ => 1.6,
            },
            Body::Theropod(body) => match body.species {
                theropod::Species::Snowraptor => 1.5,
                theropod::Species::Sandraptor => 1.5,
                theropod::Species::Woodraptor => 1.5,
                theropod::Species::Archaeos => 3.5,
                theropod::Species::Odonto => 3.5,
                theropod::Species::Yale => 0.8,
                theropod::Species::Ntouka => 3.0,
                _ => 1.8,
            },
            Body::BirdMedium(_) => 1.0,
            Body::FishMedium(_) => 1.0,
            Body::Dragon(_) => 8.0,
            Body::BirdSmall(_) => 0.6,
            Body::FishSmall(_) => 0.6,
            Body::BipedLarge(body) => match body.species {
                biped_large::Species::Slysaurok => 2.0,
                biped_large::Species::Occultsaurok => 2.0,
                biped_large::Species::Mightysaurok => 2.0,
                biped_large::Species::Mindflayer => 2.2,
                biped_large::Species::Minotaur => 3.0,

                _ => 2.3,
            },
            Body::Golem(_) => 2.5,
            Body::BipedSmall(_) => 0.75,
            Body::Object(_) => 0.4,
            Body::Ship(_) => 1.0,
        }
    }

    pub fn height(&self) -> f32 {
        match self {
            Body::Humanoid(humanoid) => match (humanoid.species, humanoid.body_type) {
                (humanoid::Species::Orc, humanoid::BodyType::Male) => 2.3,
                (humanoid::Species::Orc, humanoid::BodyType::Female) => 2.2,
                (humanoid::Species::Human, humanoid::BodyType::Male) => 2.3,
                (humanoid::Species::Human, humanoid::BodyType::Female) => 2.2,
                (humanoid::Species::Elf, humanoid::BodyType::Male) => 2.3,
                (humanoid::Species::Elf, humanoid::BodyType::Female) => 2.2,
                (humanoid::Species::Dwarf, humanoid::BodyType::Male) => 1.9,
                (humanoid::Species::Dwarf, humanoid::BodyType::Female) => 1.8,
                (humanoid::Species::Undead, humanoid::BodyType::Male) => 2.2,
                (humanoid::Species::Undead, humanoid::BodyType::Female) => 2.1,
                (humanoid::Species::Danari, humanoid::BodyType::Male) => 1.5,
                (humanoid::Species::Danari, humanoid::BodyType::Female) => 1.4,
            },
            Body::QuadrupedSmall(body) => match body.species {
                quadruped_small::Species::Dodarock => 1.5,
                quadruped_small::Species::Holladon => 1.5,
                quadruped_small::Species::Truffler => 2.0,
                _ => 1.0,
            },
            Body::QuadrupedMedium(body) => match body.species {
                quadruped_medium::Species::Tarasque => 2.6,
                quadruped_medium::Species::Lion => 2.0,
                quadruped_medium::Species::Saber => 2.0,
                quadruped_medium::Species::Catoblepas => 2.9,
                quadruped_medium::Species::Barghest => 2.5,
                quadruped_medium::Species::Dreadhorn => 2.5,
                quadruped_medium::Species::Moose => 2.5,
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
                theropod::Species::Snowraptor => 2.6,
                theropod::Species::Sandraptor => 2.6,
                theropod::Species::Woodraptor => 2.6,
                theropod::Species::Sunlizard => 2.5,
                theropod::Species::Yale => 3.0,
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
                biped_large::Species::Slysaurok => 3.4,
                biped_large::Species::Occultsaurok => 3.4,
                biped_large::Species::Mightysaurok => 3.4,
                biped_large::Species::Mindflayer => 8.0,
                biped_large::Species::Minotaur => 8.0,
                biped_large::Species::Dullahan => 5.5,
                biped_large::Species::Cyclops => 6.5,
                biped_large::Species::Werewolf => 3.5,

                _ => 6.0,
            },
            Body::Golem(_) => 5.0,
            Body::BipedSmall(_) => 1.4,
            Body::Object(object) => match object {
                object::Body::Crossbow => 1.7,
                _ => 1.0,
            },
            Body::Ship(_) => 1.0,
        }
    }

    pub fn base_energy(&self) -> u32 {
        match self {
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Dullahan => 4000,
                _ => 3000,
            },
            Body::Humanoid(_) => 750,
            _ => 1000,
        }
    }

    #[allow(unreachable_patterns)]
    pub fn base_health(&self) -> u32 {
        match self {
            Body::Humanoid(_) => 500,
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
                quadruped_medium::Species::Barghest => 1700,
                quadruped_medium::Species::Cattle => 1000,
                quadruped_medium::Species::Highland => 1200,
                quadruped_medium::Species::Yak => 1000,
                quadruped_medium::Species::Panda => 800,
                quadruped_medium::Species::Bear => 800,
                quadruped_medium::Species::Moose => 600,
                quadruped_medium::Species::Dreadhorn => 1100,
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
                biped_large::Species::Tidalwarrior => 2500,
                biped_large::Species::Yeti => 2000,
                biped_large::Species::Minotaur => 5000,
                biped_large::Species::Harvester => 2000,
                biped_large::Species::Blueoni => 2300,
                biped_large::Species::Redoni => 2300,
                _ => 1000,
            },
            Body::BipedSmall(biped_small) => match biped_small.species {
                biped_small::Species::Gnarling => 300,
                biped_small::Species::Adlet => 400,
                biped_small::Species::Sahagin => 500,
                biped_small::Species::Haniwa => 700,
                biped_small::Species::Myrmidon => 800,
                biped_small::Species::Husk => 200,
                _ => 600,
            },
            Body::Object(object) => match object {
                object::Body::TrainingDummy => 10000,
                object::Body::Crossbow => 800,
                _ => 10000,
            },
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
                quadruped_low::Species::Sandshark => 800,
                quadruped_low::Species::Hakulaq => 400,
                quadruped_low::Species::Lavadrake => 900,
                quadruped_low::Species::Deadwood => 600,
                _ => 200,
            },
            Body::Ship(_) => 10000,
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
                quadruped_medium::Species::Barghest => 50,
                quadruped_medium::Species::Cattle => 30,
                quadruped_medium::Species::Highland => 30,
                quadruped_medium::Species::Yak => 30,
                quadruped_medium::Species::Panda => 40,
                quadruped_medium::Species::Bear => 40,
                quadruped_medium::Species::Moose => 30,
                quadruped_medium::Species::Dreadhorn => 50,
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
                biped_large::Species::Tidalwarrior => 90,
                biped_large::Species::Yeti => 80,
                biped_large::Species::Harvester => 80,
                _ => 100,
            },
            Body::BipedSmall(_) => 10,
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
                quadruped_low::Species::Deadwood => 30,
                _ => 20,
            },
            Body::Ship(_) => 500,
        }
    }

    pub fn flying_height(&self) -> f32 {
        match self {
            Body::BirdSmall(_) => 30.0,
            Body::BirdMedium(_) => 40.0,
            Body::Dragon(_) => 60.0,
            Body::Ship(ship::Body::DefaultAirship) => 60.0,
            _ => 0.0,
        }
    }

    pub fn immune_to(&self, buff: BuffKind) -> bool {
        match buff {
            BuffKind::Bleeding => matches!(self, Body::Object(_) | Body::Golem(_) | Body::Ship(_)),
            _ => false,
        }
    }

    /// Returns a multiplier representing increased difficulty not accounted for
    /// due to AI or not using an actual weapon
    // TODO: Match on species
    pub fn combat_multiplier(&self) -> f32 {
        if let Body::Object(_) | Body::Ship(_) = self {
            0.0
        } else {
            1.0
        }
    }

    pub fn base_poise(&self) -> u32 {
        match self {
            Body::Humanoid(_) => 100,
            Body::BipedLarge(_) => 250,
            Body::Golem(_) => 300,
            _ => 100,
        }
    }

    /// Returns the eye height for this creature.
    pub fn eye_height(&self) -> f32 { self.height() * 0.9 }

    pub fn default_light_offset(&self) -> Vec3<f32> {
        // TODO: Make this a manifest
        match self {
            Body::Object(_) => Vec3::unit_z() * 0.5,
            _ => Vec3::unit_z(),
        }
    }

    pub fn can_strafe(&self) -> bool {
        matches!(
            self,
            Body::Humanoid(_) | Body::BipedSmall(_) | Body::BipedLarge(_)
        )
    }

    pub fn mounting_offset(&self) -> Vec3<f32> {
        match self {
            Body::Ship(ship::Body::DefaultAirship) => Vec3::from([0.0, 0.0, 10.0]),
            _ => Vec3::unit_z(),
        }
    }
}

impl Component for Body {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
