use common_base::{enum_iter, struct_iter};
use rand::{seq::SliceRandom, thread_rng};
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
    fn from(body: Body) -> Self { super::Body::QuadrupedSmall(body) }
}

// Renaming any enum entries here (re-ordering is fine) will require a
// database migration to ensure pets correctly de-serialize on player login.
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
        Goat = 10,
        Holladon = 11,
        Hyena = 12,
        Rabbit = 13,
        Truffler = 14,
        Frog = 15,
        Rat = 16,
        Axolotl = 17,
        Gecko = 18,
        Turtle = 19,
        Squirrel = 20,
        Fungome = 21,
        Porcupine = 22,
        Beaver = 23,
        Hare = 24,
        Dog = 25,
        Seal = 26,
        TreantSapling = 27,
        MossySnail = 28,
    }
}

/// Data representing per-species generic data.
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    pub holladon: SpeciesMeta,
    pub hyena: SpeciesMeta,
    pub rabbit: SpeciesMeta,
    pub truffler: SpeciesMeta,
    pub frog: SpeciesMeta,
    pub rat: SpeciesMeta,
    pub axolotl: SpeciesMeta,
    pub gecko: SpeciesMeta,
    pub turtle: SpeciesMeta,
    pub squirrel: SpeciesMeta,
    pub fungome: SpeciesMeta,
    pub porcupine: SpeciesMeta,
    pub beaver: SpeciesMeta,
    pub hare: SpeciesMeta,
    pub dog: SpeciesMeta,
    pub goat: SpeciesMeta,
    pub seal: SpeciesMeta,
    pub treant_sapling: SpeciesMeta,
    pub mossy_snail: SpeciesMeta,
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
            Species::Holladon => &self.holladon,
            Species::Hyena => &self.hyena,
            Species::Rabbit => &self.rabbit,
            Species::Truffler => &self.truffler,
            Species::Frog => &self.frog,
            Species::Rat => &self.rat,
            Species::Axolotl => &self.axolotl,
            Species::Gecko => &self.gecko,
            Species::Turtle => &self.turtle,
            Species::Squirrel => &self.squirrel,
            Species::Fungome => &self.fungome,
            Species::Porcupine => &self.porcupine,
            Species::Beaver => &self.beaver,
            Species::Hare => &self.hare,
            Species::Dog => &self.dog,
            Species::Goat => &self.goat,
            Species::Seal => &self.seal,
            Species::TreantSapling => &self.treant_sapling,
            Species::MossySnail => &self.mossy_snail,
        }
    }
}

pub const ALL_SPECIES: [Species; Species::NUM_KINDS] = Species::ALL;

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

// Renaming any enum entries here (re-ordering is fine) will require a
// database migration to ensure pets correctly de-serialize on player login.
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
