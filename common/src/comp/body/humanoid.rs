use crate::{make_case_elim, make_proj_elim};
use rand::{seq::SliceRandom, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

make_proj_elim!(
    body,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Body {
        pub species: Species,
        pub body_type: BodyType,
        #[typed(pure)]
        pub hair_style: u8,
        #[typed(pure)]
        pub beard: u8,
        #[typed(pure)]
        pub eyes: u8,
        #[typed(pure)]
        pub accessory: u8,
        #[typed(pure)]
        pub hair_color: u8,
        #[typed(pure)]
        pub skin: u8,
        #[typed(pure)]
        pub eye_color: u8,
    }
);

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        let species = *(&ALL_SPECIES).choose(&mut rng).unwrap();
        Self::random_with(&mut rng, &species)
    }

    #[inline]
    pub fn random_with(rng: &mut impl Rng, &species: &Species) -> Self {
        let body_type = *(&ALL_BODY_TYPES).choose(rng).unwrap();
        Self {
            species,
            body_type,
            hair_style: rng.gen_range(0, species.num_hair_styles(body_type)),
            beard: rng.gen_range(0, species.num_beards(body_type)),
            accessory: rng.gen_range(0, species.num_accessories(body_type)),
            hair_color: rng.gen_range(0, species.num_hair_colors()),
            skin: rng.gen_range(0, species.num_skin_colors()),
            eye_color: rng.gen_range(0, species.num_eye_colors()),
            eyes: rng.gen_range(0, 1), /* TODO Add a way to set specific head-segments for NPCs
                                        * with the default being a random one */
        }
    }

    pub fn validate(&mut self) {
        self.hair_style = self
            .hair_style
            .min(self.species.num_hair_styles(self.body_type) - 1);
        self.beard = self.beard.min(self.species.num_beards(self.body_type) - 1);
        self.hair_color = self.hair_color.min(self.species.num_hair_colors() - 1);
        self.skin = self.skin.min(self.species.num_skin_colors() - 1);
        self.eyes = self.eyes.min(self.species.num_eyes(self.body_type) - 1);
        self.eye_color = self.eye_color.min(self.species.num_eye_colors() - 1);
        self.accessory = self
            .accessory
            .min(self.species.num_accessories(self.body_type) - 1);
    }

    /// Returns a scale value relative to the average humanoid
    pub fn scale(&self) -> f32 {
        use BodyType::*;
        use Species::*;
        match (self.species, self.body_type) {
            (Orc, Male) => 1.14,
            (Orc, Female) => 1.02,
            (Human, Male) => 1.02,
            (Human, Female) => 0.96,
            (Elf, Male) => 1.02,
            (Elf, Female) => 0.96,
            (Dwarf, Male) => 0.84,
            (Dwarf, Female) => 0.78,
            (Undead, Male) => 0.96,
            (Undead, Female) => 0.9,
            (Danari, Male) => 0.696,
            (Danari, Female) => 0.696,
        }
    }

    /// Returns the eye height for this humanoid.
    pub fn eye_height(&self) -> f32 { DEFAULT_HUMANOID_EYE_HEIGHT * self.scale() }
}

pub const DEFAULT_HUMANOID_EYE_HEIGHT: f32 = 1.65;

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::Humanoid(body) }
}

make_case_elim!(
    species,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Species {
        Danari = 0,
        Dwarf = 1,
        Elf = 2,
        Human = 3,
        Orc = 4,
        Undead = 5,
    }
);

/// Data representing per-species generic data.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub danari: SpeciesMeta,
    pub dwarf: SpeciesMeta,
    pub elf: SpeciesMeta,
    pub human: SpeciesMeta,
    pub orc: SpeciesMeta,
    pub undead: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::Danari => &self.danari,
            Species::Dwarf => &self.dwarf,
            Species::Elf => &self.elf,
            Species::Human => &self.human,
            Species::Orc => &self.orc,
            Species::Undead => &self.undead,
        }
    }
}

pub const ALL_SPECIES: [Species; 6] = [
    Species::Danari,
    Species::Dwarf,
    Species::Elf,
    Species::Human,
    Species::Orc,
    Species::Undead,
];

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

// Skin colors
pub const DANARI_SKIN_COLORS: [Skin; 4] = [
    Skin::DanariOne,
    Skin::DanariTwo,
    Skin::DanariThree,
    Skin::DanariFour,
];
pub const DWARF_SKIN_COLORS: [Skin; 14] = [
    Skin::Skin1,
    Skin::Skin2,
    Skin::Skin3,
    Skin::Skin4,
    Skin::Skin5,
    Skin::Skin6,
    Skin::Skin7,
    Skin::Skin8,
    Skin::Skin9,
    Skin::Skin10,
    Skin::Skin11,
    Skin::Skin12,
    Skin::Iron,
    Skin::Steel,
];
pub const ELF_SKIN_COLORS: [Skin; 13] = [
    Skin::Skin1,
    Skin::Skin2,
    Skin::Skin3,
    Skin::Skin5,
    Skin::Skin6,
    Skin::Skin7,
    Skin::Skin8,
    Skin::Skin9,
    Skin::Skin10,
    Skin::Skin11,
    Skin::Skin12,
    Skin::ElfOne,
    Skin::ElfTwo,
    //Skin::ElfThree,
];
pub const HUMAN_SKIN_COLORS: [Skin; 18] = [
    Skin::Skin1,
    Skin::Skin2,
    Skin::Skin3,
    Skin::Skin4,
    Skin::Skin5,
    Skin::Skin6,
    Skin::Skin7,
    Skin::Skin8,
    Skin::Skin9,
    Skin::Skin10,
    Skin::Skin11,
    Skin::Skin12,
    Skin::Skin13,
    Skin::Skin14,
    Skin::Skin15,
    Skin::Skin16,
    Skin::Skin17,
    Skin::Skin18,
];
pub const ORC_SKIN_COLORS: [Skin; 4] = [Skin::OrcOne, Skin::OrcTwo, Skin::OrcThree, Skin::OrcFour];
pub const UNDEAD_SKIN_COLORS: [Skin; 3] = [Skin::UndeadOne, Skin::UndeadTwo, Skin::UndeadThree];

// Eye colors
pub const DANARI_EYE_COLORS: [EyeColor; 3] = [
    EyeColor::CuriousGreen,
    EyeColor::LoyalBrown,
    EyeColor::ViciousRed,
];
pub const DWARF_EYE_COLORS: [EyeColor; 4] = [
    EyeColor::CuriousGreen,
    EyeColor::LoyalBrown,
    EyeColor::NobleBlue,
    EyeColor::CornflowerBlue,
];
pub const ELF_EYE_COLORS: [EyeColor; 4] = [
    EyeColor::NobleBlue,
    EyeColor::CornflowerBlue,
    EyeColor::CuriousGreen,
    EyeColor::LoyalBrown,
];
pub const HUMAN_EYE_COLORS: [EyeColor; 4] = [
    EyeColor::NobleBlue,
    EyeColor::CornflowerBlue,
    EyeColor::CuriousGreen,
    EyeColor::LoyalBrown,
];
pub const ORC_EYE_COLORS: [EyeColor; 5] = [
    EyeColor::LoyalBrown,
    EyeColor::ExoticPurple,
    EyeColor::AmberOrange,
    EyeColor::PineGreen,
    EyeColor::CornflowerBlue,
];
pub const UNDEAD_EYE_COLORS: [EyeColor; 5] = [
    EyeColor::ViciousRed,
    EyeColor::PumpkinOrange,
    EyeColor::GhastlyYellow,
    EyeColor::MagicPurple,
    EyeColor::ToxicGreen,
];

impl Species {
    fn skin_colors(self) -> &'static [Skin] {
        match self {
            Species::Danari => &DANARI_SKIN_COLORS,
            Species::Dwarf => &DWARF_SKIN_COLORS,
            Species::Elf => &ELF_SKIN_COLORS,
            Species::Human => &HUMAN_SKIN_COLORS,
            Species::Orc => &ORC_SKIN_COLORS,
            Species::Undead => &UNDEAD_SKIN_COLORS,
        }
    }

    fn eye_colors(self) -> &'static [EyeColor] {
        match self {
            Species::Danari => &DANARI_EYE_COLORS,
            Species::Dwarf => &DWARF_EYE_COLORS,
            Species::Elf => &ELF_EYE_COLORS,
            Species::Human => &HUMAN_EYE_COLORS,
            Species::Orc => &ORC_EYE_COLORS,
            Species::Undead => &UNDEAD_EYE_COLORS,
        }
    }

    /// FIXME: This is a hack!  The only reason we need to do this is because
    /// hair colors are currently just indices into an array, not enum
    /// variants.  Once we have proper variants for hair colors, we won't
    /// need to do this anymore, since we will use locally defined arrays to
    /// represent per-species stuff (or have some other solution for validity).
    pub fn num_hair_colors(self) -> u8 {
        match self {
            Species::Danari => 12,
            Species::Dwarf => 21,
            Species::Elf => 24,
            Species::Human => 22,
            Species::Orc => 11,
            Species::Undead => 22,
        }
    }

    pub fn skin_color(self, val: u8) -> Skin {
        self.skin_colors()
            .get(val as usize)
            .copied()
            .unwrap_or(Skin::Skin3)
    }

    pub fn num_skin_colors(self) -> u8 { self.skin_colors().len() as u8 }

    pub fn eye_color(self, val: u8) -> EyeColor {
        self.eye_colors()
            .get(val as usize)
            .copied()
            .unwrap_or(EyeColor::NobleBlue)
    }

    pub fn num_eye_colors(self) -> u8 { self.eye_colors().len() as u8 }

    pub fn num_hair_styles(self, body_type: BodyType) -> u8 {
        match (self, body_type) {
            (Species::Danari, BodyType::Female) => 4,
            (Species::Danari, BodyType::Male) => 4,
            (Species::Dwarf, BodyType::Female) => 8,
            (Species::Dwarf, BodyType::Male) => 4,
            (Species::Elf, BodyType::Female) => 21,
            (Species::Elf, BodyType::Male) => 4,
            (Species::Human, BodyType::Female) => 19,
            (Species::Human, BodyType::Male) => 17,
            (Species::Orc, BodyType::Female) => 7,
            (Species::Orc, BodyType::Male) => 8,
            (Species::Undead, BodyType::Female) => 6,
            (Species::Undead, BodyType::Male) => 5,
        }
    }

    pub fn num_accessories(self, body_type: BodyType) -> u8 {
        match (self, body_type) {
            (Species::Danari, BodyType::Female) => 1,
            (Species::Danari, BodyType::Male) => 1,
            (Species::Dwarf, BodyType::Female) => 7,
            (Species::Dwarf, BodyType::Male) => 7,
            (Species::Elf, BodyType::Female) => 2,
            (Species::Elf, BodyType::Male) => 1,
            (Species::Human, BodyType::Female) => 1,
            (Species::Human, BodyType::Male) => 1,
            (Species::Orc, BodyType::Female) => 4,
            (Species::Orc, BodyType::Male) => 6,
            (Species::Undead, BodyType::Female) => 2,
            (Species::Undead, BodyType::Male) => 2,
        }
    }

    pub fn num_eyebrows(self, _body_type: BodyType) -> u8 { 1 }

    pub fn num_eyes(self, body_type: BodyType) -> u8 {
        match (self, body_type) {
            (Species::Danari, BodyType::Female) => 6,
            (Species::Danari, BodyType::Male) => 8,
            (Species::Dwarf, BodyType::Female) => 6,
            (Species::Dwarf, BodyType::Male) => 8,
            (Species::Elf, BodyType::Female) => 6,
            (Species::Elf, BodyType::Male) => 7,
            (Species::Human, BodyType::Female) => 6,
            (Species::Human, BodyType::Male) => 7,
            (Species::Orc, BodyType::Female) => 6,
            (Species::Orc, BodyType::Male) => 2,
            (Species::Undead, BodyType::Female) => 3,
            (Species::Undead, BodyType::Male) => 4,
        }
    }

    pub fn num_beards(self, body_type: BodyType) -> u8 {
        match (self, body_type) {
            (Species::Danari, BodyType::Female) => 1,
            (Species::Danari, BodyType::Male) => 2,
            (Species::Dwarf, BodyType::Female) => 1,
            (Species::Dwarf, BodyType::Male) => 20,
            (Species::Elf, BodyType::Female) => 1,
            (Species::Elf, BodyType::Male) => 1,
            (Species::Human, BodyType::Female) => 1,
            (Species::Human, BodyType::Male) => 4,
            (Species::Orc, BodyType::Female) => 1,
            (Species::Orc, BodyType::Male) => 3,
            (Species::Undead, BodyType::Female) => 1,
            (Species::Undead, BodyType::Male) => 1,
        }
    }
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

make_case_elim!(
    eye_color,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
    #[repr(u32)]
    pub enum EyeColor {
        VigorousBlack = 0,
        NobleBlue = 1,
        CuriousGreen = 2,
        LoyalBrown = 3,
        ViciousRed = 4,
        PumpkinOrange = 5,
        GhastlyYellow = 6,
        MagicPurple = 7,
        ToxicGreen = 8,
        ExoticPurple = 9,
        SulfurYellow = 10,
        AmberOrange = 11,
        PineGreen = 12,
        CornflowerBlue = 13,
    }
);

make_case_elim!(
    skin,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
    #[repr(u32)]
    pub enum Skin {
        Skin1 = 0,
        Skin2 = 1,
        Skin3 = 2,
        Skin4 = 3,
        Skin5 = 4,
        Skin6 = 5,
        Iron = 6,
        Steel = 7,
        DanariOne = 8,
        DanariTwo = 9,
        DanariThree = 10,
        DanariFour = 11,
        ElfOne = 12,
        ElfTwo = 13,
        //ElfThree = 14,
        OrcOne = 14,
        OrcTwo = 15,
        OrcThree = 16,
        UndeadOne = 17,
        UndeadTwo = 18,
        UndeadThree = 19,
        Skin7 = 20,
        Skin8 = 21,
        Skin9 = 22,
        Skin10 = 23,
        Skin11 = 24,
        Skin12 = 25,
        Skin13 = 26,
        Skin14 = 27,
        Skin15 = 28,
        Skin16 = 29,
        Skin17 = 30,
        Skin18 = 31,
        OrcFour = 32,
    }
);
