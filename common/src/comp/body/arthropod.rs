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
    fn from(body: Body) -> Self { super::Body::Arthropod(body) }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Species {
    Tarantula = 0,
    Blackwidow = 1,
    Antlion = 2,
    Hornbeetle = 3,
    Leafbeetle = 4,
    Stagbeetle = 5,
    Weevil = 6,
    Cavespider = 7,
    Moltencrawler = 8,
    Mosscrawler = 9,
    Sandcrawler = 10,
}

/// Data representing per-species generic data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub tarantula: SpeciesMeta,
    pub black_widow: SpeciesMeta,
    pub antlion: SpeciesMeta,
    pub horn_beetle: SpeciesMeta,
    pub leaf_beetle: SpeciesMeta,
    pub stag_beetle: SpeciesMeta,
    pub weevil: SpeciesMeta,
    pub cave_spider: SpeciesMeta,
    pub crawler_molten: SpeciesMeta,
    pub crawler_moss: SpeciesMeta,
    pub crawler_sand: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::Tarantula => &self.tarantula,
            Species::Blackwidow => &self.black_widow,
            Species::Antlion => &self.antlion,
            Species::Hornbeetle => &self.horn_beetle,
            Species::Leafbeetle => &self.leaf_beetle,
            Species::Stagbeetle => &self.stag_beetle,
            Species::Weevil => &self.weevil,
            Species::Cavespider => &self.cave_spider,
            Species::Moltencrawler => &self.crawler_molten,
            Species::Mosscrawler => &self.crawler_moss,
            Species::Sandcrawler => &self.crawler_sand,
        }
    }
}

pub const ALL_SPECIES: [Species; 11] = [
    Species::Tarantula,
    Species::Blackwidow,
    Species::Antlion,
    Species::Hornbeetle,
    Species::Leafbeetle,
    Species::Stagbeetle,
    Species::Weevil,
    Species::Cavespider,
    Species::Moltencrawler,
    Species::Mosscrawler,
    Species::Sandcrawler,
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
