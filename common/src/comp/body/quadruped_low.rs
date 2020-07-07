use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Body {
    pub species: Species,
    pub body_type: BodyType,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        let species = *(&ALL_SPECIES).choose(&mut rng).unwrap();
        Self::random_with(&mut rng, &species)
    }

    #[inline]
    pub fn random_with(rng: &mut impl rand::Rng, &species: &Species) -> Self {
        let body_type = *(&ALL_BODY_TYPES).choose(rng).unwrap();
        Self { species, body_type }
    }
}

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::QuadrupedLow(body) }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Species {
    Crocodile = 0,
    Alligator = 1,
    Salamander = 2,
    Monitor = 3,
    Asp = 4,
    Tortoise = 5,
    Rocksnapper = 6,
    Pangolin = 7,
}

/// Data representing per-species generic data.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub crocodile: SpeciesMeta,
    pub alligator: SpeciesMeta,
    pub salamander: SpeciesMeta,
    pub monitor: SpeciesMeta,
    pub asp: SpeciesMeta,
    pub tortoise: SpeciesMeta,
    pub rocksnapper: SpeciesMeta,
    pub pangolin: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::Crocodile => &self.crocodile,
            Species::Alligator => &self.alligator,
            Species::Salamander => &self.salamander,
            Species::Monitor => &self.monitor,
            Species::Asp => &self.asp,
            Species::Tortoise => &self.tortoise,
            Species::Rocksnapper => &self.rocksnapper,
            Species::Pangolin => &self.pangolin,
        }
    }
}

pub const ALL_SPECIES: [Species; 8] = [
    Species::Crocodile,
    Species::Alligator,
    Species::Salamander,
    Species::Monitor,
    Species::Asp,
    Species::Tortoise,
    Species::Rocksnapper,
    Species::Pangolin,
];

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum BodyType {
    Female = 0,
    Male = 1,
}
pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
