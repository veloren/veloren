use crate::{make_case_elim, make_proj_elim};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

make_proj_elim!(
    body,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub struct Body {
        pub species: Species,
        pub body_type: BodyType,
    }
);

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        let species = *ALL_SPECIES.choose(&mut rng).unwrap();
        Self::random_with(&mut rng, &species)
    }

    #[inline]
    pub fn random_with(rng: &mut impl rand::Rng, &species: &Species) -> Self {
        let body_type = *ALL_BODY_TYPES.choose(rng).unwrap();
        Self { species, body_type }
    }
}

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::QuadrupedLow(body) }
}

// Renaming any enum entries here (re-ordering is fine) will require a
// database migration to ensure pets correctly de-serialize on player login.
make_case_elim!(
    species,
    #[derive(
        Copy,
        Clone,
        Debug,
        Display,
        EnumString,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize,
    )]
    #[repr(u32)]
    pub enum Species {
        Crocodile = 0,
        Alligator = 1,
        Salamander = 2,
        Monitor = 3,
        Asp = 4,
        Tortoise = 5,
        Pangolin = 6,
        Maneater = 7,
        Sandshark = 8,
        Hakulaq = 9,
        Lavadrake = 10,
        Basilisk = 11,
        Deadwood = 12,
        Icedrake = 13,
        SeaCrocodile = 14,
        Dagon = 15,
        Rocksnapper = 16,
        Rootsnapper = 17,
        Reefsnapper = 18,
        Elbst = 19,
        Mossdrake = 20,
    }
);

/// Data representing per-species generic data.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub crocodile: SpeciesMeta,
    pub sea_crocodile: SpeciesMeta,
    pub alligator: SpeciesMeta,
    pub salamander: SpeciesMeta,
    pub elbst: SpeciesMeta,
    pub monitor: SpeciesMeta,
    pub asp: SpeciesMeta,
    pub tortoise: SpeciesMeta,
    pub rocksnapper: SpeciesMeta,
    pub rootsnapper: SpeciesMeta,
    pub reefsnapper: SpeciesMeta,
    pub pangolin: SpeciesMeta,
    pub maneater: SpeciesMeta,
    pub sandshark: SpeciesMeta,
    pub hakulaq: SpeciesMeta,
    pub dagon: SpeciesMeta,
    pub lavadrake: SpeciesMeta,
    pub basilisk: SpeciesMeta,
    pub deadwood: SpeciesMeta,
    pub icedrake: SpeciesMeta,
    pub mossdrake: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::Crocodile => &self.crocodile,
            Species::SeaCrocodile => &self.sea_crocodile,
            Species::Alligator => &self.alligator,
            Species::Salamander => &self.salamander,
            Species::Elbst => &self.elbst,
            Species::Monitor => &self.monitor,
            Species::Asp => &self.asp,
            Species::Tortoise => &self.tortoise,
            Species::Rocksnapper => &self.rocksnapper,
            Species::Rootsnapper => &self.rootsnapper,
            Species::Reefsnapper => &self.reefsnapper,
            Species::Pangolin => &self.pangolin,
            Species::Maneater => &self.maneater,
            Species::Sandshark => &self.sandshark,
            Species::Hakulaq => &self.hakulaq,
            Species::Dagon => &self.dagon,
            Species::Lavadrake => &self.lavadrake,
            Species::Basilisk => &self.basilisk,
            Species::Deadwood => &self.deadwood,
            Species::Icedrake => &self.icedrake,
            Species::Mossdrake => &self.mossdrake,
        }
    }
}

pub const ALL_SPECIES: [Species; 21] = [
    Species::Crocodile,
    Species::SeaCrocodile,
    Species::Alligator,
    Species::Salamander,
    Species::Elbst,
    Species::Monitor,
    Species::Asp,
    Species::Tortoise,
    Species::Rocksnapper,
    Species::Rootsnapper,
    Species::Reefsnapper,
    Species::Pangolin,
    Species::Maneater,
    Species::Sandshark,
    Species::Hakulaq,
    Species::Dagon,
    Species::Lavadrake,
    Species::Basilisk,
    Species::Deadwood,
    Species::Icedrake,
    Species::Mossdrake,
];

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

// Renaming any enum entries here (re-ordering is fine) will require a
// database migration to ensure pets correctly de-serialize on player login.
make_case_elim!(
    body_type,
    #[derive(
        Copy,
        Clone,
        Debug,
        Display,
        EnumString,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Serialize,
        Deserialize,
    )]
    #[repr(u32)]
    pub enum BodyType {
        Female = 0,
        Male = 1,
    }
);
pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
