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
    fn from(body: Body) -> Self { super::Body::QuadrupedSmall(body) }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Species {
    Pig = 0,
    Fox = 1,
    Sheep = 2,
    Boar = 3,
    Jackalope = 4,
    Skunk = 5,
    Cat = 6,
    Batfox = 7,
    Raccoon = 8,
    Quokka = 9,
    Dodarock = 10,
    Holladon = 11,
    Hyena = 12,
    Rabbit = 13,
    Truffler = 14,
}

/// Data representing per-species generic data.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub pig: SpeciesMeta,
    pub fox: SpeciesMeta,
    pub sheep: SpeciesMeta,
    pub boar: SpeciesMeta,
    pub jackalope: SpeciesMeta,
    pub skunk: SpeciesMeta,
    pub cat: SpeciesMeta,
    pub batfox: SpeciesMeta,
    pub raccoon: SpeciesMeta,
    pub quokka: SpeciesMeta,
    pub dodarock: SpeciesMeta,
    pub holladon: SpeciesMeta,
    pub hyena: SpeciesMeta,
    pub rabbit: SpeciesMeta,
    pub truffler: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::Pig => &self.pig,
            Species::Fox => &self.fox,
            Species::Sheep => &self.sheep,
            Species::Boar => &self.boar,
            Species::Jackalope => &self.jackalope,
            Species::Skunk => &self.skunk,
            Species::Cat => &self.cat,
            Species::Batfox => &self.batfox,
            Species::Raccoon => &self.raccoon,
            Species::Quokka => &self.quokka,
            Species::Dodarock => &self.dodarock,
            Species::Holladon => &self.holladon,
            Species::Hyena => &self.hyena,
            Species::Rabbit => &self.rabbit,
            Species::Truffler => &self.truffler,
        }
    }
}

pub const ALL_SPECIES: [Species; 15] = [
    Species::Pig,
    Species::Fox,
    Species::Sheep,
    Species::Boar,
    Species::Jackalope,
    Species::Skunk,
    Species::Cat,
    Species::Batfox,
    Species::Raccoon,
    Species::Quokka,
    Species::Dodarock,
    Species::Holladon,
    Species::Hyena,
    Species::Rabbit,
    Species::Truffler,
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
