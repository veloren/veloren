use crate::{make_case_elim, make_proj_elim};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

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
    fn from(body: Body) -> Self { super::Body::QuadrupedMedium(body) }
}

// Renaming any enum entries here (re-ordering is fine) will require a
// database migration to ensure pets correctly de-serialize on player login.
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
    Grolgar = 0,
    Saber = 1,
    Tiger = 2,
    Tuskram = 3,
    Lion = 6,
    Tarasque = 7,
    Wolf = 8,
    Frostfang = 9,
    Mouflon = 10,
    Catoblepas = 11,
    Bonerattler = 12,
    Deer = 13,
    Hirdrasil = 14,
    Roshwalr = 15,
    Donkey = 16,
    Camel = 17,
    Zebra = 18,
    Antelope = 19,
    Kelpie = 20,
    Horse = 21,
    Barghest = 22,
    Cattle = 23,
    Darkhound = 24,
    Highland = 25,
    Yak = 26,
    Panda = 27,
    Bear = 28,
    Dreadhorn = 29,
    Moose = 30,
    Snowleopard = 31,
    Mammoth = 32,
    Ngoubou = 33,
    Llama = 34,
    Alpaca = 35,
    Akhlut = 36,
    Bristleback = 37,
}

/// Data representing per-species generic data.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub grolgar: SpeciesMeta,
    pub saber: SpeciesMeta,
    pub tiger: SpeciesMeta,
    pub tuskram: SpeciesMeta,
    pub lion: SpeciesMeta,
    pub tarasque: SpeciesMeta,
    pub wolf: SpeciesMeta,
    pub frostfang: SpeciesMeta,
    pub mouflon: SpeciesMeta,
    pub catoblepas: SpeciesMeta,
    pub bonerattler: SpeciesMeta,
    pub deer: SpeciesMeta,
    pub hirdrasil: SpeciesMeta,
    pub roshwalr: SpeciesMeta,
    pub donkey: SpeciesMeta,
    pub camel: SpeciesMeta,
    pub zebra: SpeciesMeta,
    pub antelope: SpeciesMeta,
    pub kelpie: SpeciesMeta,
    pub horse: SpeciesMeta,
    pub barghest: SpeciesMeta,
    pub cattle: SpeciesMeta,
    pub darkhound: SpeciesMeta,
    pub highland: SpeciesMeta,
    pub yak: SpeciesMeta,
    pub panda: SpeciesMeta,
    pub bear: SpeciesMeta,
    pub dreadhorn: SpeciesMeta,
    pub moose: SpeciesMeta,
    pub snowleopard: SpeciesMeta,
    pub mammoth: SpeciesMeta,
    pub ngoubou: SpeciesMeta,
    pub llama: SpeciesMeta,
    pub alpaca: SpeciesMeta,
    pub akhlut: SpeciesMeta,
    pub bristleback: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::Grolgar => &self.grolgar,
            Species::Saber => &self.saber,
            Species::Tiger => &self.tiger,
            Species::Tuskram => &self.tuskram,
            Species::Lion => &self.lion,
            Species::Tarasque => &self.tarasque,
            Species::Wolf => &self.wolf,
            Species::Frostfang => &self.frostfang,
            Species::Mouflon => &self.mouflon,
            Species::Catoblepas => &self.catoblepas,
            Species::Bonerattler => &self.bonerattler,
            Species::Deer => &self.deer,
            Species::Hirdrasil => &self.hirdrasil,
            Species::Roshwalr => &self.roshwalr,
            Species::Donkey => &self.donkey,
            Species::Camel => &self.camel,
            Species::Zebra => &self.zebra,
            Species::Antelope => &self.antelope,
            Species::Kelpie => &self.kelpie,
            Species::Horse => &self.horse,
            Species::Barghest => &self.barghest,
            Species::Cattle => &self.cattle,
            Species::Darkhound => &self.darkhound,
            Species::Highland => &self.highland,
            Species::Yak => &self.yak,
            Species::Panda => &self.panda,
            Species::Bear => &self.bear,
            Species::Dreadhorn => &self.dreadhorn,
            Species::Moose => &self.moose,
            Species::Snowleopard => &self.snowleopard,
            Species::Mammoth => &self.mammoth,
            Species::Ngoubou => &self.ngoubou,
            Species::Llama => &self.llama,
            Species::Alpaca => &self.alpaca,
            Species::Akhlut => &self.akhlut,
            Species::Bristleback => &self.bristleback,
        }
    }
}

pub const ALL_SPECIES: [Species; 36] = [
    Species::Grolgar,
    Species::Saber,
    Species::Tiger,
    Species::Tuskram,
    Species::Lion,
    Species::Tarasque,
    Species::Wolf,
    Species::Frostfang,
    Species::Mouflon,
    Species::Catoblepas,
    Species::Bonerattler,
    Species::Deer,
    Species::Hirdrasil,
    Species::Roshwalr,
    Species::Donkey,
    Species::Camel,
    Species::Zebra,
    Species::Antelope,
    Species::Kelpie,
    Species::Horse,
    Species::Barghest,
    Species::Cattle,
    Species::Darkhound,
    Species::Highland,
    Species::Yak,
    Species::Panda,
    Species::Bear,
    Species::Dreadhorn,
    Species::Moose,
    Species::Snowleopard,
    Species::Mammoth,
    Species::Ngoubou,
    Species::Llama,
    Species::Alpaca,
    Species::Akhlut,
    Species::Bristleback,
];

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

// Renaming any enum entries here (re-ordering is fine) will require a
// database migration to ensure pets correctly de-serialize on player login.
make_case_elim!(
    body_type,
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
);

pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
