use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Body {
    pub species: Species,
    pub body_type: BodyType,
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
    fn from(body: Body) -> Self { super::Body::Theropod(body) }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Species {
    Archaeos = 0,
    Odonto = 1,
    Sandraptor = 2,
    Snowraptor = 3,
    Woodraptor = 4,
    Sunlizard = 5,
    Yale = 6,
    Ntouka = 7,
    Dodarock = 8,
    Axebeak = 9,
}

/// Data representing per-species generic data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub archaeos: SpeciesMeta,
    pub odonto: SpeciesMeta,
    pub raptor_sand: SpeciesMeta,
    pub raptor_snow: SpeciesMeta,
    pub raptor_wood: SpeciesMeta,
    pub sunlizard: SpeciesMeta,
    pub yale: SpeciesMeta,
    pub dodarock: SpeciesMeta,
    pub ntouka: SpeciesMeta,
    pub axebeak: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::Archaeos => &self.archaeos,
            Species::Odonto => &self.odonto,
            Species::Sandraptor => &self.raptor_sand,
            Species::Snowraptor => &self.raptor_snow,
            Species::Woodraptor => &self.raptor_wood,
            Species::Sunlizard => &self.sunlizard,
            Species::Yale => &self.yale,
            Species::Dodarock => &self.dodarock,
            Species::Ntouka => &self.ntouka,
            Species::Axebeak => &self.axebeak,
        }
    }
}

pub const ALL_SPECIES: [Species; 10] = [
    Species::Archaeos,
    Species::Odonto,
    Species::Sandraptor,
    Species::Snowraptor,
    Species::Woodraptor,
    Species::Sunlizard,
    Species::Yale,
    Species::Dodarock,
    Species::Ntouka,
    Species::Axebeak,
];

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum BodyType {
    Female = 0,
    Male = 1,
}
pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
