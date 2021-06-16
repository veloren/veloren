pub mod biped_large;
pub mod biped_small;
pub mod bird_large;
pub mod bird_medium;
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
    consts::{HUMAN_DENSITY, WATER_DENSITY},
    make_case_elim,
    npc::NpcKind,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use vek::*;

use super::{BuffKind, Density, Mass};

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
        BirdLarge(body: bird_large::Body) = 6,
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
/// NOTE: If you are adding new body kind and it should be spawned via /spawn
/// please add it to `[ENTITIES](crate::cmd::ENTITIES)`
#[derive(Clone, Debug, Deserialize)]
pub struct AllBodies<BodyMeta, SpeciesMeta> {
    pub humanoid: BodyData<BodyMeta, humanoid::AllSpecies<SpeciesMeta>>,
    pub quadruped_small: BodyData<BodyMeta, quadruped_small::AllSpecies<SpeciesMeta>>,
    pub quadruped_medium: BodyData<BodyMeta, quadruped_medium::AllSpecies<SpeciesMeta>>,
    pub bird_medium: BodyData<BodyMeta, bird_medium::AllSpecies<SpeciesMeta>>,
    pub fish_medium: BodyData<BodyMeta, fish_medium::AllSpecies<SpeciesMeta>>,
    pub dragon: BodyData<BodyMeta, dragon::AllSpecies<SpeciesMeta>>,
    pub bird_large: BodyData<BodyMeta, bird_large::AllSpecies<SpeciesMeta>>,
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
            NpcKind::Phoenix => &self.bird_large.body,
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
            Body::BirdLarge(_) => &self.bird_large.body,
            Body::FishMedium(_) => &self.fish_medium.body,
            Body::Dragon(_) => &self.dragon.body,
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

    /// Average density of the body
    // Units are based on kg/m³
    pub fn density(&self) -> Density {
        let d = match self {
            // based on a house sparrow (Passer domesticus)
            Body::BirdMedium(_) => 700.0,
            Body::BirdLarge(_) => 2_200.0,

            // based on its mass divided by the volume of a bird scaled up to the size of the dragon
            Body::Dragon(_) => 3_700.0,

            Body::Golem(_) => WATER_DENSITY * 2.5,
            Body::Humanoid(_) => HUMAN_DENSITY,
            Body::Ship(ship) => ship.density().0,
            Body::Object(object) => object.density().0,
            _ => HUMAN_DENSITY,
        };
        Density(d)
    }

    // Values marked with ~✅ are checked based on their RL equivalent.
    // Discrepancy in size compared to their RL equivalents has not necessarily been
    // taken into account.
    pub fn mass(&self) -> Mass {
        let m = match self {
            Body::BipedLarge(body) => match body.species {
                biped_large::Species::Slysaurok => 400.0,
                biped_large::Species::Occultsaurok => 400.0,
                biped_large::Species::Mightysaurok => 400.0,
                biped_large::Species::Mindflayer => 420.0,
                biped_large::Species::Minotaur => 500.0,
                _ => 400.0,
            },
            Body::BipedSmall(_) => 50.0,

            // ravens are 0.69-2 kg, crows are 0.51 kg on average.
            Body::BirdMedium(body) => match body.species {
                bird_medium::Species::Chicken => 2.0, // ~✅ Red junglefowl are 1-1.5 kg
                bird_medium::Species::Duck => 2.0,
                bird_medium::Species::Eagle => 10.0, // ~✅ Steller's sea eagle are 5-9 kg
                bird_medium::Species::Goose => 3.5,  // ~✅ Swan geese are 2.8-3.5 kg
                bird_medium::Species::Owl => 2.0,
                bird_medium::Species::Parrot => 2.0,
                bird_medium::Species::Peacock => 5.0,
            },
            Body::BirdLarge(_) => 100.0,

            Body::Dragon(_) => 20_000.0,
            Body::FishMedium(_) => 5.0,
            Body::FishSmall(_) => 1.0,
            Body::Golem(_) => 10_000.0,
            Body::Humanoid(humanoid) => {
                match (humanoid.species, humanoid.body_type) {
                    (humanoid::Species::Orc, humanoid::BodyType::Male) => 120.0,
                    (humanoid::Species::Orc, humanoid::BodyType::Female) => 120.0,
                    (humanoid::Species::Human, humanoid::BodyType::Male) => 77.0, // ~✅
                    (humanoid::Species::Human, humanoid::BodyType::Female) => 59.0, // ~✅
                    (humanoid::Species::Elf, humanoid::BodyType::Male) => 77.0,
                    (humanoid::Species::Elf, humanoid::BodyType::Female) => 59.0,
                    (humanoid::Species::Dwarf, humanoid::BodyType::Male) => 70.0,
                    (humanoid::Species::Dwarf, humanoid::BodyType::Female) => 70.0,
                    (humanoid::Species::Undead, humanoid::BodyType::Male) => 70.0,
                    (humanoid::Species::Undead, humanoid::BodyType::Female) => 50.0,
                    (humanoid::Species::Danari, humanoid::BodyType::Male) => 80.0,
                    (humanoid::Species::Danari, humanoid::BodyType::Female) => 60.0,
                }
            },
            Body::Object(obj) => obj.mass().0,
            Body::QuadrupedLow(body) => match body.species {
                quadruped_low::Species::Alligator => 360.0, // ~✅
                quadruped_low::Species::Asp => 300.0,
                // saltwater crocodiles can weigh around 1 ton, but our version is the size of an
                // alligator or smaller, so whatever
                quadruped_low::Species::Crocodile => 360.0,
                quadruped_low::Species::Deadwood => 400.0,
                quadruped_low::Species::Lavadrake => 500.0,
                quadruped_low::Species::Monitor => 100.0,
                quadruped_low::Species::Pangolin => 100.0,
                quadruped_low::Species::Salamander => 65.0,
                quadruped_low::Species::Tortoise => 200.0,
                _ => 200.0,
            },
            Body::QuadrupedMedium(body) => match body.species {
                quadruped_medium::Species::Bear => 500.0, // ~✅ (350-700 kg)
                quadruped_medium::Species::Cattle => 575.0, // ~✅ (500-650 kg)
                quadruped_medium::Species::Deer => 80.0,
                quadruped_medium::Species::Donkey => 200.0,
                quadruped_medium::Species::Highland => 200.0,
                quadruped_medium::Species::Horse => 500.0, // ~✅
                quadruped_medium::Species::Kelpie => 200.0,
                quadruped_medium::Species::Lion => 170.0, // ~✅ (110-225 kg)
                quadruped_medium::Species::Panda => 200.0,
                quadruped_medium::Species::Saber => 130.0,
                quadruped_medium::Species::Yak => 200.0,
                _ => 200.0,
            },
            Body::QuadrupedSmall(body) => match body.species {
                quadruped_small::Species::Axolotl => 1.0,
                quadruped_small::Species::Batfox => 10.0,
                quadruped_small::Species::Beaver => 10.0,
                quadruped_small::Species::Boar => 80.0, // ~✅ (60-100 kg)
                quadruped_small::Species::Cat => 4.0,   // ~✅ (4-5 kg)
                quadruped_small::Species::Dodarock => 500.0,
                quadruped_small::Species::Dog => 30.0, // ~✅ (German Shepherd: 30-40 kg)
                quadruped_small::Species::Fox => 10.0,
                quadruped_small::Species::Frog => 1.0,
                quadruped_small::Species::Fungome => 10.0,
                quadruped_small::Species::Gecko => 1.0,
                quadruped_small::Species::Goat => 50.0,
                quadruped_small::Species::Hare => 10.0,
                quadruped_small::Species::Holladon => 60.0,
                quadruped_small::Species::Hyena => 70.0, // ~✅ (vaguely)
                quadruped_small::Species::Jackalope => 10.0,
                quadruped_small::Species::Pig => 20.0,
                quadruped_small::Species::Porcupine => 5.0,
                quadruped_small::Species::Quokka => 10.0,
                quadruped_small::Species::Rabbit => 2.0,
                quadruped_small::Species::Raccoon => 30.0,
                quadruped_small::Species::Rat => 1.0,
                quadruped_small::Species::Sheep => 50.0,
                quadruped_small::Species::Skunk => 5.0,
                quadruped_small::Species::Squirrel => 1.0,
                quadruped_small::Species::Truffler => 70.0,
                quadruped_small::Species::Turtle => 40.0,
            },
            Body::Theropod(body) => match body.species {
                // for reference, elephants are in the range of 2.6-6.9 tons
                // and Tyrannosaurus rex were ~8.4-14 tons
                theropod::Species::Archaeos => 13_000.0,
                theropod::Species::Ntouka => 13_000.0,
                theropod::Species::Odonto => 13_000.0,

                theropod::Species::Sandraptor => 500.0,
                theropod::Species::Snowraptor => 500.0,
                theropod::Species::Sunlizard => 500.0,
                theropod::Species::Woodraptor => 500.0,
                theropod::Species::Yale => 1_000.0,
            },
            Body::Ship(ship) => ship.mass().0,
        };
        Mass(m)
    }

    /// The width (shoulder to shoulder), length (nose to tail) and height
    /// respectively
    pub fn dimensions(&self) -> Vec3<f32> {
        match self {
            Body::BipedLarge(body) => match body.species {
                biped_large::Species::Cyclops => Vec3::new(4.6, 3.0, 6.5),
                biped_large::Species::Dullahan => Vec3::new(4.6, 3.0, 5.5),
                biped_large::Species::Mightysaurok => Vec3::new(4.0, 3.0, 3.4),
                biped_large::Species::Mindflayer => Vec3::new(4.4, 3.0, 8.0),
                biped_large::Species::Minotaur => Vec3::new(6.0, 3.0, 8.0),
                biped_large::Species::Occultsaurok => Vec3::new(4.0, 3.0, 3.4),
                biped_large::Species::Slysaurok => Vec3::new(4.0, 3.0, 3.4),
                biped_large::Species::Werewolf => Vec3::new(4.0, 3.0, 3.5),
                biped_large::Species::Harvester => Vec3::new(4.6, 3.0, 5.4),
                biped_large::Species::Cultistwarlord => Vec3::new(3.0, 3.0, 4.5),
                _ => Vec3::new(4.6, 3.0, 6.0),
            },
            Body::BipedSmall(body) => match body.species {
                biped_small::Species::Gnarling => Vec3::new(1.0, 0.75, 1.4),
                biped_small::Species::Haniwa => Vec3::new(1.0, 0.75, 2.2),
                biped_small::Species::Adlet => Vec3::new(1.0, 0.75, 2.0),
                biped_small::Species::Sahagin => Vec3::new(1.0, 1.2, 1.7),
                biped_small::Species::Myrmidon => Vec3::new(1.0, 0.75, 2.2),
                biped_small::Species::Husk => Vec3::new(1.0, 0.75, 1.7),

                _ => Vec3::new(1.0, 0.75, 1.4),
            },
            Body::BirdMedium(_) => Vec3::new(2.0, 1.0, 1.5),
            Body::BirdLarge(_) => Vec3::new(2.0, 6.0, 3.5),
            Body::Dragon(_) => Vec3::new(16.0, 10.0, 16.0),
            Body::FishMedium(_) => Vec3::new(0.5, 2.0, 0.8),
            Body::FishSmall(_) => Vec3::new(0.3, 1.2, 0.6),
            Body::Golem(_) => Vec3::new(5.0, 5.0, 7.5),
            Body::Humanoid(humanoid) => {
                let height = match (humanoid.species, humanoid.body_type) {
                    (humanoid::Species::Orc, humanoid::BodyType::Male) => 2.0,
                    (humanoid::Species::Orc, humanoid::BodyType::Female) => 1.9,
                    (humanoid::Species::Human, humanoid::BodyType::Male) => 1.8,
                    (humanoid::Species::Human, humanoid::BodyType::Female) => 1.7,
                    (humanoid::Species::Elf, humanoid::BodyType::Male) => 1.9,
                    (humanoid::Species::Elf, humanoid::BodyType::Female) => 1.8,
                    (humanoid::Species::Dwarf, humanoid::BodyType::Male) => 1.6,
                    (humanoid::Species::Dwarf, humanoid::BodyType::Female) => 1.5,
                    (humanoid::Species::Undead, humanoid::BodyType::Male) => 1.9,
                    (humanoid::Species::Undead, humanoid::BodyType::Female) => 1.8,
                    (humanoid::Species::Danari, humanoid::BodyType::Male) => 1.5,
                    (humanoid::Species::Danari, humanoid::BodyType::Female) => 1.4,
                };
                Vec3::new(1.5, 0.5, height)
            },
            Body::Object(object) => object.dimensions(),
            Body::QuadrupedMedium(body) => match body.species {
                quadruped_medium::Species::Barghest => Vec3::new(2.0, 4.4, 2.7),
                quadruped_medium::Species::Bear => Vec3::new(2.0, 3.8, 3.0),
                quadruped_medium::Species::Catoblepas => Vec3::new(2.0, 4.0, 2.9),
                quadruped_medium::Species::Cattle => Vec3::new(2.0, 3.6, 2.4),
                quadruped_medium::Species::Deer => Vec3::new(2.0, 3.0, 2.2),
                quadruped_medium::Species::Dreadhorn => Vec3::new(2.0, 5.0, 4.0),
                quadruped_medium::Species::Grolgar => Vec3::new(2.0, 4.0, 2.0),
                quadruped_medium::Species::Highland => Vec3::new(2.0, 3.6, 2.4),
                quadruped_medium::Species::Horse => Vec3::new(2.0, 3.0, 2.4),
                quadruped_medium::Species::Lion => Vec3::new(2.0, 3.3, 2.0),
                quadruped_medium::Species::Moose => Vec3::new(2.0, 4.0, 2.5),
                quadruped_medium::Species::Saber => Vec3::new(2.0, 3.0, 2.0),
                quadruped_medium::Species::Tarasque => Vec3::new(2.0, 4.0, 2.6),
                quadruped_medium::Species::Yak => Vec3::new(2.0, 3.6, 3.0),
                quadruped_medium::Species::Mammoth => Vec3::new(5.0, 7.0, 8.0),
                quadruped_medium::Species::Ngoubou => Vec3::new(2.0, 3.2, 2.4),
                _ => Vec3::new(2.0, 3.0, 2.0),
            },
            Body::QuadrupedSmall(body) => match body.species {
                quadruped_small::Species::Dodarock => Vec3::new(1.2, 1.8, 1.5),
                quadruped_small::Species::Holladon => Vec3::new(1.2, 1.6, 1.5),
                quadruped_small::Species::Truffler => Vec3::new(1.2, 1.8, 2.2),
                _ => Vec3::new(1.2, 1.2, 1.0),
            },
            Body::QuadrupedLow(body) => match body.species {
                quadruped_low::Species::Asp => Vec3::new(1.0, 3.0, 1.8),
                quadruped_low::Species::Crocodile => Vec3::new(1.0, 2.8, 1.3),
                quadruped_low::Species::Deadwood => Vec3::new(1.0, 1.4, 1.3),
                quadruped_low::Species::Lavadrake => Vec3::new(1.0, 3.0, 2.5),
                quadruped_low::Species::Maneater => Vec3::new(1.0, 2.2, 4.0),
                quadruped_low::Species::Monitor => Vec3::new(1.0, 2.3, 1.5),
                quadruped_low::Species::Pangolin => Vec3::new(1.0, 2.6, 1.1),
                quadruped_low::Species::Rocksnapper => Vec3::new(1.0, 3.0, 2.9),
                quadruped_low::Species::Basilisk => Vec3::new(1.8, 3.4, 2.9),
                quadruped_low::Species::Salamander => Vec3::new(1.0, 2.4, 1.3),
                quadruped_low::Species::Tortoise => Vec3::new(1.0, 1.8, 1.6),
                _ => Vec3::new(1.0, 1.6, 1.3),
            },
            Body::Ship(ship) => ship.dimensions(),
            Body::Theropod(body) => match body.species {
                theropod::Species::Archaeos => Vec3::new(4.0, 8.5, 8.0),
                theropod::Species::Ntouka => Vec3::new(4.0, 7.0, 8.0),
                theropod::Species::Odonto => Vec3::new(4.0, 7.0, 8.0),
                theropod::Species::Sandraptor => Vec3::new(2.0, 3.0, 2.6),
                theropod::Species::Snowraptor => Vec3::new(2.0, 3.0, 2.6),
                theropod::Species::Sunlizard => Vec3::new(2.0, 3.6, 2.5),
                theropod::Species::Woodraptor => Vec3::new(2.0, 3.0, 2.6),
                theropod::Species::Yale => Vec3::new(1.5, 3.2, 4.0),
            },
        }
    }

    // Note: This is used for collisions, but it's not very accurate for shapes that
    // are very much not cylindrical. Eventually this ought to be replaced by more
    // accurate collision shapes.
    pub fn radius(&self) -> f32 {
        let dim = self.dimensions();
        dim.x.max(dim.y) / 2.0
    }

    pub fn height(&self) -> f32 { self.dimensions().z }

    pub fn base_energy(&self) -> u32 {
        match self {
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Dullahan => 4000,
                _ => 3000,
            },
            Body::BirdLarge(body) => match body.species {
                bird_large::Species::Cockatrice => 4000,
                bird_large::Species::Phoenix => 6000,
                bird_large::Species::Roc => 5000,
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
                quadruped_small::Species::Boar => 700,
                quadruped_small::Species::Batfox => 400,
                quadruped_small::Species::Dodarock => 1000,
                quadruped_small::Species::Holladon => 800,
                quadruped_small::Species::Hyena => 450,
                quadruped_small::Species::Truffler => 450,
                _ => 400,
            },
            Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                quadruped_medium::Species::Grolgar => 900,
                quadruped_medium::Species::Saber => 600,
                quadruped_medium::Species::Tiger => 700,
                quadruped_medium::Species::Lion => 900,
                quadruped_medium::Species::Tarasque => 1500,
                quadruped_medium::Species::Wolf => 550,
                quadruped_medium::Species::Frostfang => 400,
                quadruped_medium::Species::Mouflon => 500,
                quadruped_medium::Species::Catoblepas => 1000,
                quadruped_medium::Species::Bonerattler => 500,
                quadruped_medium::Species::Deer => 500,
                quadruped_medium::Species::Hirdrasil => 700,
                quadruped_medium::Species::Roshwalr => 800,
                quadruped_medium::Species::Donkey => 550,
                quadruped_medium::Species::Zebra => 550,
                quadruped_medium::Species::Antelope => 450,
                quadruped_medium::Species::Kelpie => 600,
                quadruped_medium::Species::Horse => 600,
                quadruped_medium::Species::Barghest => 1700,
                quadruped_medium::Species::Cattle => 1000,
                quadruped_medium::Species::Highland => 1200,
                quadruped_medium::Species::Yak => 1100,
                quadruped_medium::Species::Panda => 900,
                quadruped_medium::Species::Bear => 900,
                quadruped_medium::Species::Moose => 800,
                quadruped_medium::Species::Dreadhorn => 1100,
                quadruped_medium::Species::Mammoth => 1700,
                quadruped_medium::Species::Ngoubou => 1500,
                _ => 700,
            },
            Body::BirdMedium(bird_medium) => match bird_medium.species {
                bird_medium::Species::Chicken => 300,
                bird_medium::Species::Duck => 300,
                bird_medium::Species::Goose => 300,
                bird_medium::Species::Parrot => 250,
                bird_medium::Species::Peacock => 350,
                bird_medium::Species::Eagle => 450,
                _ => 250,
            },
            Body::FishMedium(_) => 250,
            Body::Dragon(_) => 5000,
            Body::BirdLarge(bird_large) => match bird_large.species {
                bird_large::Species::Roc => 2800,
                _ => 3000,
            },
            Body::FishSmall(_) => 20,
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Ogre => 3200,
                biped_large::Species::Cyclops => 3200,
                biped_large::Species::Wendigo => 2800,
                biped_large::Species::Cavetroll => 2400,
                biped_large::Species::Mountaintroll => 2400,
                biped_large::Species::Swamptroll => 2400,
                biped_large::Species::Dullahan => 3000,
                biped_large::Species::Mindflayer => 12500,
                biped_large::Species::Tidalwarrior => 16000,
                biped_large::Species::Yeti => 12000,
                biped_large::Species::Minotaur => 30000,
                biped_large::Species::Harvester => 5000,
                biped_large::Species::Blueoni => 2400,
                biped_large::Species::Redoni => 2400,
                _ => 1200,
            },
            Body::BipedSmall(biped_small) => match biped_small.species {
                biped_small::Species::Gnarling => 500,
                biped_small::Species::Adlet => 600,
                biped_small::Species::Sahagin => 800,
                biped_small::Species::Haniwa => 900,
                biped_small::Species::Myrmidon => 900,
                biped_small::Species::Husk => 200,
                _ => 600,
            },
            Body::Object(object) => match object {
                object::Body::TrainingDummy => 10000,
                object::Body::Crossbow => 800,
                object::Body::HaniwaSentry => 600,
                object::Body::SeaLantern => 1000,
                _ => 10000,
            },
            Body::Golem(golem) => match golem.species {
                golem::Species::ClayGolem => 4500,
                _ => 10000,
            },
            Body::Theropod(theropod) => match theropod.species {
                theropod::Species::Archaeos => 3500,
                theropod::Species::Odonto => 3000,
                theropod::Species::Ntouka => 3000,
                _ => 1100,
            },
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Crocodile => 800,
                quadruped_low::Species::Alligator => 900,
                quadruped_low::Species::Monitor => 600,
                quadruped_low::Species::Asp => 750,
                quadruped_low::Species::Tortoise => 900,
                quadruped_low::Species::Rocksnapper => 1400,
                quadruped_low::Species::Pangolin => 400,
                quadruped_low::Species::Maneater => 1300,
                quadruped_low::Species::Sandshark => 900,
                quadruped_low::Species::Hakulaq => 500,
                quadruped_low::Species::Lavadrake => 1600,
                quadruped_low::Species::Basilisk => 2000,
                quadruped_low::Species::Deadwood => 700,
                _ => 700,
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
                quadruped_medium::Species::Mammoth => 70,
                quadruped_medium::Species::Ngoubou => 50,
                _ => 20,
            },
            Body::BirdMedium(bird_medium) => match bird_medium.species {
                bird_medium::Species::Chicken => 10,
                bird_medium::Species::Duck => 10,
                bird_medium::Species::Goose => 10,
                bird_medium::Species::Parrot => 10,
                bird_medium::Species::Peacock => 10,
                bird_medium::Species::Eagle => 10,
                _ => 20,
            },
            Body::FishMedium(_) => 10,
            Body::Dragon(_) => 500,
            Body::BirdLarge(bird_large) => match bird_large.species {
                bird_large::Species::Roc => 110,
                _ => 120,
            },
            Body::FishSmall(_) => 10,
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Ogre => 70,
                biped_large::Species::Cyclops => 80,
                biped_large::Species::Wendigo => 80,
                biped_large::Species::Cavetroll => 60,
                biped_large::Species::Mountaintroll => 60,
                biped_large::Species::Swamptroll => 60,
                biped_large::Species::Dullahan => 120,
                // Boss enemies have their health set, not adjusted by level.
                biped_large::Species::Mindflayer => 0,
                biped_large::Species::Minotaur => 0,
                biped_large::Species::Tidalwarrior => 0,
                biped_large::Species::Yeti => 0,
                biped_large::Species::Harvester => 0,
                _ => 100,
            },
            Body::BipedSmall(_) => 10,
            Body::Object(_) => 10,
            Body::Golem(_) => 0,
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
            Body::BirdLarge(_) => 50.0,
            Body::BirdMedium(_) => 40.0,
            Body::Dragon(_) => 60.0,
            Body::Ship(ship) if ship.can_fly() => 60.0,
            _ => 0.0,
        }
    }

    pub fn immune_to(&self, buff: BuffKind) -> bool {
        match buff {
            BuffKind::Bleeding => matches!(self, Body::Object(_) | Body::Golem(_) | Body::Ship(_)),
            BuffKind::Burning => match self {
                Body::Golem(g) => matches!(g.species, golem::Species::ClayGolem),
                Body::BipedSmall(b) => matches!(b.species, biped_small::Species::Haniwa),
                Body::Object(object::Body::HaniwaSentry) => true,
                _ => false,
            },
            BuffKind::Ensnared => {
                matches!(self, Body::BipedLarge(b) if matches!(b.species, biped_large::Species::Harvester))
            },
            _ => false,
        }
    }

    /// Returns a multiplier representing increased difficulty not accounted for
    /// due to AI or not using an actual weapon
    // TODO: Match on species
    pub fn combat_multiplier(&self) -> f32 {
        match self {
            Body::Object(_) | Body::Ship(_) => 0.0,
            Body::BipedLarge(b) => match b.species {
                biped_large::Species::Mindflayer => 4.8,
                biped_large::Species::Minotaur => 3.2,
                biped_large::Species::Tidalwarrior => 2.25,
                biped_large::Species::Yeti => 2.0,
                biped_large::Species::Harvester => 2.4,
                _ => 1.0,
            },
            Body::Golem(g) => match g.species {
                golem::Species::ClayGolem => 2.0,
                _ => 1.0,
            },
            _ => 1.0,
        }
    }

    pub fn base_poise(&self) -> u32 {
        match self {
            Body::Humanoid(_) => 100,
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Mindflayer => 320,
                biped_large::Species::Minotaur => 280,
                _ => 250,
            },
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
            Body::Ship(ship::Body::AirBalloon) => Vec3::from([0.0, 0.0, 5.0]),
            _ => Vec3::unit_z(),
        }
    }
}

impl Component for Body {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
