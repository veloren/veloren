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
    fn from(body: Body) -> Self { super::Body::BipedLarge(body) }
}

make_case_elim!(
    species,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Species {
        Ogre = 0,
        Cyclops = 1,
        Wendigo = 2,
        Cavetroll = 3,
        Mountaintroll = 4,
        Swamptroll = 5,
        Dullahan = 6,
        Werewolf = 7,
        Occultsaurok = 8,
        Mightysaurok = 9,
        Slysaurok = 10,
        Mindflayer = 11,
        Minotaur = 12,
        Tidalwarrior = 13,
        Yeti = 14,
        Harvester = 15,
        Blueoni = 16,
        Redoni = 17,
        Cultistwarlord = 18,
        Cultistwarlock = 19,
        Huskbrute = 20,
        Tursus = 21,
        Gigasfrost = 22,
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
    pub troll_cave: SpeciesMeta,
    pub troll_mountain: SpeciesMeta,
    pub troll_swamp: SpeciesMeta,
    pub dullahan: SpeciesMeta,
    pub werewolf: SpeciesMeta,
    pub saurok_occult: SpeciesMeta,
    pub saurok_mighty: SpeciesMeta,
    pub saurok_sly: SpeciesMeta,
    pub mindflayer: SpeciesMeta,
    pub minotaur: SpeciesMeta,
    pub tidalwarrior: SpeciesMeta,
    pub yeti: SpeciesMeta,
    pub harvester: SpeciesMeta,
    pub oni_blue: SpeciesMeta,
    pub oni_red: SpeciesMeta,
    pub cultist_warlord: SpeciesMeta,
    pub cultist_warlock: SpeciesMeta,
    pub husk_brute: SpeciesMeta,
    pub tursus: SpeciesMeta,
    pub gigas_frost: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::Ogre => &self.ogre,
            Species::Cyclops => &self.cyclops,
            Species::Wendigo => &self.wendigo,
            Species::Cavetroll => &self.troll_cave,
            Species::Mountaintroll => &self.troll_mountain,
            Species::Swamptroll => &self.troll_swamp,
            Species::Dullahan => &self.dullahan,
            Species::Werewolf => &self.werewolf,
            Species::Occultsaurok => &self.saurok_occult,
            Species::Mightysaurok => &self.saurok_mighty,
            Species::Slysaurok => &self.saurok_sly,
            Species::Mindflayer => &self.mindflayer,
            Species::Minotaur => &self.minotaur,
            Species::Tidalwarrior => &self.tidalwarrior,
            Species::Yeti => &self.yeti,
            Species::Harvester => &self.harvester,
            Species::Blueoni => &self.oni_blue,
            Species::Redoni => &self.oni_red,
            Species::Cultistwarlord => &self.cultist_warlord,
            Species::Cultistwarlock => &self.cultist_warlock,
            Species::Huskbrute => &self.husk_brute,
            Species::Tursus => &self.tursus,
            Species::Gigasfrost => &self.gigas_frost,
        }
    }
}

pub const ALL_SPECIES: [Species; 23] = [
    Species::Ogre,
    Species::Cyclops,
    Species::Wendigo,
    Species::Cavetroll,
    Species::Mountaintroll,
    Species::Swamptroll,
    Species::Dullahan,
    Species::Werewolf,
    Species::Occultsaurok,
    Species::Mightysaurok,
    Species::Slysaurok,
    Species::Mindflayer,
    Species::Minotaur,
    Species::Tidalwarrior,
    Species::Yeti,
    Species::Harvester,
    Species::Blueoni,
    Species::Redoni,
    Species::Cultistwarlord,
    Species::Cultistwarlock,
    Species::Huskbrute,
    Species::Tursus,
    Species::Gigasfrost,
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
