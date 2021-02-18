use crate::{make_case_elim, make_proj_elim};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

make_proj_elim!(
    body,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Body {
        pub species: Species,
        pub body_type: BodyType,
    }
);

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
    fn from(body: Body) -> Self { super::Body::BipedLarge(body) }
}

make_case_elim!(
    species,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Species {
        Ogre = 0,
        Cyclops = 1,
        Wendigo = 2,
        Troll = 3,
        Dullahan = 4,
        Werewolf = 5,
        Occultsaurok = 6,
        Mightysaurok = 7,
        Slysaurok = 8,
        Mindflayer = 9,
        Minotaur = 10,
        Tidalwarrior = 11,
    }
);

/// Data representing per-species generic data.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub ogre: SpeciesMeta,
    pub cyclops: SpeciesMeta,
    pub wendigo: SpeciesMeta,
    pub troll: SpeciesMeta,
    pub dullahan: SpeciesMeta,
    pub werewolf: SpeciesMeta,
    pub saurok_occult: SpeciesMeta,
    pub saurok_mighty: SpeciesMeta,
    pub saurok_sly: SpeciesMeta,
    pub mindflayer: SpeciesMeta,
    pub minotaur: SpeciesMeta,
    pub tidalwarrior: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::Ogre => &self.ogre,
            Species::Cyclops => &self.cyclops,
            Species::Wendigo => &self.wendigo,
            Species::Troll => &self.troll,
            Species::Dullahan => &self.dullahan,
            Species::Werewolf => &self.werewolf,
            Species::Occultsaurok => &self.saurok_occult,
            Species::Mightysaurok => &self.saurok_mighty,
            Species::Slysaurok => &self.saurok_sly,
            Species::Mindflayer => &self.mindflayer,
            Species::Minotaur => &self.minotaur,
            Species::Tidalwarrior => &self.tidalwarrior,
        }
    }
}

pub const ALL_SPECIES: [Species; 12] = [
    Species::Ogre,
    Species::Cyclops,
    Species::Wendigo,
    Species::Troll,
    Species::Dullahan,
    Species::Werewolf,
    Species::Occultsaurok,
    Species::Mightysaurok,
    Species::Slysaurok,
    Species::Mindflayer,
    Species::Minotaur,
    Species::Tidalwarrior,
];

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

make_case_elim!(
    body_type,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum BodyType {
        Female = 0,
        Male = 1,
    }
);
pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
