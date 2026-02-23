use common_base::{enum_iter, struct_iter};
use rand::{prelude::IndexedRandom, rng};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

struct_iter! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub struct Body {
        pub species: Species,
        pub body_type: BodyType,
    }
}

impl Body {
    pub fn random() -> Self {
        let mut rng = rng();
        let species = *ALL_SPECIES.choose(&mut rng).unwrap();
        Self::random_with(&mut rng, &species)
    }

    #[inline]
    pub fn random_with(rng: &mut impl rand::RngExt, &species: &Species) -> Self {
        let body_type = *ALL_BODY_TYPES.choose(rng).unwrap();
        Self { species, body_type }
    }
}

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::BirdMedium(body) }
}

enum_iter! {
    ~const_array(ALL)
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
        SnowyOwl = 0,
        HornedOwl = 1,
        Duck = 2,
        Cockatiel = 3,
        Chicken = 4,
        Bat = 5,
        Penguin = 6,
        Goose = 7,
        Peacock = 8,
        Eagle = 9,
        Parrot = 10,
        Crow = 11,
        Dodo = 12,
        Parakeet = 13,
        Puffin = 14,
        Toucan = 15,
        BloodmoonBat = 16,
        VampireBat = 17,
    }
}

/// Data representing per-species generic data.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub snowy_owl: SpeciesMeta,
    pub horned_owl: SpeciesMeta,
    pub duck: SpeciesMeta,
    pub cockatiel: SpeciesMeta,
    pub chicken: SpeciesMeta,
    pub bat: SpeciesMeta,
    pub penguin: SpeciesMeta,
    pub goose: SpeciesMeta,
    pub peacock: SpeciesMeta,
    pub eagle: SpeciesMeta,
    pub parrot: SpeciesMeta,
    pub crow: SpeciesMeta,
    pub dodo: SpeciesMeta,
    pub parakeet: SpeciesMeta,
    pub puffin: SpeciesMeta,
    pub toucan: SpeciesMeta,
    pub bloodmoon_bat: SpeciesMeta,
    pub vampire_bat: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::SnowyOwl => &self.snowy_owl,
            Species::HornedOwl => &self.horned_owl,
            Species::Duck => &self.duck,
            Species::Cockatiel => &self.cockatiel,
            Species::Chicken => &self.chicken,
            Species::Bat => &self.bat,
            Species::Penguin => &self.penguin,
            Species::Goose => &self.goose,
            Species::Peacock => &self.peacock,
            Species::Eagle => &self.eagle,
            Species::Parrot => &self.parrot,
            Species::Crow => &self.crow,
            Species::Dodo => &self.dodo,
            Species::Parakeet => &self.parakeet,
            Species::Puffin => &self.puffin,
            Species::Toucan => &self.toucan,
            Species::BloodmoonBat => &self.bloodmoon_bat,
            Species::VampireBat => &self.vampire_bat,
        }
    }
}

pub const ALL_SPECIES: [Species; Species::NUM_KINDS] = Species::ALL;

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

enum_iter! {
    ~const_array(ALL)
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
}
pub const ALL_BODY_TYPES: [BodyType; BodyType::NUM_KINDS] = BodyType::ALL;
