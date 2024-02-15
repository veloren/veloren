use crate::{make_case_elim, make_proj_elim};
use common_i18n::Content;
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

    /// Should be only used with npc-tell_monster.
    ///
    /// If you want to use for displaying names in HUD, add new strings.
    /// If you want to use for anything else, add new strings.
    pub fn localize_npc(&self) -> Option<Content> {
        let key = match &self.species {
            Species::Ogre => match self.body_type {
                BodyType::Male => "body-npc-speech-biped_large-ogre-male",
                BodyType::Female => "body-npc-speech-biped_large-ogre-female",
            },
            Species::Cyclops => "body-npc-speech-biped_large-cyclops",
            Species::Wendigo => "body-npc-speech-biped_large-wendigo",
            Species::Werewolf => "body-npc-speech-biped_large-werewolf",
            Species::Cavetroll => "body-npc-speech-biped_large-cave_troll",
            Species::Mountaintroll => "body-npc-speech-biped_large-mountain_troll",
            Species::Swamptroll => "body-npc-speech-biped_large-swamp_troll",
            Species::Blueoni => "body-npc-speech-biped_large-blue_oni",
            Species::Redoni => "body-npc-speech-biped_large-red_oni",
            Species::Tursus => "body-npc-speech-biped_large-tursus",
            Species::Dullahan => "body-npc-speech-biped_large-dullahan",
            Species::Occultsaurok => "body-npc-speech-biped_large-occult_saurok",
            Species::Mightysaurok => "body-npc-speech-biped_large-mighty_saurok",
            Species::Slysaurok => "body-npc-speech-biped_large-sly_saurok",
            Species::Mindflayer => "body-npc-speech-biped_large-mindflayer",
            Species::Minotaur => "body-npc-speech-biped_large-minotaur",
            Species::Tidalwarrior => "body-npc-speech-biped_large-tidal_warrior",
            Species::Yeti => "body-npc-speech-biped_large-yeti",
            Species::Harvester => "body-npc-speech-biped_large-harvester",
            Species::Cultistwarlord => "body-npc-speech-biped_large-cultist_warlord",
            Species::Cultistwarlock => "body-npc-speech-biped_large-cultist_warlock",
            Species::Huskbrute => "body-npc-speech-biped_large-husk_brute",
            Species::Gigasfrost => "body-npc-speech-biped_large-gigas_frost",
            Species::AdletElder => "body-npc-speech-biped_large-adlet_elder",
            Species::SeaBishop => "body-npc-speech-biped_large-sea_bishop",
            Species::HaniwaGeneral => "body-npc-speech-biped_large-haniwa_general",
            Species::TerracottaBesieger => "body-npc-speech-biped_large-terracotta_besieger",
            Species::TerracottaDemolisher => "body-npc-speech-biped_large-terracotta_demolisher",
            Species::TerracottaPunisher => "body-npc-speech-biped_large-terracotta_punisher",
            Species::TerracottaPursuer => "body-npc-speech-biped_large-terracotta_pursuer",
            Species::Cursekeeper => "body-npc-speech-biped_large-cursekeeper",
        };

        Some(Content::localized(key))
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
        AdletElder = 23,
        SeaBishop = 24,
        HaniwaGeneral = 25,
        TerracottaBesieger = 26,
        TerracottaDemolisher = 27,
        TerracottaPunisher = 28,
        TerracottaPursuer = 29,
        Cursekeeper = 30,
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
    pub adlet_elder: SpeciesMeta,
    pub sea_bishop: SpeciesMeta,
    pub haniwa_general: SpeciesMeta,
    pub terracotta_besieger: SpeciesMeta,
    pub terracotta_demolisher: SpeciesMeta,
    pub terracotta_punisher: SpeciesMeta,
    pub terracotta_pursuer: SpeciesMeta,
    pub cursekeeper: SpeciesMeta,
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
            Species::AdletElder => &self.adlet_elder,
            Species::SeaBishop => &self.sea_bishop,
            Species::HaniwaGeneral => &self.haniwa_general,
            Species::TerracottaBesieger => &self.terracotta_besieger,
            Species::TerracottaDemolisher => &self.terracotta_demolisher,
            Species::TerracottaPunisher => &self.terracotta_punisher,
            Species::TerracottaPursuer => &self.terracotta_pursuer,
            Species::Cursekeeper => &self.cursekeeper,
        }
    }
}

pub const ALL_SPECIES: [Species; 31] = [
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
    Species::AdletElder,
    Species::SeaBishop,
    Species::HaniwaGeneral,
    Species::TerracottaBesieger,
    Species::TerracottaDemolisher,
    Species::TerracottaPunisher,
    Species::TerracottaPursuer,
    Species::Cursekeeper,
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
