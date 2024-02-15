use crate::{make_case_elim, make_proj_elim};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

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
    fn from(body: Body) -> Self { super::Body::Golem(body) }
}

make_case_elim!(
    species,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Species {
        StoneGolem = 0,
        Treant = 1,
        ClayGolem = 2,
        WoodGolem = 3,
        CoralGolem = 4,
        Gravewarden = 5,
        AncientEffigy = 6,
        Mogwai = 7,
    }
);

/// Data representing per-species generic data.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub stonegolem: SpeciesMeta,
    pub treant: SpeciesMeta,
    pub claygolem: SpeciesMeta,
    pub woodgolem: SpeciesMeta,
    pub coralgolem: SpeciesMeta,
    pub gravewarden: SpeciesMeta,
    pub ancienteffigy: SpeciesMeta,
    pub mogwai: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::StoneGolem => &self.stonegolem,
            Species::Treant => &self.treant,
            Species::ClayGolem => &self.claygolem,
            Species::WoodGolem => &self.woodgolem,
            Species::CoralGolem => &self.coralgolem,
            Species::Gravewarden => &self.gravewarden,
            Species::AncientEffigy => &self.ancienteffigy,
            Species::Mogwai => &self.mogwai,
        }
    }
}

pub const ALL_SPECIES: [Species; 8] = [
    Species::StoneGolem,
    Species::Treant,
    Species::ClayGolem,
    Species::WoodGolem,
    Species::CoralGolem,
    Species::Gravewarden,
    Species::AncientEffigy,
    Species::Mogwai,
];

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

make_case_elim!(
    body_type,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum BodyType {
        Female = 0,
        Male = 1,
    }
);

pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
