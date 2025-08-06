pub mod arthropod;
pub mod biped_large;
pub mod biped_small;
pub mod bird_large;
pub mod bird_medium;
pub mod crustacean;
pub mod dragon;
pub mod fish_medium;
pub mod fish_small;
pub mod golem;
pub mod humanoid;
pub mod item;
pub mod object;
pub mod parts;
pub mod plugin;
pub mod quadruped_low;
pub mod quadruped_medium;
pub mod quadruped_small;
pub mod ship;
pub mod theropod;

use crate::{
    assets::{self, Asset},
    consts::{HUMAN_DENSITY, WATER_DENSITY},
    npc::NpcKind,
};
use common_base::enum_iter;
use common_i18n::Content;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use strum::{Display, IntoEnumIterator};
use vek::*;

use super::{BuffKind, Collider, Density, Mass, Scale};

enum_iter! {
    #[derive(
        Copy, Clone, Debug, Display, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize,
    )]
    #[repr(u32)]
    pub enum Body {
        Humanoid(humanoid::Body) = 0,
        QuadrupedSmall(quadruped_small::Body) = 1,
        QuadrupedMedium(quadruped_medium::Body) = 2,
        BirdMedium(bird_medium::Body) = 3,
        FishMedium(fish_medium::Body) = 4,
        Dragon(dragon::Body) = 5,
        BirdLarge(bird_large::Body) = 6,
        FishSmall(fish_small::Body) = 7,
        BipedLarge(biped_large::Body) = 8,
        BipedSmall(biped_small::Body) = 9,
        Object(object::Body) = 10,
        Golem(golem::Body) = 11,
        Theropod(theropod::Body) = 12,
        QuadrupedLow(quadruped_low::Body) = 13,
        Ship(ship::Body) = 14,
        Arthropod(arthropod::Body) = 15,
        Item(item::Body) = 16,
        Crustacean(crustacean::Body) = 17,
        Plugin(plugin::Body) = 18,
    }
}

// Implemented for Buff, to be able to implement EnumIter.
impl Default for Body {
    fn default() -> Self {
        Body::QuadrupedSmall(quadruped_small::Body {
            species: quadruped_small::Species::Frog,
            body_type: quadruped_small::BodyType::Female,
        })
    }
}

/// Data representing data generic to the body together with per-species data.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BodyData<BodyMeta, SpeciesData> {
    /// Shared metadata for this whole body type.
    pub body: BodyMeta,
    /// All the metadata for species with this body type.
    pub species: SpeciesData,
}

/// Metadata intended to be stored per-body, together with data intended to be
/// stored for each species for each body.
///
/// NOTE: If you are adding new body kind and it should be spawned via /spawn
/// please add it to `[ENTITIES](crate::cmd::ENTITIES)`
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    pub item: BodyData<BodyMeta, ()>,
    pub golem: BodyData<BodyMeta, golem::AllSpecies<SpeciesMeta>>,
    pub theropod: BodyData<BodyMeta, theropod::AllSpecies<SpeciesMeta>>,
    pub quadruped_low: BodyData<BodyMeta, quadruped_low::AllSpecies<SpeciesMeta>>,
    pub ship: BodyData<BodyMeta, ()>,
    pub arthropod: BodyData<BodyMeta, arthropod::AllSpecies<SpeciesMeta>>,
    pub crustacean: BodyData<BodyMeta, crustacean::AllSpecies<SpeciesMeta>>,
    pub plugin: BodyData<BodyMeta, plugin::AllSpecies<SpeciesMeta>>,
}

impl<BodyMeta, SpeciesMeta> AllBodies<BodyMeta, SpeciesMeta> {
    /// Get species meta associated with the body.
    ///
    /// Returns `None` if the body doesn't have any associated meta, i.e ships,
    /// objects, items.
    pub fn get_species_meta<'a>(&'a self, body: &Body) -> Option<&'a SpeciesMeta> {
        Some(match body {
            Body::Humanoid(b) => &self.humanoid.species[&b.species],
            Body::QuadrupedSmall(b) => &self.quadruped_small.species[&b.species],
            Body::QuadrupedMedium(b) => &self.quadruped_medium.species[&b.species],
            Body::BirdMedium(b) => &self.bird_medium.species[&b.species],
            Body::BirdLarge(b) => &self.bird_large.species[&b.species],
            Body::FishMedium(b) => &self.fish_medium.species[&b.species],
            Body::Dragon(b) => &self.dragon.species[&b.species],
            Body::FishSmall(b) => &self.fish_small.species[&b.species],
            Body::BipedLarge(b) => &self.biped_large.species[&b.species],
            Body::BipedSmall(b) => &self.biped_small.species[&b.species],
            Body::Golem(b) => &self.golem.species[&b.species],
            Body::Theropod(b) => &self.theropod.species[&b.species],
            Body::QuadrupedLow(b) => &self.quadruped_low.species[&b.species],
            Body::Arthropod(b) => &self.arthropod.species[&b.species],
            Body::Crustacean(b) => &self.crustacean.species[&b.species],
            Body::Plugin(b) => &self.plugin.species[&b.species],
            Body::Item(_) | Body::Ship(_) | Body::Object(_) => return None,
        })
    }
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
            NpcKind::Tarantula => &self.arthropod.body,
            NpcKind::Crab => &self.crustacean.body,
            NpcKind::Plugin => &self.plugin.body,
        }
    }
}

/// Can only retrieve body metadata by direct index.
impl<BodyMeta, SpeciesMeta> core::ops::Index<&Body> for AllBodies<BodyMeta, SpeciesMeta> {
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
            Body::Item(_) => &self.item.body,
            Body::Golem(_) => &self.golem.body,
            Body::Theropod(_) => &self.theropod.body,
            Body::QuadrupedLow(_) => &self.quadruped_low.body,
            Body::Arthropod(_) => &self.arthropod.body,
            Body::Ship(_) => &self.ship.body,
            Body::Crustacean(_) => &self.crustacean.body,
            Body::Plugin(_) => &self.plugin.body,
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

/// Semantic gender aka body_type
///
/// Should be used for localization with extreme care.
/// For basically everything except *maybe* humanoids, it's simply wrong to
/// assume that this may be used as grammatical gender.
/// Read more on `Body::default_gender`.
///
/// TODO: move this to common::i18n (or create new alternative), extend with
/// more options and add GUI for players to choose preferred gender.
/// Read a comment for `gender_str` in voxygen/i18n-helpers/src/lib.rs.
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub enum Gender {
    Masculine,
    Feminine,
    Neuter,
}

impl Body {
    pub fn is_same_species_as(&self, other: &Body) -> bool {
        match self {
            Body::Humanoid(b1) => match other {
                Body::Humanoid(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::QuadrupedSmall(b1) => match other {
                Body::QuadrupedSmall(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::QuadrupedMedium(b1) => match other {
                Body::QuadrupedMedium(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::BirdMedium(b1) => match other {
                Body::BirdMedium(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::BirdLarge(b1) => match other {
                Body::BirdLarge(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::FishMedium(b1) => match other {
                Body::FishMedium(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::Dragon(b1) => match other {
                Body::Dragon(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::FishSmall(b1) => match other {
                Body::FishSmall(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::BipedLarge(b1) => match other {
                Body::BipedLarge(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::BipedSmall(b1) => match other {
                Body::BipedSmall(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::Object(_) => false,
            Body::Item(_) => false,
            Body::Golem(b1) => match other {
                Body::Golem(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::Theropod(b1) => match other {
                Body::Theropod(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::QuadrupedLow(b1) => match other {
                Body::QuadrupedLow(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::Arthropod(b1) => match other {
                Body::Arthropod(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::Ship(_) => false,
            Body::Crustacean(b1) => match other {
                Body::Crustacean(b2) => b1.species == b2.species,
                _ => false,
            },
            Body::Plugin(b1) => match other {
                Body::Plugin(b2) => b1.species == b2.species,
                _ => false,
            },
        }
    }

    /// How many heads this body has in the `Heads` component if any.
    pub fn heads(&self) -> Option<usize> {
        match self {
            Body::QuadrupedLow(body) => match body.species {
                quadruped_low::Species::Hydra => Some(3),
                _ => None,
            },
            _ => None,
        }
    }

    /// This is used to see if a body can be controlled by its rider.
    pub fn has_free_will(&self) -> bool {
        match self {
            Body::Humanoid(_) | Body::BipedLarge(_) | Body::BipedSmall(_) => true,
            Body::Crustacean(body) => match body.species {
                crustacean::Species::Crab => false,
                crustacean::Species::SoldierCrab => false,
                crustacean::Species::Karkatha => true,
            },
            Body::QuadrupedSmall(_)
            | Body::QuadrupedMedium(_)
            | Body::BirdMedium(_)
            | Body::FishMedium(_)
            | Body::Dragon(_)
            | Body::BirdLarge(_)
            | Body::FishSmall(_)
            | Body::Object(_)
            | Body::Golem(_)
            | Body::Theropod(_)
            | Body::QuadrupedLow(_)
            | Body::Ship(_)
            | Body::Arthropod(_)
            | Body::Item(_)
            | Body::Plugin(_) => false,
        }
    }

    pub fn is_humanoid(&self) -> bool { matches!(self, Body::Humanoid(_)) }

    pub fn is_campfire(&self) -> bool { matches!(self, Body::Object(object::Body::CampfireLit)) }

    pub fn is_portal(&self) -> bool {
        matches!(
            self,
            Body::Object(object::Body::Portal | object::Body::PortalActive)
        )
    }

    pub fn bleeds(&self) -> bool {
        !matches!(
            self,
            Body::Object(_) | Body::Ship(_) | Body::Item(_) | Body::Golem(_)
        )
    }

    /// The length of the stride of the body, in metres (not accounting for
    /// different legs)
    pub fn stride_length(&self) -> f32 {
        if let Body::Humanoid(body) = self {
            body.scaler() * 3.75
        } else {
            // Rough heuristic
            let dims = self.dimensions();
            0.65 + (dims.y + dims.z) * 0.6
        }
    }

    pub fn scale(&self) -> Scale {
        let s = match self {
            Body::BirdMedium(bird_medium) => match bird_medium.species {
                bird_medium::Species::Bat | bird_medium::Species::VampireBat => 0.5,
                _ => 1.0,
            },
            _ => 1.0,
        };
        Scale(s)
    }

    /// Average density of the body
    // Units are based on kg/m³
    pub fn density(&self) -> Density {
        let d = match self {
            // based on a house sparrow (Passer domesticus)
            Body::BirdMedium(_) => 700.0,
            Body::BirdLarge(_) => 2_200.0,

            Body::Dragon(_) => 5_000.0,

            Body::Golem(_) => WATER_DENSITY * 2.5,
            Body::Humanoid(_) => HUMAN_DENSITY,
            Body::Ship(ship) => ship.density().0,
            Body::Object(object) => object.density().0,
            Body::Item(item) => item.density().0,
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
                biped_large::Species::Gigasfrost | biped_large::Species::Gigasfire => 400.0,
                biped_large::Species::AdletElder => 350.0,
                biped_large::Species::HaniwaGeneral => 360.0,
                biped_large::Species::TerracottaBesieger
                | biped_large::Species::TerracottaDemolisher
                | biped_large::Species::TerracottaPunisher
                | biped_large::Species::TerracottaPursuer
                | biped_large::Species::Cursekeeper => 380.0,
                biped_large::Species::Forgemaster => 1600.0,
                _ => 400.0,
            },
            Body::BipedSmall(body) => match body.species {
                biped_small::Species::IronDwarf => 1000.0,
                biped_small::Species::Flamekeeper => 1000.0,
                biped_small::Species::Boreal => 270.0,
                _ => 50.0,
            },
            // ravens are 0.69-2 kg, crows are 0.51 kg on average.
            Body::BirdMedium(body) => match body.species {
                bird_medium::Species::SnowyOwl => 3.0,
                bird_medium::Species::HornedOwl => 3.0,
                bird_medium::Species::Duck => 3.5,
                bird_medium::Species::Cockatiel => 2.0,
                bird_medium::Species::Chicken => 2.5, // ~✅ Red junglefowl are 1-1.5 kg
                bird_medium::Species::Bat => 1.5,
                bird_medium::Species::Penguin => 10.0,
                bird_medium::Species::Eagle => 7.0, // ~✅ Steller's sea eagle are 5-9 kg
                bird_medium::Species::Goose => 3.5, // ~✅ Swan geese are 2.8-3.5 kg
                bird_medium::Species::Parrot => 1.0,
                bird_medium::Species::Peacock => 6.0,
                bird_medium::Species::Crow => 3.0,
                bird_medium::Species::Dodo => 4.0,
                bird_medium::Species::Parakeet => 1.0,
                bird_medium::Species::Puffin => 2.0,
                bird_medium::Species::Toucan => 4.5,
                bird_medium::Species::BloodmoonBat => 1.5,
                bird_medium::Species::VampireBat => 1.5,
            },
            Body::BirdLarge(_) => 250.0,
            Body::Dragon(_) => 20_000.0,
            Body::FishMedium(_) => 5.0,
            Body::FishSmall(_) => 1.0,
            Body::Golem(_) => 10_000.0,
            Body::Humanoid(humanoid) => {
                // Understand that changing the mass values can have effects
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
            Body::Item(item) => item.mass().0,
            Body::QuadrupedLow(body) => match body.species {
                quadruped_low::Species::Alligator => 360.0, // ~✅
                quadruped_low::Species::Snaretongue => 280.0,
                quadruped_low::Species::Asp => 300.0,
                // saltwater crocodiles can weigh around 1 ton, but our version is the size of an
                // alligator or smaller, so whatever
                quadruped_low::Species::Crocodile => 360.0,
                quadruped_low::Species::SeaCrocodile => 410.0,
                quadruped_low::Species::Deadwood => 200.0,
                quadruped_low::Species::Monitor => 200.0,
                quadruped_low::Species::Pangolin => 300.0,
                quadruped_low::Species::Salamander => 350.0,
                quadruped_low::Species::Elbst => 350.0,
                quadruped_low::Species::Tortoise => 300.0,
                quadruped_low::Species::Lavadrake => 700.0,
                quadruped_low::Species::Icedrake => 700.0,
                quadruped_low::Species::Mossdrake => 700.0,
                quadruped_low::Species::Rocksnapper => 450.0,
                quadruped_low::Species::Rootsnapper => 450.0,
                quadruped_low::Species::Reefsnapper => 450.0,
                quadruped_low::Species::Maneater => 80.0,
                quadruped_low::Species::Sandshark => 450.0,
                quadruped_low::Species::Hakulaq => 400.0,
                quadruped_low::Species::Dagon => 600.0,
                quadruped_low::Species::Basilisk => 800.0,
                quadruped_low::Species::Driggle => 55.0,
                quadruped_low::Species::Hydra => 800.0,
            },
            Body::QuadrupedMedium(body) => match body.species {
                quadruped_medium::Species::Bear => 500.0, // ~✅ (350-700 kg)
                quadruped_medium::Species::Cattle => 575.0, // ~✅ (500-650 kg)
                quadruped_medium::Species::Deer => 80.0,
                quadruped_medium::Species::Donkey => 200.0,
                quadruped_medium::Species::Highland => 200.0,
                quadruped_medium::Species::Horse => 300.0, // ~✅
                quadruped_medium::Species::Kelpie => 250.0,
                quadruped_medium::Species::Lion => 170.0, // ~✅ (110-225 kg)
                quadruped_medium::Species::Panda => 200.0,
                quadruped_medium::Species::Saber => 130.0,
                quadruped_medium::Species::Yak => 200.0,
                quadruped_medium::Species::Dreadhorn => 500.0,
                quadruped_medium::Species::Mammoth => 1500.0,
                quadruped_medium::Species::Catoblepas => 300.0,
                _ => 200.0,
            },
            Body::QuadrupedSmall(body) => match body.species {
                quadruped_small::Species::Axolotl => 1.0,
                quadruped_small::Species::Batfox => 10.0,
                quadruped_small::Species::Beaver => 10.0,
                quadruped_small::Species::Boar => 80.0, // ~✅ (60-100 kg)
                quadruped_small::Species::Cat => 4.0,   // ~✅ (4-5 kg)
                quadruped_small::Species::Dog => 30.0,  // ~✅ (German Shepherd: 30-40 kg)
                quadruped_small::Species::Fox => 10.0,
                quadruped_small::Species::Frog => 1.0,
                quadruped_small::Species::Fungome => 10.0,
                quadruped_small::Species::Gecko => 1.0,
                quadruped_small::Species::Goat => 50.0,
                quadruped_small::Species::Hare => 10.0,
                quadruped_small::Species::Holladon => 70.0,
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
                quadruped_small::Species::Seal => 15.0,
                quadruped_small::Species::TreantSapling => 80.0,
                quadruped_small::Species::MossySnail => 5.0,
            },
            Body::Theropod(body) => match body.species {
                // for reference, elephants are in the range of 2.6-6.9 tons
                // and Tyrannosaurus rex were ~8.4-14 tons
                theropod::Species::Archaeos => 8_000.0,
                theropod::Species::Ntouka => 8_000.0,
                theropod::Species::Odonto => 8_000.0,
                theropod::Species::Dodarock => 700.0,
                theropod::Species::Sandraptor => 500.0,
                theropod::Species::Snowraptor => 500.0,
                theropod::Species::Sunlizard => 500.0,
                theropod::Species::Woodraptor => 500.0,
                theropod::Species::Yale => 1_000.0,
                theropod::Species::Axebeak => 300.0,
            },
            Body::Ship(ship) => ship.mass().0,
            Body::Arthropod(_) => 200.0,
            // TODO: mass
            Body::Crustacean(body) => match body.species {
                crustacean::Species::Crab | crustacean::Species::SoldierCrab => 50.0,
                crustacean::Species::Karkatha => 1200.0,
            },
            Body::Plugin(body) => body.mass().0,
        };
        Mass(m)
    }

    /// The width (shoulder to shoulder), length (nose to tail) and height
    /// respectively (in metres)
    // Code reviewers: should we replace metres with 'block height'?
    pub fn dimensions(&self) -> Vec3<f32> {
        match self {
            Body::BipedLarge(body) => match body.species {
                biped_large::Species::Cyclops => Vec3::new(5.6, 3.0, 8.0),
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
                biped_large::Species::Tursus => Vec3::new(4.0, 3.0, 4.0),
                biped_large::Species::Gigasfrost => Vec3::new(6.0, 3.0, 11.0),
                biped_large::Species::Gigasfire => Vec3::new(6.0, 3.0, 11.0),
                biped_large::Species::AdletElder => Vec3::new(3.5, 3.0, 5.0),
                biped_large::Species::SeaBishop => Vec3::new(3.7, 2.5, 4.2),
                biped_large::Species::HaniwaGeneral => Vec3::new(3.3, 2.3, 3.8),
                biped_large::Species::TerracottaBesieger => Vec3::new(3.8, 3.0, 5.0),
                biped_large::Species::TerracottaDemolisher => Vec3::new(3.3, 2.5, 3.8),
                biped_large::Species::TerracottaPunisher => Vec3::new(3.3, 2.5, 3.8),
                biped_large::Species::TerracottaPursuer => Vec3::new(3.3, 2.5, 3.8),
                biped_large::Species::Cursekeeper => Vec3::new(3.8, 3.0, 5.0),
                biped_large::Species::Forgemaster => Vec3::new(6.5, 5.0, 8.0),
                biped_large::Species::Strigoi => Vec3::new(3.8, 3.0, 5.0),
                biped_large::Species::Executioner => Vec3::new(2.8, 2.8, 4.7),
                biped_large::Species::Tidalwarrior => Vec3::new(7.0, 8.2, 6.8),
                _ => Vec3::new(4.6, 3.0, 6.0),
            },
            Body::BipedSmall(body) => match body.species {
                biped_small::Species::Gnarling => Vec3::new(1.0, 0.75, 1.4),
                biped_small::Species::Haniwa => Vec3::new(1.3, 1.0, 2.2),
                biped_small::Species::Adlet => Vec3::new(1.3, 1.0, 2.0),
                biped_small::Species::Sahagin => Vec3::new(1.3, 2.0, 1.7),
                biped_small::Species::Myrmidon => Vec3::new(1.3, 1.0, 2.2),
                biped_small::Species::Husk => Vec3::new(1.7, 0.7, 2.7),
                biped_small::Species::Boreal => Vec3::new(2.6, 2.0, 4.6),
                biped_small::Species::Ashen => Vec3::new(1.3, 2.0, 2.5),
                biped_small::Species::Bushly => Vec3::new(1.2, 1.3, 1.6),
                biped_small::Species::Cactid => Vec3::new(1.0, 0.75, 1.4),
                biped_small::Species::Irrwurz => Vec3::new(1.5, 1.5, 2.0),
                biped_small::Species::IronDwarf => Vec3::new(1.3, 2.0, 2.5),
                biped_small::Species::ShamanicSpirit => Vec3::new(1.3, 2.0, 2.3),
                biped_small::Species::Jiangshi => Vec3::new(1.3, 1.8, 2.5),
                biped_small::Species::Flamekeeper => Vec3::new(1.5, 1.5, 2.5),
                biped_small::Species::TreasureEgg => Vec3::new(1.1, 1.1, 1.4),
                biped_small::Species::GnarlingChieftain => Vec3::new(1.0, 0.75, 1.4),
                biped_small::Species::BloodmoonHeiress => Vec3::new(3.5, 3.5, 5.5),
                biped_small::Species::Bloodservant => Vec3::new(1.3, 1.8, 2.5),
                biped_small::Species::Harlequin => Vec3::new(1.3, 2.0, 2.5),
                biped_small::Species::GoblinThug => Vec3::new(1.3, 1.0, 1.6),
                biped_small::Species::GoblinChucker => Vec3::new(1.3, 1.0, 1.6),
                biped_small::Species::GoblinRuffian => Vec3::new(1.3, 1.0, 1.6),
                biped_small::Species::GreenLegoom => Vec3::new(1.4, 1.2, 1.8),
                biped_small::Species::OchreLegoom => Vec3::new(1.4, 1.2, 1.8),
                biped_small::Species::PurpleLegoom => Vec3::new(1.4, 1.2, 1.8),
                biped_small::Species::RedLegoom => Vec3::new(1.4, 1.2, 1.8),
                _ => Vec3::new(1.0, 0.75, 1.4),
            },
            Body::BirdLarge(body) => match body.species {
                bird_large::Species::Cockatrice => Vec3::new(2.5, 5.5, 3.5),
                bird_large::Species::Roc => Vec3::new(2.2, 7.5, 4.0),
                bird_large::Species::FlameWyvern
                | bird_large::Species::FrostWyvern
                | bird_large::Species::CloudWyvern
                | bird_large::Species::SeaWyvern
                | bird_large::Species::WealdWyvern => Vec3::new(2.5, 9.0, 4.5),
                _ => Vec3::new(2.0, 6.0, 4.4),
            },
            Body::Dragon(_) => Vec3::new(16.0, 10.0, 16.0),
            Body::FishMedium(_) => Vec3::new(0.5, 2.0, 0.8),
            Body::FishSmall(_) => Vec3::new(0.3, 1.2, 0.6),
            Body::Golem(body) => match body.species {
                golem::Species::CoralGolem => Vec3::new(3.0, 5.0, 4.0),
                golem::Species::ClayGolem => Vec3::new(6.8, 3.5, 7.5),
                golem::Species::AncientEffigy => Vec3::new(2.5, 2.5, 3.8),
                golem::Species::Mogwai => Vec3::new(2.5, 2.5, 3.8),
                _ => Vec3::new(5.0, 4.5, 7.5),
            },
            Body::Humanoid(humanoid) => {
                let height = humanoid.height();
                Vec3::new(height / 1.3, 1.75 / 2.0, height)
            },
            Body::Object(object) => object.dimensions(),
            Body::Item(item) => item.dimensions(),
            Body::QuadrupedMedium(body) => match body.species {
                quadruped_medium::Species::Akhlut => Vec3::new(2.5, 7.0, 3.0),
                quadruped_medium::Species::Barghest => Vec3::new(2.0, 4.4, 2.7),
                quadruped_medium::Species::Bear => Vec3::new(2.0, 3.8, 3.0),
                quadruped_medium::Species::Catoblepas => Vec3::new(2.0, 4.0, 2.3),
                quadruped_medium::Species::Cattle => Vec3::new(2.0, 3.6, 2.4),
                quadruped_medium::Species::Deer => Vec3::new(2.0, 3.0, 2.2),
                quadruped_medium::Species::Dreadhorn => Vec3::new(3.5, 7.0, 4.0),
                quadruped_medium::Species::Frostfang => Vec3::new(1.5, 3.0, 1.5),
                quadruped_medium::Species::Grolgar => Vec3::new(2.0, 4.0, 2.0),
                quadruped_medium::Species::Highland => Vec3::new(2.0, 3.6, 2.4),
                quadruped_medium::Species::Horse => Vec3::new(1.6, 3.0, 2.4),
                quadruped_medium::Species::Lion => Vec3::new(2.0, 3.3, 2.0),
                quadruped_medium::Species::Moose => Vec3::new(2.0, 4.0, 2.5),
                quadruped_medium::Species::Bristleback => Vec3::new(2.0, 3.0, 2.0),
                quadruped_medium::Species::Roshwalr => Vec3::new(3.4, 5.2, 3.7),
                quadruped_medium::Species::Saber => Vec3::new(2.0, 3.0, 2.0),
                quadruped_medium::Species::Tarasque => Vec3::new(2.0, 4.0, 2.6),
                quadruped_medium::Species::Yak => Vec3::new(2.0, 3.6, 3.0),
                quadruped_medium::Species::Mammoth => Vec3::new(7.5, 11.5, 8.0),
                quadruped_medium::Species::Ngoubou => Vec3::new(2.0, 3.2, 2.4),
                quadruped_medium::Species::Llama => Vec3::new(2.0, 2.5, 2.6),
                quadruped_medium::Species::ClaySteed => Vec3::new(2.2, 4.8, 4.0),
                quadruped_medium::Species::Alpaca => Vec3::new(2.0, 2.0, 2.0),
                quadruped_medium::Species::Camel => Vec3::new(2.0, 4.0, 3.5),
                quadruped_medium::Species::Wolf => Vec3::new(1.25, 3.0, 1.8),
                // FIXME: We really shouldn't be doing wildcards here
                _ => Vec3::new(2.0, 3.0, 2.0),
            },
            Body::QuadrupedSmall(body) => match body.species {
                quadruped_small::Species::Batfox => Vec3::new(1.4, 1.7, 1.3),
                quadruped_small::Species::Holladon => Vec3::new(1.3, 1.9, 1.5),
                quadruped_small::Species::Hyena => Vec3::new(1.2, 1.4, 1.3),
                quadruped_small::Species::Truffler => Vec3::new(1.2, 1.8, 2.2),
                quadruped_small::Species::MossySnail => Vec3::new(1.4, 1.4, 1.2),
                _ => Vec3::new(1.2, 1.2, 1.0),
            },
            Body::QuadrupedLow(body) => match body.species {
                quadruped_low::Species::Asp => Vec3::new(2.0, 3.0, 1.7),
                quadruped_low::Species::Crocodile => Vec3::new(1.0, 2.8, 1.3),
                quadruped_low::Species::SeaCrocodile => Vec3::new(1.2, 4.5, 1.3),
                quadruped_low::Species::Deadwood => Vec3::new(1.3, 1.3, 1.4),
                quadruped_low::Species::Hakulaq => Vec3::new(1.8, 3.0, 2.0),
                quadruped_low::Species::Dagon => Vec3::new(3.0, 6.0, 2.0),
                quadruped_low::Species::Icedrake => Vec3::new(2.0, 5.5, 2.5),
                quadruped_low::Species::Lavadrake => Vec3::new(2.0, 5.5, 2.5),
                quadruped_low::Species::Mossdrake => Vec3::new(2.0, 5.5, 2.5),
                quadruped_low::Species::Maneater => Vec3::new(2.0, 3.7, 4.0),
                quadruped_low::Species::Monitor => Vec3::new(1.4, 3.2, 1.3),
                quadruped_low::Species::Pangolin => Vec3::new(1.0, 2.6, 1.1),
                quadruped_low::Species::Rocksnapper => Vec3::new(2.5, 3.5, 2.9),
                quadruped_low::Species::Rootsnapper => Vec3::new(2.5, 3.5, 2.9),
                quadruped_low::Species::Reefsnapper => Vec3::new(2.5, 3.5, 2.9),
                quadruped_low::Species::Sandshark => Vec3::new(2.1, 4.3, 1.7),
                quadruped_low::Species::Basilisk => Vec3::new(2.7, 6.0, 2.9),
                quadruped_low::Species::Salamander => Vec3::new(1.7, 4.0, 1.3),
                quadruped_low::Species::Elbst => Vec3::new(1.7, 4.0, 1.3),
                quadruped_low::Species::Tortoise => Vec3::new(1.7, 2.7, 1.5),
                quadruped_low::Species::Driggle => Vec3::new(1.6, 2.7, 1.0),
                quadruped_low::Species::Snaretongue => Vec3::new(2.0, 2.8, 1.6),
                quadruped_low::Species::Hydra => Vec3::new(3.0, 5.0, 2.8),
                _ => Vec3::new(1.0, 1.6, 1.3),
            },
            Body::Ship(ship) => ship.dimensions(),
            Body::Theropod(body) => match body.species {
                theropod::Species::Archaeos => Vec3::new(4.5, 8.5, 8.0),
                theropod::Species::Ntouka => Vec3::new(4.5, 9.0, 6.6),
                theropod::Species::Odonto => Vec3::new(4.5, 8.0, 6.6),
                theropod::Species::Dodarock => Vec3::new(2.0, 3.0, 2.6),
                theropod::Species::Sandraptor => Vec3::new(2.0, 3.0, 2.6),
                theropod::Species::Snowraptor => Vec3::new(2.0, 3.0, 2.6),
                theropod::Species::Sunlizard => Vec3::new(2.0, 3.6, 2.5),
                theropod::Species::Woodraptor => Vec3::new(2.0, 3.0, 2.6),
                theropod::Species::Yale => Vec3::new(2.0, 3.2, 4.0),
                theropod::Species::Axebeak => Vec3::new(2.0, 3.6, 3.0),
            },
            Body::Arthropod(body) => match body.species {
                arthropod::Species::Tarantula => Vec3::new(4.0, 4.0, 1.8),
                arthropod::Species::Blackwidow => Vec3::new(4.0, 4.0, 2.0),
                arthropod::Species::Antlion => Vec3::new(4.0, 4.0, 2.2),
                arthropod::Species::Hornbeetle => Vec3::new(3.2, 3.2, 1.3),
                arthropod::Species::Leafbeetle => Vec3::new(2.4, 2.8, 1.2),
                arthropod::Species::Stagbeetle => Vec3::new(3.2, 3.2, 1.3),
                arthropod::Species::Weevil => Vec3::new(2.2, 2.4, 1.1),
                arthropod::Species::Cavespider => Vec3::new(4.0, 4.0, 1.4),
                arthropod::Species::Moltencrawler => Vec3::new(3.2, 4.0, 1.5),
                arthropod::Species::Mosscrawler => Vec3::new(3.2, 4.0, 1.4),
                arthropod::Species::Sandcrawler => Vec3::new(3.2, 4.0, 1.4),
                arthropod::Species::Dagonite => Vec3::new(3.2, 4.7, 1.4),
                arthropod::Species::Emberfly => Vec3::new(1.3, 1.5, 0.9),
            },
            Body::BirdMedium(body) => match body.species {
                bird_medium::Species::SnowyOwl => Vec3::new(1.2, 1.2, 0.9),
                bird_medium::Species::HornedOwl => Vec3::new(1.2, 1.2, 0.9),
                bird_medium::Species::Duck => Vec3::new(0.8, 1.3, 0.8),
                bird_medium::Species::Cockatiel => Vec3::new(0.8, 1.0, 0.7),
                bird_medium::Species::Chicken => Vec3::new(1.2, 1.5, 0.9),
                bird_medium::Species::Bat => Vec3::new(2.0, 1.8, 1.3),
                bird_medium::Species::Penguin => Vec3::new(1.0, 1.0, 1.2),
                bird_medium::Species::Goose => Vec3::new(1.5, 1.5, 1.1),
                bird_medium::Species::Peacock => Vec3::new(1.6, 1.8, 1.4),
                bird_medium::Species::Eagle => Vec3::new(1.5, 2.2, 1.0),
                bird_medium::Species::Parrot => Vec3::new(1.2, 1.5, 1.1),
                bird_medium::Species::Crow => Vec3::new(1.0, 1.2, 0.8),
                bird_medium::Species::Dodo => Vec3::new(1.2, 1.8, 1.1),
                bird_medium::Species::Parakeet => Vec3::new(0.8, 0.9, 0.7),
                bird_medium::Species::Puffin => Vec3::new(1.0, 1.0, 1.0),
                bird_medium::Species::Toucan => Vec3::new(2.1, 1.1, 1.2),
                bird_medium::Species::BloodmoonBat => Vec3::new(3.5, 3.5, 2.5),
                bird_medium::Species::VampireBat => Vec3::new(2.0, 1.8, 1.3),
            },
            Body::Crustacean(body) => match body.species {
                crustacean::Species::Crab => Vec3::new(1.2, 1.2, 0.7),
                crustacean::Species::SoldierCrab => Vec3::new(1.2, 1.2, 1.0),
                crustacean::Species::Karkatha => Vec3::new(10.0, 10.0, 7.5),
            },
            Body::Plugin(body) => body.dimensions(),
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

    pub fn front_radius(&self) -> f32 { self.dimensions().y / 2.0 }

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

    /// Body collider
    pub fn collider(&self) -> Collider {
        if let Body::Ship(ship) = self {
            ship.make_collider()
        } else {
            let (p0, p1, radius) = self.sausage();

            Collider::CapsulePrism {
                p0,
                p1,
                radius,
                z_min: 0.0,
                z_max: self.height(),
            }
        }
    }

    /// How far away other entities should try to be. Will be added upon the
    /// other entity's spacing_radius. So an entity with 2.0 and an entity
    /// with 3.0 will lead to that both entities will try to keep 5.0 units
    /// away from each other.
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
                biped_large::Species::Cultistwarlord | biped_large::Species::Cultistwarlock => 240,
                _ => 300,
            },
            Body::BirdLarge(body) => match body.species {
                bird_large::Species::Cockatrice => 400,
                bird_large::Species::Phoenix => 600,
                bird_large::Species::Roc => 500,
                bird_large::Species::FlameWyvern => 600,
                bird_large::Species::CloudWyvern => 600,
                bird_large::Species::FrostWyvern => 600,
                bird_large::Species::SeaWyvern => 600,
                bird_large::Species::WealdWyvern => 600,
            },
            Body::Humanoid(_) => 100,
            Body::Object(object::Body::Crux) => 0,
            _ => 100,
        }
    }

    /// If this body will retain 1 hp when it would die, and consume death
    /// protection, and entering a downed state.
    pub fn has_death_protection(&self) -> bool { matches!(self, Body::Humanoid(_)) }

    pub fn base_health(&self) -> u16 {
        match self {
            Body::Humanoid(_) => 100,
            Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
                // T1
                quadruped_small::Species::Batfox => 40,
                quadruped_small::Species::Boar => 55,
                quadruped_small::Species::Fox => 25,
                quadruped_small::Species::Goat => 30,
                quadruped_small::Species::Hare => 20,
                quadruped_small::Species::Holladon => 25,
                quadruped_small::Species::Jackalope => 30,
                quadruped_small::Species::MossySnail => 15,
                quadruped_small::Species::Porcupine => 25,
                quadruped_small::Species::Sheep => 30,
                quadruped_small::Species::TreantSapling => 20,
                quadruped_small::Species::Truffler => 70,
                // T2
                quadruped_small::Species::Hyena => 85,
                // T0
                quadruped_small::Species::Beaver => 20,
                quadruped_small::Species::Cat => 25,
                quadruped_small::Species::Dog => 30,
                quadruped_small::Species::Fungome => 15,
                quadruped_small::Species::Pig => 25,
                quadruped_small::Species::Quokka => 15,
                quadruped_small::Species::Rabbit => 15,
                quadruped_small::Species::Raccoon => 20,
                quadruped_small::Species::Rat => 10,
                quadruped_small::Species::Seal => 20,
                quadruped_small::Species::Skunk => 20,
                quadruped_small::Species::Turtle => 10,
                _ => 5,
            },
            Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                // T1
                quadruped_medium::Species::Alpaca => 55,
                quadruped_medium::Species::Antelope => 70,
                quadruped_medium::Species::Darkhound => 80,
                quadruped_medium::Species::Camel => 100,
                quadruped_medium::Species::Cattle => 90,
                quadruped_medium::Species::Deer => 55,
                quadruped_medium::Species::Donkey => 65,
                quadruped_medium::Species::Horse => 75,
                quadruped_medium::Species::Llama => 65,
                quadruped_medium::Species::Mouflon => 75,
                quadruped_medium::Species::Zebra => 90,
                // T2
                quadruped_medium::Species::Barghest => 120,
                quadruped_medium::Species::Bear => 240,
                quadruped_medium::Species::Bristleback => 175,
                quadruped_medium::Species::Bonerattler => 100,
                quadruped_medium::Species::Frostfang => 185,
                quadruped_medium::Species::Highland => 205,
                quadruped_medium::Species::Kelpie => 150,
                quadruped_medium::Species::Lion => 175,
                quadruped_medium::Species::Moose => 265,
                quadruped_medium::Species::Panda => 215,
                quadruped_medium::Species::Saber => 210,
                quadruped_medium::Species::Snowleopard => 175,
                quadruped_medium::Species::Tiger => 205,
                quadruped_medium::Species::Tuskram => 175,
                quadruped_medium::Species::Wolf => 110,
                quadruped_medium::Species::Yak => 215,
                // T3A
                quadruped_medium::Species::Akhlut => 720,
                quadruped_medium::Species::Catoblepas => 720,
                quadruped_medium::Species::ClaySteed => 400,
                quadruped_medium::Species::Dreadhorn => 690,
                quadruped_medium::Species::Grolgar => 450,
                quadruped_medium::Species::Hirdrasil => 480,
                quadruped_medium::Species::Mammoth => 880,
                quadruped_medium::Species::Ngoubou => 590,
                quadruped_medium::Species::Roshwalr => 640,
                quadruped_medium::Species::Tarasque => 370,
            },
            Body::FishMedium(fish_medium) => match fish_medium.species {
                // T2
                fish_medium::Species::Marlin => 50,
                fish_medium::Species::Icepike => 90,
            },
            Body::Dragon(_) => 500,
            Body::BirdLarge(bird_large) => match bird_large.species {
                // T3A
                bird_large::Species::Cockatrice => 540,
                bird_large::Species::Roc => 450,
                // T3B
                bird_large::Species::FlameWyvern
                | bird_large::Species::CloudWyvern
                | bird_large::Species::FrostWyvern
                | bird_large::Species::SeaWyvern
                | bird_large::Species::WealdWyvern => 1000,
                bird_large::Species::Phoenix => 2000,
            },
            Body::BirdMedium(bird_medium) => match bird_medium.species {
                // T0
                bird_medium::Species::Bat => 10,
                bird_medium::Species::Chicken => 10,
                bird_medium::Species::Cockatiel => 10,
                bird_medium::Species::Dodo => 20,
                bird_medium::Species::Duck => 10,
                bird_medium::Species::Parakeet => 10,
                bird_medium::Species::Peacock => 20,
                bird_medium::Species::Penguin => 10,
                bird_medium::Species::Puffin => 20,
                // T1
                bird_medium::Species::Crow => 15,
                bird_medium::Species::Eagle => 35,
                bird_medium::Species::Goose => 25,
                bird_medium::Species::HornedOwl => 35,
                bird_medium::Species::Parrot => 15,
                bird_medium::Species::SnowyOwl => 35,
                bird_medium::Species::Toucan => 15,
                // T3B
                bird_medium::Species::VampireBat => 100,
                bird_medium::Species::BloodmoonBat => 1200,
            },
            Body::FishSmall(fish_small) => match fish_small.species {
                // T0
                fish_small::Species::Clownfish => 5,
                // T1
                fish_small::Species::Piranha => 10,
            },
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Ogre => 320,
                biped_large::Species::Cyclops => 1000,
                biped_large::Species::Wendigo => 280,
                biped_large::Species::Cavetroll => 240,
                biped_large::Species::Mountaintroll => 240,
                biped_large::Species::Swamptroll => 240,
                biped_large::Species::Dullahan => 600,
                biped_large::Species::Mindflayer => 2000,
                biped_large::Species::Tidalwarrior => 1600,
                biped_large::Species::Yeti => 1800,
                biped_large::Species::Minotaur => 3000,
                biped_large::Species::Harvester => 1300,
                biped_large::Species::Blueoni => 240,
                biped_large::Species::Redoni => 240,
                biped_large::Species::Huskbrute => 800,
                biped_large::Species::Cultistwarlord => 200,
                biped_large::Species::Cultistwarlock => 200,
                biped_large::Species::Gigasfrost => 30000,
                biped_large::Species::Gigasfire => 25000,
                biped_large::Species::AdletElder => 1500,
                biped_large::Species::Tursus => 300,
                biped_large::Species::SeaBishop => 550,
                biped_large::Species::HaniwaGeneral => 600,
                biped_large::Species::TerracottaBesieger
                | biped_large::Species::TerracottaDemolisher
                | biped_large::Species::TerracottaPunisher
                | biped_large::Species::TerracottaPursuer => 300,
                biped_large::Species::Cursekeeper => 3000,
                biped_large::Species::Forgemaster => 10000,
                biped_large::Species::Strigoi => 800,
                biped_large::Species::Executioner => 800,
                _ => 120,
            },
            Body::BipedSmall(biped_small) => match biped_small.species {
                biped_small::Species::GoblinThug
                | biped_small::Species::GoblinChucker
                | biped_small::Species::GoblinRuffian => 30,
                biped_small::Species::GreenLegoom
                | biped_small::Species::OchreLegoom
                | biped_small::Species::PurpleLegoom
                | biped_small::Species::RedLegoom => 40,
                biped_small::Species::Cactid => 50,
                biped_small::Species::Gnarling => 50,
                biped_small::Species::GnarlingChieftain => 150,
                biped_small::Species::Mandragora => 65,
                biped_small::Species::Adlet => 65,
                biped_small::Species::Sahagin => 85,
                biped_small::Species::Haniwa => 100,
                biped_small::Species::Myrmidon => 100,
                biped_small::Species::Husk => 50,
                biped_small::Species::Boreal => 800,
                biped_small::Species::Ashen => 300,
                biped_small::Species::IronDwarf => 250,
                biped_small::Species::Irrwurz => 100,
                biped_small::Species::ShamanicSpirit => 240,
                biped_small::Species::Jiangshi => 250,
                biped_small::Species::Flamekeeper => 2000,
                biped_small::Species::BloodmoonHeiress => 2000,
                biped_small::Species::Bloodservant => 300,
                biped_small::Species::Harlequin => 500,
                _ => 60,
            },
            Body::Object(object) => match object {
                object::Body::TrainingDummy => 60000,
                object::Body::Crossbow => 80,
                object::Body::Flamethrower => 80,
                object::Body::Lavathrower => 80,
                object::Body::BarrelOrgan => 500,
                object::Body::HaniwaSentry => 60,
                object::Body::SeaLantern => 100,
                object::Body::TerracottaStatue => 600,
                object::Body::GnarlingTotemGreen => 15,
                object::Body::GnarlingTotemRed | object::Body::GnarlingTotemWhite => 15,
                object::Body::Crux => 350,
                _ => 1000,
            },
            Body::Item(_) => 1000,
            Body::Golem(golem) => match golem.species {
                golem::Species::WoodGolem => 120,
                golem::Species::ClayGolem => 350,
                golem::Species::Gravewarden => 1000,
                golem::Species::CoralGolem => 550,
                golem::Species::AncientEffigy => 250,
                golem::Species::Mogwai => 500,
                golem::Species::IronGolem => 2500,
                _ => 1000,
            },
            Body::Theropod(theropod) => match theropod.species {
                // T1
                theropod::Species::Dodarock => 20,
                // T2
                theropod::Species::Axebeak => 275,
                theropod::Species::Sandraptor => 110,
                theropod::Species::Snowraptor => 110,
                theropod::Species::Sunlizard => 110,
                theropod::Species::Woodraptor => 110,
                // T3A
                theropod::Species::Yale => 610,
                // T3B
                theropod::Species::Archaeos => 880,
                theropod::Species::Ntouka => 880,
                theropod::Species::Odonto => 1320,
            },
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                // T1
                quadruped_low::Species::Driggle => 50,
                quadruped_low::Species::Pangolin => 20,
                quadruped_low::Species::Tortoise => 45,
                // T2
                quadruped_low::Species::Alligator => 130,
                quadruped_low::Species::Asp => 175,
                quadruped_low::Species::Crocodile => 145,
                quadruped_low::Species::Deadwood => 85,
                quadruped_low::Species::Elbst => 145,
                quadruped_low::Species::Hakulaq => 155,
                quadruped_low::Species::Monitor => 95,
                quadruped_low::Species::Salamander => 210,
                quadruped_low::Species::SeaCrocodile => 180,
                // T3A
                quadruped_low::Species::Dagon => 1200,
                quadruped_low::Species::Icedrake => 340,
                quadruped_low::Species::Lavadrake => 340,
                quadruped_low::Species::Maneater => 510,
                quadruped_low::Species::Mossdrake => 340,
                quadruped_low::Species::Rocksnapper => 400,
                quadruped_low::Species::Reefsnapper => 400,
                quadruped_low::Species::Rootsnapper => 400,
                quadruped_low::Species::Sandshark => 540,
                quadruped_low::Species::Hydra => 1000,
                // T3B
                quadruped_low::Species::Basilisk => 660,
                quadruped_low::Species::Snaretongue => 1500,
            },
            Body::Arthropod(arthropod) => match arthropod.species {
                // T1
                arthropod::Species::Dagonite => 70,
                arthropod::Species::Emberfly => 20,
                arthropod::Species::Leafbeetle => 40,
                arthropod::Species::Weevil => 40,
                // T2
                arthropod::Species::Cavespider => 170,
                arthropod::Species::Hornbeetle => 170,
                arthropod::Species::Moltencrawler => 145,
                arthropod::Species::Mosscrawler => 145,
                arthropod::Species::Sandcrawler => 145,
                arthropod::Species::Stagbeetle => 170,
                arthropod::Species::Tarantula => 155,
                // T3A
                arthropod::Species::Antlion => 480,
                arthropod::Species::Blackwidow => 370,
            },
            Body::Ship(_) => 1000,
            Body::Crustacean(crustacean) => match crustacean.species {
                // T0
                crustacean::Species::Crab => 40,
                // T2
                crustacean::Species::SoldierCrab => 50,
                crustacean::Species::Karkatha => 2000,
            },
            Body::Plugin(body) => body.base_health(),
        }
    }

    pub fn flying_height(&self) -> f32 {
        match self {
            Body::BirdLarge(_) => 50.0,
            Body::BirdMedium(_) => 40.0,
            Body::Dragon(_) => 60.0,
            Body::Ship(ship) => ship.flying_height(),
            _ => 0.0,
        }
    }

    pub fn immune_to(&self, buff: BuffKind) -> bool {
        match buff {
            BuffKind::Bleeding => match self {
                Body::Golem(_) | Body::Ship(_) => true,
                Body::Object(object) => !matches!(object, object::Body::TrainingDummy),
                Body::BipedSmall(b) => matches!(
                    b.species,
                    biped_small::Species::Husk
                        | biped_small::Species::Boreal
                        | biped_small::Species::IronDwarf
                        | biped_small::Species::Haniwa
                        | biped_small::Species::ShamanicSpirit
                        | biped_small::Species::Jiangshi
                ),
                Body::BipedLarge(b) => matches!(
                    b.species,
                    biped_large::Species::Huskbrute
                        | biped_large::Species::Gigasfrost
                        | biped_large::Species::Gigasfire
                        | biped_large::Species::Dullahan
                        | biped_large::Species::HaniwaGeneral
                        | biped_large::Species::TerracottaBesieger
                        | biped_large::Species::TerracottaDemolisher
                        | biped_large::Species::TerracottaPunisher
                        | biped_large::Species::TerracottaPursuer
                        | biped_large::Species::Cursekeeper
                ),
                Body::QuadrupedMedium(b) => {
                    matches!(b.species, quadruped_medium::Species::ClaySteed)
                },
                _ => false,
            },
            BuffKind::Crippled => match self {
                Body::Golem(_) | Body::Ship(_) => true,
                Body::Object(object) => !matches!(object, object::Body::TrainingDummy),
                Body::BipedLarge(b) => matches!(
                    b.species,
                    biped_large::Species::Dullahan | biped_large::Species::HaniwaGeneral
                ),
                Body::BipedSmall(b) => matches!(b.species, biped_small::Species::Haniwa),
                Body::QuadrupedMedium(b) => {
                    matches!(b.species, quadruped_medium::Species::ClaySteed)
                },
                _ => false,
            },
            BuffKind::Burning => match self {
                Body::Golem(g) => matches!(
                    g.species,
                    golem::Species::Gravewarden
                        | golem::Species::AncientEffigy
                        | golem::Species::IronGolem
                ),
                Body::BipedSmall(b) => matches!(
                    b.species,
                    biped_small::Species::Haniwa
                        | biped_small::Species::Flamekeeper
                        | biped_small::Species::IronDwarf
                ),
                Body::Object(object) => matches!(
                    object,
                    object::Body::HaniwaSentry
                        | object::Body::Lavathrower
                        | object::Body::Flamethrower
                        | object::Body::TerracottaStatue
                        | object::Body::Crux
                ),
                Body::QuadrupedLow(q) => matches!(
                    q.species,
                    quadruped_low::Species::Lavadrake | quadruped_low::Species::Salamander
                ),
                Body::BirdLarge(b) => matches!(
                    b.species,
                    bird_large::Species::Phoenix
                        | bird_large::Species::Cockatrice
                        | bird_large::Species::FlameWyvern
                        | bird_large::Species::CloudWyvern
                        | bird_large::Species::FrostWyvern
                        | bird_large::Species::SeaWyvern
                        | bird_large::Species::WealdWyvern
                ),
                Body::Arthropod(b) => matches!(b.species, arthropod::Species::Moltencrawler),
                Body::BipedLarge(b) => matches!(
                    b.species,
                    biped_large::Species::Cyclops
                        | biped_large::Species::Minotaur
                        | biped_large::Species::Forgemaster
                        | biped_large::Species::Gigasfire
                ),
                _ => false,
            },
            BuffKind::Ensnared => match self {
                Body::BipedLarge(b) => matches!(b.species, biped_large::Species::Harvester),
                Body::Arthropod(_) => true,
                _ => false,
            },
            BuffKind::Regeneration => {
                matches!(
                    self,
                    Body::Object(
                        object::Body::GnarlingTotemRed
                            | object::Body::GnarlingTotemGreen
                            | object::Body::GnarlingTotemWhite
                            | object::Body::Crux
                    )
                )
            },
            BuffKind::Frozen => match self {
                Body::BipedLarge(b) => matches!(
                    b.species,
                    biped_large::Species::Yeti
                        | biped_large::Species::Gigasfrost
                        | biped_large::Species::Tursus
                ),
                Body::QuadrupedLow(q) => matches!(q.species, quadruped_low::Species::Icedrake),
                Body::BirdLarge(b) => matches!(b.species, bird_large::Species::FrostWyvern),
                Body::BipedSmall(b) => matches!(b.species, biped_small::Species::Boreal),
                Body::QuadrupedMedium(b) => matches!(
                    b.species,
                    quadruped_medium::Species::Roshwalr | quadruped_medium::Species::Frostfang
                ),
                _ => false,
            },
            BuffKind::ProtectingWard => matches!(self, Body::Object(object::Body::BarrelOrgan)),
            _ => false,
        }
    }

    // Entity still recieves the buff to allow for particle rendering or other
    // secondary effects, but still removes any direct `BuffEffect`
    pub fn negates_buff(&self, buff: BuffKind) -> bool {
        self.immune_to(buff)
            || match buff {
                BuffKind::Burning => match self {
                    Body::BipedSmall(b) => matches!(b.species, biped_small::Species::Ashen),
                    _ => false,
                },
                _ => false,
            }
    }

    /// Returns a multiplier representing increased difficulty not accounted for
    /// due to AI or not using an actual weapon
    // TODO: Match on species
    pub fn combat_multiplier(&self) -> f32 {
        match self {
            Body::Object(object) => match object {
                object::Body::BarrelOrgan | object::Body::ArrowTurret => 0.05,
                object::Body::TerracottaStatue => 1.5,
                _ => 0.0,
            },
            Body::Ship(_) => 0.0,
            Body::BipedLarge(b) => match b.species {
                biped_large::Species::Mindflayer => 4.35,
                biped_large::Species::Minotaur => 4.05,
                biped_large::Species::Tidalwarrior => 2.75,
                biped_large::Species::Yeti => 2.25,
                _ => 1.0,
            },
            Body::BipedSmall(b) => match b.species {
                biped_small::Species::IronDwarf => 2.0,
                biped_small::Species::Flamekeeper => 4.0,
                _ => 1.0,
            },
            Body::Golem(g) => match g.species {
                golem::Species::Gravewarden => 2.45,
                _ => 1.0,
            },
            Body::QuadrupedLow(b) => match b.species {
                quadruped_low::Species::Snaretongue => 2.0,
                _ => 1.0,
            },
            Body::QuadrupedSmall(b) => match b.species {
                quadruped_small::Species::Axolotl | quadruped_small::Species::Gecko => 0.6,
                _ => 1.0,
            },
            #[expect(unreachable_patterns)] // TODO: Remove when more medium fish species are added
            Body::FishMedium(b) => match b.species {
                fish_medium::Species::Marlin | fish_medium::Species::Icepike => 0.6,
                _ => 1.0,
            },
            #[expect(unreachable_patterns)] // TODO: Remove when more small fish species are added
            Body::FishSmall(b) => match b.species {
                fish_small::Species::Clownfish | fish_small::Species::Piranha => 0.6,
                _ => 1.0,
            },
            Body::Crustacean(b) => match b.species {
                crustacean::Species::Crab | crustacean::Species::SoldierCrab => 0.6,
                _ => 1.0,
            },
            _ => 1.0,
        }
    }

    pub fn base_poise(&self) -> u16 {
        match self {
            Body::Humanoid(_) => 100,
            Body::BipedLarge(biped_large) => match biped_large.species {
                biped_large::Species::Mindflayer => 777,
                biped_large::Species::Minotaur => 340,
                biped_large::Species::Forgemaster => 300,
                biped_large::Species::Gigasfrost => 990,
                biped_large::Species::Gigasfire => 990,
                _ => 300,
            },
            Body::BipedSmall(b) => match b.species {
                biped_small::Species::GnarlingChieftain => 130,
                biped_small::Species::IronDwarf | biped_small::Species::Flamekeeper => 300,
                biped_small::Species::Boreal => 470,
                _ => 100,
            },
            Body::BirdLarge(b) => match b.species {
                bird_large::Species::FlameWyvern
                | bird_large::Species::FrostWyvern
                | bird_large::Species::CloudWyvern
                | bird_large::Species::SeaWyvern
                | bird_large::Species::WealdWyvern => 220,
                _ => 165,
            },
            Body::Golem(_) => 365,
            Body::QuadrupedMedium(b) => match b.species {
                quadruped_medium::Species::Bear | quadruped_medium::Species::Grolgar => 195,
                quadruped_medium::Species::Cattle
                | quadruped_medium::Species::Llama
                | quadruped_medium::Species::Alpaca
                | quadruped_medium::Species::Camel
                | quadruped_medium::Species::ClaySteed
                | quadruped_medium::Species::Zebra
                | quadruped_medium::Species::Donkey
                | quadruped_medium::Species::Highland
                | quadruped_medium::Species::Horse
                | quadruped_medium::Species::Kelpie
                | quadruped_medium::Species::Hirdrasil
                | quadruped_medium::Species::Antelope => 165,
                quadruped_medium::Species::Deer => 140,
                quadruped_medium::Species::Wolf
                | quadruped_medium::Species::Tiger
                | quadruped_medium::Species::Barghest
                | quadruped_medium::Species::Bonerattler
                | quadruped_medium::Species::Darkhound
                | quadruped_medium::Species::Moose
                | quadruped_medium::Species::Snowleopard
                | quadruped_medium::Species::Akhlut
                | quadruped_medium::Species::Bristleback
                | quadruped_medium::Species::Catoblepas
                | quadruped_medium::Species::Lion => 190,
                quadruped_medium::Species::Panda => 150,
                quadruped_medium::Species::Saber
                | quadruped_medium::Species::Yak
                | quadruped_medium::Species::Frostfang
                | quadruped_medium::Species::Tarasque
                | quadruped_medium::Species::Tuskram
                | quadruped_medium::Species::Mouflon
                | quadruped_medium::Species::Roshwalr
                | quadruped_medium::Species::Dreadhorn => 205,
                quadruped_medium::Species::Mammoth | quadruped_medium::Species::Ngoubou => 230,
            },
            Body::QuadrupedLow(b) => match b.species {
                quadruped_low::Species::Dagon => 270,
                quadruped_low::Species::Crocodile
                | quadruped_low::Species::Deadwood
                | quadruped_low::Species::SeaCrocodile
                | quadruped_low::Species::Alligator
                | quadruped_low::Species::Sandshark
                | quadruped_low::Species::Snaretongue
                | quadruped_low::Species::Asp => 190,
                quadruped_low::Species::Tortoise
                | quadruped_low::Species::Rocksnapper
                | quadruped_low::Species::Rootsnapper
                | quadruped_low::Species::Reefsnapper
                | quadruped_low::Species::Maneater
                | quadruped_low::Species::Hakulaq
                | quadruped_low::Species::Lavadrake
                | quadruped_low::Species::Icedrake
                | quadruped_low::Species::Basilisk
                | quadruped_low::Species::Hydra
                | quadruped_low::Species::Mossdrake => 205,
                quadruped_low::Species::Elbst
                | quadruped_low::Species::Salamander
                | quadruped_low::Species::Monitor
                | quadruped_low::Species::Pangolin
                | quadruped_low::Species::Driggle => 130,
            },
            Body::Theropod(b) => match b.species {
                theropod::Species::Archaeos
                | theropod::Species::Ntouka
                | theropod::Species::Odonto => 240,
                theropod::Species::Yale => 220,
                _ => 195,
            },
            _ => 100,
        }
    }

    /// Returns the eye height for this creature.
    pub fn eye_height(&self, scale: f32) -> f32 { self.height() * 0.9 * scale }

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
            Body::Humanoid(_)
                | Body::BipedSmall(_)
                | Body::BipedLarge(_)
                | Body::Crustacean(crustacean::Body {
                    species: crustacean::Species::Crab,
                    ..
                })
                // Allows for reverse gear!
                | Body::Ship(ship::Body::Train)
        )
    }

    /// Component of the mounting offset specific to the mount
    // #[inline_tweak::tweak_fn]
    pub fn mount_offset(&self) -> Vec3<f32> {
        match self {
            Body::Humanoid(_) => (self.dimensions() * Vec3::new(0.5, 0.0, 0.6)).into_tuple(),
            Body::QuadrupedSmall(b) => match (b.species, b.body_type) {
                (quadruped_small::Species::Pig, _) => (0.0, 0.1, 0.6),
                (quadruped_small::Species::Fox, _) => (0.0, 0.1, 0.6),
                (quadruped_small::Species::Sheep, _) => (0.0, 0.1, 0.7),
                (quadruped_small::Species::Boar, _) => (0.0, -0.2, 1.0),
                (quadruped_small::Species::Jackalope, _) => (0.0, -0.1, 0.5),
                (quadruped_small::Species::Skunk, _) => (0.0, 0.0, 0.55),
                (quadruped_small::Species::Cat, _) => (0.0, 0.0, 0.45),
                (quadruped_small::Species::Batfox, _) => (0.0, -0.1, 0.7),
                (quadruped_small::Species::Raccoon, _) => (0.0, 0.0, 0.65),
                (quadruped_small::Species::Quokka, _) => (0.0, 0.1, 0.7),
                (quadruped_small::Species::Goat, _) => (0.0, 0.2, 0.7),
                (quadruped_small::Species::Holladon, _) => (0.0, -0.3, 1.05),
                (quadruped_small::Species::Hyena, _) => (0.0, -0.4, 0.95),
                (quadruped_small::Species::Rabbit, _) => (0.0, -0.05, 0.4),
                (quadruped_small::Species::Truffler, _) => (0.0, -0.5, 1.84),
                (quadruped_small::Species::Frog, _) => (0.0, -0.07, 0.35),
                (quadruped_small::Species::Rat, _) => (0.0, 0.35, 0.35),
                (quadruped_small::Species::Axolotl, _) => (0.0, 0.095, 0.295),
                (quadruped_small::Species::Gecko, _) => (0.0, 0.35, 0.25),
                (quadruped_small::Species::Turtle, _) => (0.0, -0.16, 0.5),
                (quadruped_small::Species::Squirrel, _) => (0.0, 0.16, 0.24),
                (quadruped_small::Species::Fungome, _) => (0.0, 0.03, 0.43),
                (quadruped_small::Species::Porcupine, _) => (0.0, 0.56, 0.74),
                (quadruped_small::Species::Beaver, _) => (0.0, 0.18, 0.7),
                (quadruped_small::Species::Hare, quadruped_small::BodyType::Female) => {
                    (0.0, -0.21, 0.44)
                },
                (quadruped_small::Species::Hare, quadruped_small::BodyType::Male) => {
                    (0.0, -0.21, 0.54)
                },
                (quadruped_small::Species::Dog, _) => (0.0, -0.27, 0.76),
                (quadruped_small::Species::Seal, _) => (0.0, -0.14, 0.54),
                (quadruped_small::Species::TreantSapling, _) => (0.0, -0.2, 1.03),
                (quadruped_small::Species::MossySnail, _) => (0.0, -0.34, 0.96),
            },
            Body::QuadrupedMedium(b) => match (b.species, b.body_type) {
                (quadruped_medium::Species::Grolgar, _) => (0.0, 0.4, 1.98),
                (quadruped_medium::Species::Saber, _) => (0.0, -0.1, 1.3),
                (quadruped_medium::Species::Tiger, _) => (0.0, 0.2, 1.63),
                (quadruped_medium::Species::Tuskram, _) => (0.0, -0.5, 1.5),
                (quadruped_medium::Species::Lion, _) => (0.0, 0.3, 1.5),
                (quadruped_medium::Species::Tarasque, _) => (0.0, 0.6, 2.0),
                (quadruped_medium::Species::Wolf, _) => (0.0, 0.5, 1.3),
                (quadruped_medium::Species::Frostfang, _) => (0.0, 0.05, 1.2),
                (quadruped_medium::Species::Mouflon, _) => (0.0, 0.3, 1.2),
                (quadruped_medium::Species::Catoblepas, _) => (0.0, 0.0, 2.0),
                (quadruped_medium::Species::Bonerattler, _) => (0.0, 0.5, 1.2),
                (quadruped_medium::Species::Deer, _) => (0.0, 0.2, 1.3),
                (quadruped_medium::Species::Hirdrasil, _) => (0.0, 0.0, 1.4),
                (quadruped_medium::Species::Roshwalr, _) => (0.0, 0.9, 3.2),
                (quadruped_medium::Species::Donkey, _) => (0.0, 0.5, 1.6),
                (quadruped_medium::Species::Camel, _) => (0.0, -0.1, 2.7),
                (quadruped_medium::Species::Zebra, _) => (0.0, 0.5, 1.8),
                (quadruped_medium::Species::Antelope, _) => (0.0, 0.3, 1.4),
                (quadruped_medium::Species::Kelpie, _) => (0.0, 0.5, 1.9),
                (quadruped_medium::Species::Horse, _) => (0.0, 0.0, 2.0),
                (quadruped_medium::Species::Barghest, _) => (0.0, 0.5, 2.3),
                (quadruped_medium::Species::Cattle, quadruped_medium::BodyType::Male) => {
                    (0.0, 0.5, 2.5)
                },
                (quadruped_medium::Species::Cattle, quadruped_medium::BodyType::Female) => {
                    (0.0, 0.7, 2.3)
                },
                (quadruped_medium::Species::Darkhound, _) => (0.0, 0.5, 1.4),
                (quadruped_medium::Species::Highland, _) => (0.0, 0.5, 2.3),
                (quadruped_medium::Species::Yak, _) => (0.0, 0.0, 3.0),
                (quadruped_medium::Species::Panda, _) => (0.0, -0.2, 1.4),
                (quadruped_medium::Species::Bear, _) => (0.0, -0.4, 2.35),
                (quadruped_medium::Species::Dreadhorn, _) => (0.0, 0.2, 3.8),
                (quadruped_medium::Species::Moose, _) => (0.0, -0.6, 2.1),
                (quadruped_medium::Species::Snowleopard, _) => (0.0, -0.5, 1.4),
                (quadruped_medium::Species::Mammoth, _) => (0.0, 4.9, 7.2),
                (quadruped_medium::Species::Ngoubou, _) => (0.0, 0.3, 2.4),
                (quadruped_medium::Species::Llama, _) => (0.0, 0.1, 1.5),
                (quadruped_medium::Species::Alpaca, _) => (0.0, -0.1, 1.0),
                (quadruped_medium::Species::Akhlut, _) => (0.0, 1.9, 2.6),
                (quadruped_medium::Species::Bristleback, _) => (0.0, -0.4, 1.3),
                (quadruped_medium::Species::ClaySteed, _) => (0.0, -0.3, 2.8),
            },
            Body::BirdMedium(b) => match (b.species, b.body_type) {
                (bird_medium::Species::SnowyOwl, _) => (0.0, -0.25, 0.52),
                (bird_medium::Species::HornedOwl, _) => (0.0, -0.25, 0.45),
                (bird_medium::Species::Duck, _) => (0.0, -0.18, 0.47),
                (bird_medium::Species::Cockatiel, _) => (0.0, -0.05, 0.44),
                (bird_medium::Species::Chicken, bird_medium::BodyType::Female) => {
                    (0.0, -0.13, 0.62)
                },
                (bird_medium::Species::Chicken, bird_medium::BodyType::Male) => (0.0, -0.1, 0.62),
                (bird_medium::Species::Bat, _) => (0.0, 0.02, 0.47),
                (bird_medium::Species::Penguin, _) => (0.0, -0.15, 0.88),
                (bird_medium::Species::Goose, _) => (0.0, -0.04, 0.74),
                (bird_medium::Species::Peacock, _) => (0.0, -0.18, 0.77),
                (bird_medium::Species::Eagle, _) => (0.0, -0.26, 0.64),
                (bird_medium::Species::Parrot, _) => (0.0, -0.18, 0.52),
                (bird_medium::Species::Crow, _) => (0.0, -0.08, 0.5),
                (bird_medium::Species::Dodo, _) => (0.0, -0.25, 0.7),
                (bird_medium::Species::Parakeet, _) => (0.0, -0.08, 0.42),
                (bird_medium::Species::Puffin, _) => (0.0, -0.21, 0.56),
                (bird_medium::Species::Toucan, _) => (0.0, -0.12, 0.52),
                (bird_medium::Species::BloodmoonBat, _) => (0.0, 0.1, 1.6),
                (bird_medium::Species::VampireBat, _) => (0.0, 0.02, 0.5),
            },
            Body::FishMedium(b) => match (b.species, b.body_type) {
                (fish_medium::Species::Marlin, _) => (0.0, 0.26, 0.6),
                (fish_medium::Species::Icepike, _) => (0.0, 0.34, 0.65),
            },
            Body::Dragon(b) => match (b.species, b.body_type) {
                (dragon::Species::Reddragon, _) => (0.0, 0.5, 20.5),
            },
            Body::BirdLarge(b) => match (b.species, b.body_type) {
                (bird_large::Species::Phoenix, _) => (0.0, 0.4, 3.1),
                (bird_large::Species::Cockatrice, _) => (0.0, 0.94, 2.98),
                (bird_large::Species::Roc, _) => (0.0, 2.18, 4.23),
                (bird_large::Species::FlameWyvern, _) => (0.0, 1.58, 3.28),
                (bird_large::Species::CloudWyvern, _) => (0.0, 1.7, 3.15),
                (bird_large::Species::FrostWyvern, _) => (0.0, 1.8, 3.09),
                (bird_large::Species::SeaWyvern, _) => (0.0, 1.41, 3.31),
                (bird_large::Species::WealdWyvern, _) => (0.0, 1.8, 3.46),
            },
            Body::FishSmall(b) => match (b.species, b.body_type) {
                (fish_small::Species::Clownfish, _) => (0.0, 0.05, 0.6),
                (fish_small::Species::Piranha, _) => (0.0, -0.065, 0.715),
            },
            Body::BipedLarge(b) => match (b.species, b.body_type) {
                (biped_large::Species::Ogre, biped_large::BodyType::Female) => (1.2, 0.2, 4.2),
                (biped_large::Species::Ogre, biped_large::BodyType::Male) => (1.5, -0.25, 4.5),
                (biped_large::Species::Cyclops, _) => (3.0, 1.1, 6.6),
                (biped_large::Species::Wendigo, _) => (1.25, -0.2, 4.3),
                (biped_large::Species::Cavetroll, _) => (1.8, 0.1, 4.1),
                (biped_large::Species::Mountaintroll, _) => (1.9, 0.5, 4.2),
                (biped_large::Species::Swamptroll, _) => (1.9, 0.0, 4.3),
                (biped_large::Species::Dullahan, _) => (0.0, -0.2, 4.5),
                (biped_large::Species::Werewolf, _) => (1.0, 1.0, 2.8),
                (biped_large::Species::Occultsaurok, _) => (0.0, 0.4, 3.4),
                (biped_large::Species::Mightysaurok, _) => (0.0, 0.4, 3.4),
                (biped_large::Species::Slysaurok, _) => (0.0, 0.4, 3.4),
                (biped_large::Species::Mindflayer, _) => (1.8, 0.6, 6.6),
                (biped_large::Species::Minotaur, _) => (2.1, 0.4, 6.6),
                (biped_large::Species::Tidalwarrior, _) => (0.0, -0.5, 7.4),
                (biped_large::Species::Yeti, _) => (1.6, 0.5, 3.7),
                (biped_large::Species::Harvester, _) => (1.3, 0.4, 2.6),
                (biped_large::Species::Blueoni, _) => (1.7, 0.4, 3.6),
                (biped_large::Species::Redoni, _) => (1.7, 0.4, 3.6),
                (biped_large::Species::Cultistwarlord, _) => (1.1, 0.3, 2.7),
                (biped_large::Species::Cultistwarlock, _) => (1.0, 0.3, 2.7),
                (biped_large::Species::Huskbrute, _) => (1.6, 0.2, 3.8),
                (biped_large::Species::Tursus, _) => (1.6, 0.5, 3.4),
                (biped_large::Species::Gigasfrost, _) => (2.5, 0.5, 7.3),
                (biped_large::Species::Gigasfire, _) => (2.5, 0.5, 7.3),
                (biped_large::Species::AdletElder, _) => (1.2, 0.6, 2.4),
                (biped_large::Species::SeaBishop, _) => (0.9, 0.2, 2.0),
                (biped_large::Species::HaniwaGeneral, _) => (1.2, 0.4, 2.4),
                (biped_large::Species::TerracottaBesieger, _) => (1.5, -0.2, 3.3),
                (biped_large::Species::TerracottaDemolisher, _) => (1.1, -0.1, 2.2),
                (biped_large::Species::TerracottaPunisher, _) => (1.1, -0.1, 2.1),
                (biped_large::Species::TerracottaPursuer, _) => (1.1, -0.1, 2.2),
                (biped_large::Species::Cursekeeper, _) => (1.3, -0.4, 3.0),
                (biped_large::Species::Forgemaster, _) => (2.6, 0.8, 6.1),
                (biped_large::Species::Strigoi, _) => (1.7, -0.1, 3.6),
                (biped_large::Species::Executioner, _) => (1.1, 0.1, 3.3),
            },
            Body::BipedSmall(b) => match (b.species, b.body_type) {
                (biped_small::Species::Gnome, _) => (0.0, -0.33, 1.22),
                (biped_small::Species::Sahagin, _) => (0.0, 0.0, 1.9),
                (biped_small::Species::Adlet, _) => (0.0, -0.35, 1.7),
                (biped_small::Species::Gnarling, _) => (0.0, -0.3, 1.07),
                (biped_small::Species::Mandragora, _) => (0.0, -0.25, 0.72),
                (biped_small::Species::Kappa, _) => (0.0, -0.34, 1.1),
                (biped_small::Species::Cactid, _) => (0.0, -0.25, 1.13),
                (biped_small::Species::Gnoll, _) => (0.0, -0.28, 1.25),
                (biped_small::Species::Haniwa, _) => (0.0, -0.6, 1.91),
                (biped_small::Species::Myrmidon, _) => (0.0, -0.89, 0.96),
                (biped_small::Species::Husk, _) => (0.0, -0.5, 2.32),
                (biped_small::Species::Boreal, _) => (0.0, -0.81, 4.4),
                (biped_small::Species::Ashen, _) => (0.0, -0.81, 4.4),
                (biped_small::Species::Bushly, _) => (0.0, -0.26, 1.81),
                (biped_small::Species::Irrwurz, _) => (0.0, -0.35, 1.42),
                (biped_small::Species::IronDwarf, _) => (0.0, -0.12, 2.98),
                (biped_small::Species::Flamekeeper, _) => (0.0, -0.31, 2.2),
                (biped_small::Species::ShamanicSpirit, _) => (0.0, -0.03, 2.29),
                (biped_small::Species::Jiangshi, _) => (0.0, -0.21, 2.53),
                (biped_small::Species::TreasureEgg, _) => (0.0, -0.31, 1.0),
                (biped_small::Species::GnarlingChieftain, _) => (0.0, -0.27, 1.28),
                (biped_small::Species::BloodmoonHeiress, _) => (0.0, -0.12, 5.22),
                (biped_small::Species::Bloodservant, _) => (0.0, -0.21, 2.26),
                (biped_small::Species::Harlequin, _) => (0.0, -0.26, 2.17),
                (biped_small::Species::GoblinThug, _) => (0.0, -0.37, 0.49),
                (biped_small::Species::GoblinChucker, _) => (0.0, -0.83, 0.49),
                (biped_small::Species::GoblinRuffian, _) => (0.0, -0.37, 0.49),
                (biped_small::Species::GreenLegoom, _) => (0.0, -0.4, 1.48),
                (biped_small::Species::OchreLegoom, _) => (0.0, -0.4, 1.2),
                (biped_small::Species::PurpleLegoom, _) => (0.0, -0.36, 1.66),
                (biped_small::Species::RedLegoom, _) => (0.0, -0.3, 1.4),
            },
            Body::Golem(b) => match (b.species, b.body_type) {
                (golem::Species::StoneGolem, _) => (0.0, 0.2, 8.8),
                (golem::Species::Treant, _) => (0.0, 3.6, 6.4),
                (golem::Species::ClayGolem, _) => (0.0, 0.0, 7.8),
                (golem::Species::WoodGolem, _) => (0.0, 0.5, 7.2),
                (golem::Species::CoralGolem, _) => (0.0, 0.0, 4.0),
                (golem::Species::Gravewarden, _) => (0.0, -0.5, 7.5),
                (golem::Species::AncientEffigy, _) => (0.0, -0.2, 3.5),
                (golem::Species::Mogwai, _) => (0.0, 0.4, 3.7),
                (golem::Species::IronGolem, _) => (0.0, 0.2, 10.2),
            },
            Body::Theropod(b) => match (b.species, b.body_type) {
                (theropod::Species::Archaeos, _) => (0.0, 2.2, 6.1),
                (theropod::Species::Odonto, _) => (0.0, 4.1, 4.0),
                (theropod::Species::Sandraptor, _) => (0.0, -0.15, 1.63),
                (theropod::Species::Snowraptor, _) => (0.0, -0.15, 1.58),
                (theropod::Species::Woodraptor, _) => (0.0, -0.14, 1.66),
                (theropod::Species::Sunlizard, _) => (0.0, -0.2, 1.75),
                (theropod::Species::Yale, _) => (0.0, -0.615, 2.75),
                (theropod::Species::Ntouka, _) => (0.0, -1.1, 5.5),
                (theropod::Species::Dodarock, _) => (0.0, -0.09, 1.49),
                (theropod::Species::Axebeak, _) => (0.0, -0.34, 1.85),
            },
            Body::QuadrupedLow(b) => match (b.species, b.body_type) {
                (quadruped_low::Species::Crocodile, _) => (0.0, 0.35, 0.8),
                (quadruped_low::Species::Alligator, _) => (0.0, 0.27, 0.8),
                (quadruped_low::Species::Salamander, _) => (0.0, 0.21, 0.9),
                (quadruped_low::Species::Monitor, _) => (0.0, 0.01, 0.64),
                (quadruped_low::Species::Asp, _) => (0.0, 0.21, 1.2),
                (quadruped_low::Species::Tortoise, _) => (0.0, 0.01, 1.37),
                (quadruped_low::Species::Pangolin, _) => (0.0, -0.08, 1.08),
                (quadruped_low::Species::Maneater, _) => (0.0, 1.3, 2.0),
                (quadruped_low::Species::Sandshark, _) => (0.0, -0.48, 1.62),
                (quadruped_low::Species::Hakulaq, _) => (0.0, 0.49, 1.51),
                (quadruped_low::Species::Lavadrake, _) => (0.0, 0.22, 1.61),
                (quadruped_low::Species::Basilisk, _) => (0.0, -0.17, 2.71),
                (quadruped_low::Species::Deadwood, _) => (0.0, -0.15, 1.21),
                (quadruped_low::Species::Icedrake, _) => (0.0, -0.7, 2.28),
                (quadruped_low::Species::SeaCrocodile, _) => (0.1, 0.35, 1.03),
                (quadruped_low::Species::Dagon, _) => (0.1, 0.92, 1.32),
                (quadruped_low::Species::Rocksnapper, _) => (0.0, 0.72, 2.73),
                (quadruped_low::Species::Rootsnapper, _) => (0.0, -0.18, 3.13),
                (quadruped_low::Species::Reefsnapper, _) => (0.0, 0.22, 2.32),
                (quadruped_low::Species::Elbst, _) => (0.0, 0.22, 0.8),
                (quadruped_low::Species::Mossdrake, _) => (0.0, 0.22, 2.02),
                (quadruped_low::Species::Driggle, _) => (0.0, 0.5, 0.8),
                (quadruped_low::Species::Snaretongue, _) => (0.0, -0.54, 1.35),
                (quadruped_low::Species::Hydra, _) => (0.0, 0.28, 2.71),
            },
            Body::Arthropod(b) => match (b.species, b.body_type) {
                (arthropod::Species::Tarantula, _) => (0.0, -0.65, 1.66),
                (arthropod::Species::Blackwidow, _) => (0.0, -0.82, 2.48),
                (arthropod::Species::Antlion, _) => (0.0, -0.49, 1.98),
                (arthropod::Species::Hornbeetle, _) => (0.0, 0.27, 1.98),
                (arthropod::Species::Leafbeetle, _) => (0.0, -0.06, 1.1),
                (arthropod::Species::Stagbeetle, _) => (0.0, 0.6, 1.48),
                (arthropod::Species::Weevil, _) => (0.0, -0.1, 1.02),
                (arthropod::Species::Cavespider, _) => (0.0, -0.65, 1.82),
                (arthropod::Species::Moltencrawler, _) => (0.0, 0.43, 1.82),
                (arthropod::Species::Mosscrawler, _) => (0.0, 0.6, 1.82),
                (arthropod::Species::Sandcrawler, _) => (0.0, 0.6, 1.82),
                (arthropod::Species::Dagonite, _) => (0.0, 1.1, 2.15),
                (arthropod::Species::Emberfly, _) => (0.0, -0.28, 0.36),
            },
            Body::Crustacean(b) => match (b.species, b.body_type) {
                (crustacean::Species::Crab, _) => (0.0, -0.22, 0.36),
                (crustacean::Species::SoldierCrab, _) => (0.0, -0.14, 0.5),
                (crustacean::Species::Karkatha, _) => (0.0, -0.23, 7.66),
            },
            Body::Ship(ship) => match ship {
                ship::Body::DefaultAirship => (0.0, 0.0, 10.0),
                ship::Body::AirBalloon => (0.0, 0.0, 5.0),
                ship::Body::SailBoat => (-2.0, -5.0, 4.0),
                ship::Body::Galleon => (-2.0, -5.0, 4.0),
                ship::Body::Skiff => (1.0, -2.0, 2.0),
                ship::Body::Submarine => (1.0, -2.0, 2.0),
                ship::Body::Carriage => (1.0, -2.0, 2.0),
                ship::Body::Cart => (1.0, -2.0, 2.0),
                ship::Body::Volume => (0.0, 0.0, 0.0),
                ship::Body::Train => (1.0, -2.0, 2.0),
            },
            _ => (0.0, 0.0, 0.0),
        }
        .into()
    }

    /// Component of the mounting offset specific to the rider
    pub fn rider_offset(&self) -> Vec3<f32> {
        match self {
            Body::Humanoid(_) => [0.0, 0.0, 0.0],
            _ => [0.0, 0.0, 0.0],
        }
        .into()
    }

    pub fn tether_offset_leader(&self) -> Vec3<f32> {
        Vec3::new(0.0, self.dimensions().y * -0.4, self.dimensions().z * 0.7)
    }

    pub fn tether_offset_follower(&self) -> Vec3<f32> {
        Vec3::new(0.0, self.dimensions().y * 0.6, self.dimensions().z * 0.7)
    }

    /// Should be only used with npc-tell_monster.
    ///
    /// If you want to use for displaying names in HUD, add new strings.
    /// If you want to use for anything else, add new strings.
    pub fn localize_npc(&self) -> Content {
        fn try_localize(body: &Body) -> Option<Content> {
            match body {
                Body::BipedLarge(biped_large) => biped_large.localize_npc(),
                _ => None,
            }
        }

        try_localize(self).unwrap_or_else(|| Content::localized("body-npc-speech-generic"))
    }

    /// Read comment on `Gender` for more
    pub fn humanoid_gender(&self) -> Option<Gender> {
        match self {
            Body::Humanoid(b) => match b.body_type {
                humanoid::BodyType::Male => Some(Gender::Masculine),
                humanoid::BodyType::Female => Some(Gender::Feminine),
            },
            _ => None,
        }
    }

    /// Return gender information for this entity.
    ///
    /// It's an imprecise approximation, because body type != gender, and
    /// we need more advanced scheme here, but that's all we have atm.
    ///
    /// At the moment used for two things:
    /// - Grammatical gender indicator regarding players for proper grammatical
    ///   agreement in sentences for languages that require it. At the moment
    ///   can be used only for chat messages, but should be extended further.
    /// - Semantic indicator to pick proper name variant for NPC. For example,
    ///   Hunter vs Huntress or Lion vs Lioness.
    pub fn default_gender(&self) -> Gender {
        match self {
            Body::Humanoid(b) => match b.body_type {
                humanoid::BodyType::Male => Gender::Masculine,
                humanoid::BodyType::Female => Gender::Feminine,
            },
            Body::QuadrupedSmall(b) => match b.body_type {
                quadruped_small::BodyType::Male => Gender::Masculine,
                quadruped_small::BodyType::Female => Gender::Feminine,
            },
            Body::QuadrupedMedium(b) => match b.body_type {
                quadruped_medium::BodyType::Male => Gender::Masculine,
                quadruped_medium::BodyType::Female => Gender::Feminine,
            },
            Body::QuadrupedLow(b) => match b.body_type {
                quadruped_low::BodyType::Male => Gender::Masculine,
                quadruped_low::BodyType::Female => Gender::Feminine,
            },
            Body::BirdMedium(b) => match b.body_type {
                bird_medium::BodyType::Male => Gender::Masculine,
                bird_medium::BodyType::Female => Gender::Feminine,
            },
            Body::BirdLarge(b) => match b.body_type {
                bird_large::BodyType::Male => Gender::Masculine,
                bird_large::BodyType::Female => Gender::Feminine,
            },
            Body::FishMedium(b) => match b.body_type {
                fish_medium::BodyType::Male => Gender::Masculine,
                fish_medium::BodyType::Female => Gender::Feminine,
            },
            Body::FishSmall(b) => match b.body_type {
                fish_small::BodyType::Male => Gender::Masculine,
                fish_small::BodyType::Female => Gender::Feminine,
            },
            Body::Dragon(b) => match b.body_type {
                dragon::BodyType::Male => Gender::Masculine,
                dragon::BodyType::Female => Gender::Feminine,
            },
            Body::BipedLarge(b) => match b.body_type {
                biped_large::BodyType::Male => Gender::Masculine,
                biped_large::BodyType::Female => Gender::Feminine,
            },
            Body::BipedSmall(b) => match b.body_type {
                biped_small::BodyType::Male => Gender::Masculine,
                biped_small::BodyType::Female => Gender::Feminine,
            },
            Body::Golem(b) => match b.body_type {
                golem::BodyType::Male => Gender::Masculine,
                golem::BodyType::Female => Gender::Feminine,
            },
            Body::Theropod(b) => match b.body_type {
                theropod::BodyType::Male => Gender::Masculine,
                theropod::BodyType::Female => Gender::Feminine,
            },
            Body::Arthropod(b) => match b.body_type {
                arthropod::BodyType::Male => Gender::Masculine,
                arthropod::BodyType::Female => Gender::Feminine,
            },
            Body::Crustacean(b) => match b.body_type {
                crustacean::BodyType::Male => Gender::Masculine,
                crustacean::BodyType::Female => Gender::Feminine,
            },
            // TODO: do smth about it
            Body::Plugin(_) => Gender::Neuter,
            Body::Object(_) | Body::Ship(_) | Body::Item(_) => Gender::Neuter,
        }
    }

    /// For use with NPC name localization.
    ///
    /// Not a grammatical gender, keep that in mind.
    pub fn gender_attr(&self) -> &'static str {
        match self.default_gender() {
            Gender::Feminine => "fem",
            Gender::Masculine => "masc",
            Gender::Neuter => "neut",
        }
    }
}

impl Component for Body {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}
