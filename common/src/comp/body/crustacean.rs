use common_base::struct_iter;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

struct_iter! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub struct Body {
        pub species: Species,
        pub body_type: BodyType,
    }
}

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
    fn from(body: Body) -> Self { super::Body::Crustacean(body) }
}

// Renaming any enum entries here (re-ordering is fine) will require a
// database migration to ensure pets correctly de-serialize on player login.
#[derive(
    Copy,
    Clone,
    Debug,
    Display,
    EnumIter,
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
    Crab = 0,
    SoldierCrab = 1,
    Karkatha = 2,
}

/// Data representing per-species generic data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub crab: SpeciesMeta,
    pub soldier_crab: SpeciesMeta,
    pub karkatha: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::Crab => &self.crab,
            Species::SoldierCrab => &self.soldier_crab,
            Species::Karkatha => &self.karkatha,
        }
    }
}

pub const ALL_SPECIES: [Species; 3] = [Species::Crab, Species::SoldierCrab, Species::Karkatha];

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

#[derive(
    Copy,
    Clone,
    Debug,
    Display,
    EnumIter,
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

pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
