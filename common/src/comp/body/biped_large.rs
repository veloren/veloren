use common_base::{enum_iter, struct_iter};
use common_i18n::Content;
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
                BodyType::Male => "noun-ogre-male",
                BodyType::Female => "noun-ogre-female",
            },
            Species::Cyclops => "noun-cyclops",
            Species::Wendigo => "noun-wendigo",
            Species::Werewolf => "noun-werewolf",
            Species::Cavetroll => "noun-cave_troll",
            Species::Mountaintroll => "noun-mountain_troll",
            Species::Swamptroll => "noun-swamp_troll",
            Species::Blueoni => "noun-blue_oni",
            Species::Redoni => "noun-red_oni",
            Species::Tursus => "noun-tursus",
            Species::Dullahan => "noun-dullahan",
            Species::Occultsaurok => "noun-occult_saurok",
            Species::Mightysaurok => "noun-mighty_saurok",
            Species::Slysaurok => "noun-sly_saurok",
            Species::Mindflayer => "noun-mindflayer",
            Species::Minotaur => "noun-minotaur",
            Species::Tidalwarrior => "noun-tidal_warrior",
            Species::Yeti => "noun-yeti",
            Species::Harvester => "noun-harvester",
            Species::Cultistwarlord => "noun-cultist_warlord",
            Species::Cultistwarlock => "noun-cultist_warlock",
            Species::Huskbrute => "noun-husk_brute",
            Species::Gigasfrost => "noun-gigas_frost",
            Species::Gigasfire => "noun-gigas_fire",
            Species::AdletElder => "noun-adlet_elder",
            Species::SeaBishop => "noun-sea_bishop",
            Species::HaniwaGeneral => "noun-haniwa_general",
            Species::TerracottaBesieger => "noun-terracotta_besieger",
            Species::TerracottaDemolisher => "noun-terracotta_demolisher",
            Species::TerracottaPunisher => "noun-terracotta_punisher",
            Species::TerracottaPursuer => "noun-terracotta_pursuer",
            Species::Cursekeeper => "noun-cursekeeper",
            Species::Forgemaster => "noun-forgemaster",
            Species::Strigoi => "noun-strigoi",
            Species::Executioner => "noun-executioner",
        };

        Some(Content::localized(key))
    }
}

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::BipedLarge(body) }
}

enum_iter! {
    ~const_array(ALL)
    #[derive(
        Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
        Forgemaster = 31,
        Strigoi = 32,
        Executioner = 33,
        Gigasfire = 34,
    }
}

/// Data representing per-species generic data.
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    pub forgemaster: SpeciesMeta,
    pub strigoi: SpeciesMeta,
    pub executioner: SpeciesMeta,
    pub gigas_fire: SpeciesMeta,
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
            Species::Forgemaster => &self.forgemaster,
            Species::Strigoi => &self.strigoi,
            Species::Executioner => &self.executioner,
            Species::Gigasfire => &self.gigas_fire,
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
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EnumString, Display)]
    #[repr(u32)]
    pub enum BodyType {
        Female = 0,
        Male = 1,
    }
}
pub const ALL_BODY_TYPES: [BodyType; BodyType::NUM_KINDS] = BodyType::ALL;
