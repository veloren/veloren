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
    consts::{HUMANOID_HP_PER_LEVEL, HUMAN_DENSITY, WATER_DENSITY},
    make_case_elim,
    npc::NpcKind,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use strum_macros::Display;
use vek::*;

use super::{BuffKind, Density, Mass};

make_case_elim!(
    body,
    #[derive(Copy, Clone, Debug, Display, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

    pub fn is_campfire(&self) -> bool { matches!(self, Body::Object(object::Body::CampfireLit)) }

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
                biped_large::Species::Cavetroll => 600.0,
                biped_large::Species::Mountaintroll => 600.0,
                biped_large::Species::Swamptroll => 600.0,
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
                // Understand that chaning the mass values can have effects
                // on multiple systems.
                //
                // If you want to change that value, consult with
                // Physics and Combat teams
                //
                // Weight is proportional height, where
                // a 1.75m character would weigh 65kg
                65.0 * humanoid.height() / 1.75f32
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
    /// respectively (in metres)
    // Code reviewers: should we replace metres with 'block height'?
    pub fn dimensions(&self) -> Vec3<f32> {
        match self {
            Body::BipedLarge(body) => match body.species {
                biped_large::Species::Cyclops => Vec3::new(5.6, 3.0, 6.5),
                biped_large::Species::Dullahan => Vec3::new(4.6, 3.0, 5.5),
                biped_large::Species::Mightysaurok => Vec3::new(4.0, 3.0, 3.4),
                biped_large::Species::Mindflayer => Vec3::new(4.4, 3.0, 8.0),
                biped_large::Species::Minotaur => Vec3::new(6.0, 3.0, 8.0),
                biped_large::Species::Occultsaurok => Vec3::new(4.0, 3.0, 3.4),
                biped_large::Species::Slysaurok => Vec3::new(4.0, 3.0, 3.4),
                biped_large::Species::Werewolf => Vec3::new(4.0, 3.0, 3.5),
                biped_large::Species::Harvester => Vec3::new(4.6, 3.0, 5.4),
                biped_large::Species::Cultistwarlord => Vec3::new(3.0, 3.0, 4.5),
                biped_large::Species::Cultistwarlock => Vec3::new(3.0, 3.0, 3.5),
                biped_large::Species::Huskbrute => Vec3::new(4.6, 3.0, 5.0),
                _ => Vec3::new(4.6, 3.0, 6.0),
            },
            Body::BipedSmall(body) => match body.species {
                biped_small::Species::Gnarling => Vec3::new(1.0, 0.75, 1.4),
                biped_small::Species::Haniwa => Vec3::new(1.0, 0.75, 2.2),
                biped_small::Species::Adlet => Vec3::new(1.0, 0.75, 2.0),
                biped_small::Species::Sahagin => Vec3::new(1.0, 1.2, 1.7),
                biped_small::Species::Myrmidon => Vec3::new(1.0, 0.75, 2.2),
                biped_small::Species::Husk => Vec3::new(1.7, 0.7, 2.7),

                _ => Vec3::new(1.0, 0.75, 1.4),
            },
            Body::BirdMedium(body) => match body.species {
                bird_medium::Species::Chicken => Vec3::new(1.0, 1.0, 1.35),
                bird_medium::Species::Duck => Vec3::new(0.9, 1.0, 1.4),
                bird_medium::Species::Goose => Vec3::new(1.0, 1.2, 1.5),
                bird_medium::Species::Peacock => Vec3::new(1.3, 1.1, 1.4),
                _ => Vec3::new(2.0, 1.0, 1.5),
            },
            Body::BirdLarge(_) => Vec3::new(2.0, 6.0, 3.5),
            Body::Dragon(_) => Vec3::new(16.0, 10.0, 16.0),
            Body::FishMedium(_) => Vec3::new(0.5, 2.0, 0.8),
            Body::FishSmall(_) => Vec3::new(0.3, 1.2, 0.6),
            Body::Golem(_) => Vec3::new(5.0, 5.0, 7.5),
            Body::Humanoid(humanoid) => {
                let height = humanoid.height();
                Vec3::new(height / 1.3, 1.75 / 2.0, height)
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
                quadruped_medium::Species::Mammoth => Vec3::new(2.0, 7.0, 8.0),
                quadruped_medium::Species::Ngoubou => Vec3::new(2.0, 3.2, 2.4),
                quadruped_medium::Species::Llama => Vec3::new(2.0, 2.5, 2.6),
                quadruped_medium::Species::Alpaca => Vec3::new(2.0, 2.0, 2.0),
                quadruped_medium::Species::Camel => Vec3::new(2.0, 4.0, 3.5),
                // FIXME: We really shouldn't be doing wildcards here
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
                quadruped_low::Species::Sandshark => Vec3::new(1.1, 4.3, 1.7),
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
    pub fn max_radius(&self) -> f32 {
        let dim = self.dimensions();
        let (x, y) = (dim.x, dim.y);

        x.max(y) / 2.0
    }

    pub fn min_radius(&self) -> f32 {
        let (_p0, _p1, radius) = self.sausage();

        radius
    }

    /// Base of our Capsule Prism used for collisions.
    /// Returns line segment and radius. See [this wiki page][stadium_wiki].
    ///
    /// [stadium_wiki]: <https://en.wikipedia.org/wiki/Stadium_(geometry)>
    pub fn sausage(&self) -> (Vec2<f32>, Vec2<f32>, f32) {
        // Consider this ascii-art stadium with radius `r` and line segment `a`
        //
        //      xxxxxxxxxxxxxxxxx
        //
        //        _ ----------_
        // y    -*      r      *-
        // y   *        r        *
        // y  * rrr aaaaaaaaa rrr *
        // y  *         r         *
        // y   *        r        *
        //       *____________ ^
        let dim = self.dimensions();
        // The width (shoulder to shoulder) and length (nose to tail)
        let (width, length) = (dim.x, dim.y);

        if length > width {
            // Dachshund-like
            let radius = width / 2.0;

            let a = length - 2.0 * radius;

            let p0 = Vec2::new(0.0, -a / 2.0);
            let p1 = Vec2::new(0.0, a / 2.0);

            (p0, p1, radius)
        } else {
            // Cyclops-like
            let radius = length / 2.0;

            let a = width - 2.0 * radius;

            let p0 = Vec2::new(-a / 2.0, 0.0);
            let p1 = Vec2::new(a / 2.0, 0.0);

            (p0, p1, radius)
        }
    }

    // How far away other entities should try to be. Will be added uppon the other
    // entitys spacing_radius. So an entity with 2.0 and an entity with 3.0 will
    // lead to that both entities will try to keep 5.0 units away from each
    // other.
    pub fn spacing_radius(&self) -> f32 {
        self.max_radius()
            + match self {
                Body::QuadrupedSmall(body) => match body.species {
                    quadruped_small::Species::Rat => 0.0,
                    _ => 2.0,
                },
                Body::QuadrupedLow(body) => match body.species {
                    quadruped_low::Species::Hakulaq => 0.0,
                    _ => 2.0,
                },
                Body::BipedSmall(body) => match body.species {
                    biped_small::Species::Husk => 3.0,
                    _ => 2.0,
                },
                _ => 2.0,
            }
    }

    /// Height from the bottom to the top (in metres)
    pub fn height(&self) -> f32 { self.dimensions().z }

    pub fn base_energy(&self) -> u16 {
        match self {
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Dullahan => 400,
                _ => 300,
            },
            Body::BirdLarge(body) => match body.species {
                bird_large::Species::Cockatrice => 400,
                bird_large::Species::Phoenix => 600,
                bird_large::Species::Roc => 500,
                bird_large::Species::FlameWyvern => 600,
            },
            Body::Humanoid(_) => 75,
            _ => 100,
        }
    }

    #[allow(unreachable_patterns)]
    pub fn base_health(&self) -> u16 {
        match self {
            Body::Humanoid(_) => 50,
            Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
                quadruped_small::Species::Boar => 70,
                quadruped_small::Species::Batfox => 40,
                quadruped_small::Species::Dodarock => 100,
                quadruped_small::Species::Holladon => 80,
                quadruped_small::Species::Hyena => 45,
                quadruped_small::Species::Truffler => 45,
                _ => 40,
            },
            Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                quadruped_medium::Species::Grolgar => 90,
                quadruped_medium::Species::Saber => 60,
                quadruped_medium::Species::Tiger => 70,
                quadruped_medium::Species::Lion => 90,
                quadruped_medium::Species::Tarasque => 150,
                quadruped_medium::Species::Wolf => 55,
                quadruped_medium::Species::Frostfang => 40,
                quadruped_medium::Species::Mouflon => 50,
                quadruped_medium::Species::Catoblepas => 100,
                quadruped_medium::Species::Bonerattler => 50,
                quadruped_medium::Species::Deer => 50,
                quadruped_medium::Species::Hirdrasil => 70,
                quadruped_medium::Species::Roshwalr => 80,
                quadruped_medium::Species::Donkey => 55,
                quadruped_medium::Species::Zebra => 55,
                quadruped_medium::Species::Antelope => 45,
                quadruped_medium::Species::Kelpie => 60,
                quadruped_medium::Species::Horse => 60,
                quadruped_medium::Species::Barghest => 170,
                quadruped_medium::Species::Cattle => 100,
                quadruped_medium::Species::Highland => 120,
                quadruped_medium::Species::Yak => 110,
                quadruped_medium::Species::Panda => 90,
                quadruped_medium::Species::Bear => 90,
                quadruped_medium::Species::Moose => 80,
                quadruped_medium::Species::Dreadhorn => 370,
                quadruped_medium::Species::Mammoth => 250,
                quadruped_medium::Species::Ngoubou => 290,
                _ => 70,
            },
            Body::BirdMedium(bird_medium) => match bird_medium.species {
                bird_medium::Species::Chicken => 30,
                bird_medium::Species::Duck => 30,
                bird_medium::Species::Goose => 30,
                bird_medium::Species::Parrot => 25,
                bird_medium::Species::Peacock => 35,
                bird_medium::Species::Eagle => 45,
                _ => 25,
            },
            Body::FishMedium(_) => 25,
            Body::Dragon(_) => 500,
            Body::BirdLarge(bird_large) => match bird_large.species {
                bird_large::Species::Roc => 280,
                _ => 300,
            },
            Body::FishSmall(_) => 2,
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Ogre => 320,
                biped_large::Species::Cyclops => 320,
                biped_large::Species::Wendigo => 280,
                biped_large::Species::Cavetroll => 240,
                biped_large::Species::Mountaintroll => 240,
                biped_large::Species::Swamptroll => 240,
                biped_large::Species::Dullahan => 700,
                biped_large::Species::Mindflayer => 1250,
                biped_large::Species::Tidalwarrior => 1600,
                biped_large::Species::Yeti => 1200,
                biped_large::Species::Minotaur => 3000,
                biped_large::Species::Harvester => 500,
                biped_large::Species::Blueoni => 240,
                biped_large::Species::Redoni => 240,
                biped_large::Species::Huskbrute => 800,
                _ => 120,
            },
            Body::BipedSmall(biped_small) => match biped_small.species {
                biped_small::Species::Gnarling => 50,
                biped_small::Species::Adlet => 60,
                biped_small::Species::Sahagin => 80,
                biped_small::Species::Haniwa => 90,
                biped_small::Species::Myrmidon => 90,
                biped_small::Species::Husk => 20,
                _ => 60,
            },
            Body::Object(object) => match object {
                object::Body::TrainingDummy => 1000,
                object::Body::Crossbow => 80,
                object::Body::HaniwaSentry => 60,
                object::Body::SeaLantern => 100,
                _ => 1000,
            },
            Body::Golem(golem) => match golem.species {
                golem::Species::ClayGolem => 450,
                _ => 1000,
            },
            Body::Theropod(theropod) => match theropod.species {
                theropod::Species::Archaeos => 350,
                theropod::Species::Yale => 280,
                theropod::Species::Odonto => 300,
                theropod::Species::Ntouka => 300,
                _ => 110,
            },
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Crocodile => 80,
                quadruped_low::Species::Alligator => 90,
                quadruped_low::Species::Monitor => 60,
                quadruped_low::Species::Asp => 75,
                quadruped_low::Species::Tortoise => 90,
                quadruped_low::Species::Rocksnapper => 140,
                quadruped_low::Species::Pangolin => 40,
                quadruped_low::Species::Maneater => 130,
                quadruped_low::Species::Sandshark => 110,
                quadruped_low::Species::Hakulaq => 120,
                quadruped_low::Species::Lavadrake => 160,
                quadruped_low::Species::Basilisk => 200,
                quadruped_low::Species::Deadwood => 120,
                _ => 70,
            },
            Body::Ship(_) => 1000,
        }
    }

    #[allow(unreachable_patterns)]
    pub fn base_health_increase(&self) -> u16 {
        match self {
            Body::Humanoid(_) => HUMANOID_HP_PER_LEVEL,
            Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
                quadruped_small::Species::Boar => 2,
                quadruped_small::Species::Batfox => 1,
                quadruped_small::Species::Dodarock => 3,
                quadruped_small::Species::Holladon => 3,
                quadruped_small::Species::Hyena => 2,
                quadruped_small::Species::Truffler => 2,
                _ => 1,
            },
            Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                quadruped_medium::Species::Grolgar => 3,
                quadruped_medium::Species::Saber => 2,
                quadruped_medium::Species::Tiger => 2,
                quadruped_medium::Species::Tuskram => 3,
                quadruped_medium::Species::Lion => 4,
                quadruped_medium::Species::Tarasque => 6,
                quadruped_medium::Species::Wolf => 2,
                quadruped_medium::Species::Frostfang => 4,
                quadruped_medium::Species::Mouflon => 3,
                quadruped_medium::Species::Catoblepas => 5,
                quadruped_medium::Species::Bonerattler => 3,
                quadruped_medium::Species::Deer => 2,
                quadruped_medium::Species::Hirdrasil => 3,
                quadruped_medium::Species::Roshwalr => 4,
                quadruped_medium::Species::Donkey => 3,
                quadruped_medium::Species::Camel => 3,
                quadruped_medium::Species::Zebra => 3,
                quadruped_medium::Species::Antelope => 2,
                quadruped_medium::Species::Kelpie => 3,
                quadruped_medium::Species::Horse => 3,
                quadruped_medium::Species::Barghest => 5,
                quadruped_medium::Species::Cattle => 3,
                quadruped_medium::Species::Highland => 3,
                quadruped_medium::Species::Yak => 5,
                quadruped_medium::Species::Panda => 4,
                quadruped_medium::Species::Bear => 4,
                quadruped_medium::Species::Moose => 3,
                quadruped_medium::Species::Dreadhorn => 6,
                quadruped_medium::Species::Mammoth => 8,
                quadruped_medium::Species::Ngoubou => 7,
                _ => 2,
            },
            Body::BirdMedium(bird_medium) => match bird_medium.species {
                bird_medium::Species::Chicken => 1,
                bird_medium::Species::Duck => 1,
                bird_medium::Species::Goose => 1,
                bird_medium::Species::Parrot => 1,
                bird_medium::Species::Peacock => 1,
                bird_medium::Species::Eagle => 1,
                _ => 2,
            },
            Body::FishMedium(_) => 1,
            Body::Dragon(_) => 50,
            Body::BirdLarge(bird_large) => match bird_large.species {
                bird_large::Species::Roc => 11,
                _ => 12,
            },
            Body::FishSmall(_) => 1,
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Ogre => 7,
                biped_large::Species::Cyclops => 8,
                biped_large::Species::Wendigo => 8,
                biped_large::Species::Cavetroll => 6,
                biped_large::Species::Mountaintroll => 6,
                biped_large::Species::Swamptroll => 6,
                biped_large::Species::Dullahan => 12,
                biped_large::Species::Huskbrute => 10,
                // Boss enemies have their health set, not adjusted by level.
                biped_large::Species::Huskbrute => 0,
                biped_large::Species::Mindflayer => 0,
                biped_large::Species::Minotaur => 0,
                biped_large::Species::Tidalwarrior => 0,
                biped_large::Species::Yeti => 0,
                biped_large::Species::Harvester => 0,
                _ => 10,
            },
            Body::BipedSmall(_) => 1,
            Body::Object(_) => 1,
            Body::Golem(_) => 0,
            Body::Theropod(_) => 2,
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Crocodile => 2,
                quadruped_low::Species::Alligator => 2,
                quadruped_low::Species::Salamander => 1,
                quadruped_low::Species::Monitor => 1,
                quadruped_low::Species::Asp => 1,
                quadruped_low::Species::Tortoise => 2,
                quadruped_low::Species::Rocksnapper => 5,
                quadruped_low::Species::Pangolin => 1,
                quadruped_low::Species::Maneater => 3,
                quadruped_low::Species::Sandshark => 5,
                quadruped_low::Species::Hakulaq => 1,
                quadruped_low::Species::Deadwood => 3,
                _ => 2,
            },
            Body::Ship(_) => 50,
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
                Body::QuadrupedLow(q) => matches!(q.species, quadruped_low::Species::Lavadrake),
                Body::BirdLarge(b) => matches!(
                    b.species,
                    bird_large::Species::Phoenix
                        | bird_large::Species::Cockatrice
                        | bird_large::Species::FlameWyvern
                ),
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
                biped_large::Species::Mindflayer => 4.35,
                biped_large::Species::Minotaur => 4.05,
                biped_large::Species::Tidalwarrior => 2.75,
                biped_large::Species::Yeti => 2.25,
                biped_large::Species::Harvester => 2.1,
                _ => 1.0,
            },
            Body::Golem(g) => match g.species {
                golem::Species::ClayGolem => 2.45,
                _ => 1.0,
            },
            _ => 1.0,
        }
    }

    pub fn base_poise(&self) -> u16 {
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

    /// Component of the mounting offset specific to the mountee
    pub fn mountee_offset(&self) -> Vec3<f32> {
        match self {
            Body::QuadrupedMedium(quadruped_medium) => {
                match (quadruped_medium.species, quadruped_medium.body_type) {
                    (quadruped_medium::Species::Grolgar, _) => [0.5, 0.5, 1.8],
                    (quadruped_medium::Species::Saber, _) => [0.3, 0.3, 1.3],
                    (quadruped_medium::Species::Tiger, _) => [0.2, 0.2, 1.4],
                    (quadruped_medium::Species::Tuskram, _) => [-0.5, -0.5, 1.5],
                    (quadruped_medium::Species::Lion, _) => [0.3, 0.3, 1.5],
                    (quadruped_medium::Species::Tarasque, _) => [0.6, 0.6, 2.0],
                    (quadruped_medium::Species::Wolf, _) => [0.5, 0.5, 1.3],
                    (quadruped_medium::Species::Frostfang, _) => [0.5, 0.5, 1.2],
                    (quadruped_medium::Species::Mouflon, _) => [0.3, 0.3, 1.2],
                    (quadruped_medium::Species::Catoblepas, _) => [0.0, 0.0, 2.0],
                    (quadruped_medium::Species::Bonerattler, _) => [0.5, 0.5, 1.2],
                    (quadruped_medium::Species::Deer, _) => [0.2, 0.2, 1.3],
                    (quadruped_medium::Species::Hirdrasil, _) => [0.0, 0.0, 1.4],
                    (quadruped_medium::Species::Roshwalr, _) => [0.5, 0.5, 1.8],
                    (quadruped_medium::Species::Donkey, _) => [0.5, 0.5, 1.5],
                    (quadruped_medium::Species::Camel, _) => [-0.1, -0.1, 2.8],
                    (quadruped_medium::Species::Zebra, _) => [0.5, 0.5, 1.8],
                    (quadruped_medium::Species::Antelope, _) => [0.3, 0.3, 1.4],
                    (quadruped_medium::Species::Kelpie, _) => [0.5, 0.5, 1.9],
                    (quadruped_medium::Species::Horse, _) => [0.0, 0.0, 2.0],
                    (quadruped_medium::Species::Barghest, _) => [0.5, 0.5, 2.2],
                    (quadruped_medium::Species::Cattle, quadruped_medium::BodyType::Male) => {
                        [0.5, 0.5, 2.6]
                    },
                    (quadruped_medium::Species::Cattle, quadruped_medium::BodyType::Female) => {
                        [0.7, 0.7, 2.2]
                    },
                    (quadruped_medium::Species::Darkhound, _) => [0.5, 0.5, 1.4],
                    (quadruped_medium::Species::Highland, _) => [0.5, 0.5, 2.3],
                    (quadruped_medium::Species::Yak, _) => [0.0, 0.0, 3.0],
                    (quadruped_medium::Species::Panda, _) => [-0.2, -0.2, 1.4],
                    (quadruped_medium::Species::Bear, _) => [-0.4, -0.4, 2.5],
                    (quadruped_medium::Species::Dreadhorn, _) => [0.2, 0.2, 3.5],
                    (quadruped_medium::Species::Moose, _) => [-0.6, -0.6, 2.1],
                    (quadruped_medium::Species::Snowleopard, _) => [-0.5, -0.5, 1.4],
                    (quadruped_medium::Species::Mammoth, _) => [0.0, 4.9, 7.2],
                    (quadruped_medium::Species::Ngoubou, _) => [0.0, 0.3, 2.0],
                    (quadruped_medium::Species::Llama, _) => [0.0, 0.1, 1.5],
                    (quadruped_medium::Species::Alpaca, _) => [0.0, -0.1, 1.0],
                }
            },
            Body::Ship(ship) => match ship {
                ship::Body::DefaultAirship => [0.0, 0.0, 10.0],
                ship::Body::AirBalloon => [0.0, 0.0, 5.0],
                ship::Body::SailBoat => [-2.0, -5.0, 4.0],
                ship::Body::Galleon => [-2.0, -5.0, 4.0],
                ship::Body::Volume => [0.0, 0.0, 0.0],
            },
            _ => [0.0, 0.0, 0.0],
        }
        .into()
    }

    /// Component of the mounting offset specific to the mounter
    pub fn mounter_offset(&self) -> Vec3<f32> {
        match self {
            Body::Humanoid(_) => [0.0, 0.0, 0.0],
            _ => [0.0, 0.0, 0.0],
        }
        .into()
    }
}

impl Component for Body {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
