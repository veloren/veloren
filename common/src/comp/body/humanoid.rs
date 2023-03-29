use crate::{make_case_elim, make_proj_elim};
use rand::{seq::SliceRandom, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

make_proj_elim!(
    body,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
        let species = *ALL_SPECIES.choose(&mut rng).unwrap();
        Self::random_with(&mut rng, &species)
    }

    #[inline]
    pub fn random_with(rng: &mut impl Rng, &species: &Species) -> Self {
        let body_type = *ALL_BODY_TYPES.choose(rng).unwrap();
        Self {
            species,
            body_type,
            hair_style: rng.gen_range(0..species.num_hair_styles(body_type)),
            beard: rng.gen_range(0..species.num_beards(body_type)),
            accessory: rng.gen_range(0..species.num_accessories(body_type)),
            hair_color: rng.gen_range(0..species.num_hair_colors()),
            skin: rng.gen_range(0..species.num_skin_colors()),
            eye_color: rng.gen_range(0..species.num_eye_colors()),
            eyes: rng.gen_range(0..1), /* TODO Add a way to set specific head-segments for NPCs
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

    pub fn height(&self) -> f32 { (20.0 / 9.0) * self.scaler() }

    pub fn scaler(&self) -> f32 {
        match (self.species, self.body_type) {
            (Species::Orc, BodyType::Male) => 0.91,
            (Species::Orc, BodyType::Female) => 0.81,
            (Species::Human, BodyType::Male) => 0.81,
            (Species::Human, BodyType::Female) => 0.76,
            (Species::Elf, BodyType::Male) => 0.82,
            (Species::Elf, BodyType::Female) => 0.76,
            (Species::Dwarf, BodyType::Male) => 0.67,
            (Species::Dwarf, BodyType::Female) => 0.62,
            (Species::Draugr, BodyType::Male) => 0.78,
            (Species::Draugr, BodyType::Female) => 0.72,
            (Species::Danari, BodyType::Male) => 0.56,
            (Species::Danari, BodyType::Female) => 0.56,
        }
    }
}

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::Humanoid(body) }
}

make_case_elim!(
    species,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Species {
        Danari = 0,
        Dwarf = 1,
        Elf = 2,
        Human = 3,
        Orc = 4,
        Draugr = 5,
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
    pub draugr: SpeciesMeta,
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
            Species::Draugr => &self.draugr,
        }
    }
}

pub const ALL_SPECIES: [Species; 6] = [
    Species::Danari,
    Species::Dwarf,
    Species::Elf,
    Species::Human,
    Species::Orc,
    Species::Draugr,
];

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

// Skin colors
pub const DANARI_SKIN_COLORS: [Skin; 7] = [
    Skin::DanariOne,
    Skin::DanariTwo,
    Skin::DanariThree,
    Skin::DanariFour,
    Skin::DanariFive,
    Skin::DanariSix,
    Skin::DanariSeven,
];
pub const DWARF_SKIN_COLORS: [Skin; 14] = [
    Skin::DwarfOne,
    Skin::DwarfTwo,
    Skin::DwarfThree,
    Skin::DwarfFour,
    Skin::DwarfFive,
    Skin::DwarfSix,
    Skin::DwarfSeven,
    Skin::DwarfEight,
    Skin::DwarfNine,
    Skin::DwarfTen,
    Skin::DwarfEleven,
    Skin::DwarfTwelve,
    Skin::DwarfThirteen,
    Skin::DwarfFourteen,
];
pub const ELF_SKIN_COLORS: [Skin; 18] = [
    Skin::ElfOne,
    Skin::ElfTwo,
    Skin::ElfThree,
    Skin::ElfFour,
    Skin::ElfFive,
    Skin::ElfSix,
    Skin::ElfSeven,
    Skin::ElfEight,
    Skin::ElfNine,
    Skin::ElfTen,
    Skin::ElfEleven,
    Skin::ElfTwelve,
    Skin::ElfThirteen,
    Skin::ElfFourteen,
    Skin::ElfFifteen,
    Skin::ElfSixteen,
    Skin::ElfSeventeen,
    Skin::ElfEighteen,
];
pub const HUMAN_SKIN_COLORS: [Skin; 18] = [
    Skin::HumanOne,
    Skin::HumanTwo,
    Skin::HumanThree,
    Skin::HumanFour,
    Skin::HumanFive,
    Skin::HumanSix,
    Skin::HumanSeven,
    Skin::HumanEight,
    Skin::HumanNine,
    Skin::HumanTen,
    Skin::HumanEleven,
    Skin::HumanTwelve,
    Skin::HumanThirteen,
    Skin::HumanFourteen,
    Skin::HumanFifteen,
    Skin::HumanSixteen,
    Skin::HumanSeventeen,
    Skin::HumanEighteen,
];
pub const ORC_SKIN_COLORS: [Skin; 8] = [
    Skin::OrcOne,
    Skin::OrcTwo,
    Skin::OrcThree,
    Skin::OrcFour,
    Skin::OrcFive,
    Skin::OrcSix,
    Skin::OrcSeven,
    Skin::OrcEight,
];
pub const DRAUGR_SKIN_COLORS: [Skin; 9] = [
    Skin::DraugrOne,
    Skin::DraugrTwo,
    Skin::DraugrThree,
    Skin::DraugrFour,
    Skin::DraugrFive,
    Skin::DraugrSix,
    Skin::DraugrSeven,
    Skin::DraugrEight,
    Skin::DraugrNine,
];

// Eye colors
pub const DANARI_EYE_COLORS: [EyeColor; 4] = [
    EyeColor::EmeraldGreen,
    EyeColor::LoyalBrown,
    EyeColor::RegalPurple,
    EyeColor::ViciousRed,
];
pub const DWARF_EYE_COLORS: [EyeColor; 6] = [
    EyeColor::AmberYellow,
    EyeColor::CornflowerBlue,
    EyeColor::LoyalBrown,
    EyeColor::NobleBlue,
    EyeColor::PineGreen,
    EyeColor::RustBrown,
];
pub const ELF_EYE_COLORS: [EyeColor; 7] = [
    EyeColor::AmberYellow,
    EyeColor::BrightBrown,
    EyeColor::EmeraldGreen,
    EyeColor::NobleBlue,
    EyeColor::SapphireBlue,
    EyeColor::RegalPurple,
    EyeColor::RubyRed,
];
pub const HUMAN_EYE_COLORS: [EyeColor; 5] = [
    EyeColor::NobleBlue,
    EyeColor::CornflowerBlue,
    EyeColor::CuriousGreen,
    EyeColor::LoyalBrown,
    EyeColor::VigorousBlack,
];
pub const ORC_EYE_COLORS: [EyeColor; 6] = [
    EyeColor::AmberYellow,
    EyeColor::CornflowerBlue,
    EyeColor::ExoticPurple,
    EyeColor::LoyalBrown,
    EyeColor::PineGreen,
    EyeColor::RustBrown,
];
pub const DRAUGR_EYE_COLORS: [EyeColor; 6] = [
    EyeColor::FrozenBlue,
    EyeColor::GhastlyYellow,
    EyeColor::MagicPurple,
    EyeColor::PumpkinOrange,
    EyeColor::ToxicGreen,
    EyeColor::ViciousRed,
];

impl Species {
    fn skin_colors(self) -> &'static [Skin] {
        match self {
            Species::Danari => &DANARI_SKIN_COLORS,
            Species::Dwarf => &DWARF_SKIN_COLORS,
            Species::Elf => &ELF_SKIN_COLORS,
            Species::Human => &HUMAN_SKIN_COLORS,
            Species::Orc => &ORC_SKIN_COLORS,
            Species::Draugr => &DRAUGR_SKIN_COLORS,
        }
    }

    fn eye_colors(self) -> &'static [EyeColor] {
        match self {
            Species::Danari => &DANARI_EYE_COLORS,
            Species::Dwarf => &DWARF_EYE_COLORS,
            Species::Elf => &ELF_EYE_COLORS,
            Species::Human => &HUMAN_EYE_COLORS,
            Species::Orc => &ORC_EYE_COLORS,
            Species::Draugr => &DRAUGR_EYE_COLORS,
        }
    }

    /// FIXME: This is a hack!  The only reason we need to do this is because
    /// hair colors are currently just indices into an array, not enum
    /// variants.  Once we have proper variants for hair colors, we won't
    /// need to do this anymore, since we will use locally defined arrays to
    /// represent per-species stuff (or have some other solution for validity).
    pub fn num_hair_colors(self) -> u8 {
        match self {
            Species::Danari => 17,
            Species::Dwarf => 21,
            Species::Elf => 24,
            Species::Human => 22,
            Species::Orc => 13,
            Species::Draugr => 25,
        }
    }

    pub fn skin_color(self, val: u8) -> Skin {
        self.skin_colors()
            .get(val as usize)
            .copied()
            .unwrap_or(Skin::HumanThree)
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
            (Species::Danari, BodyType::Female) => 15,
            (Species::Danari, BodyType::Male) => 15,
            (Species::Dwarf, BodyType::Female) => 15,
            (Species::Dwarf, BodyType::Male) => 15,
            (Species::Elf, BodyType::Female) => 22,
            (Species::Elf, BodyType::Male) => 15,
            (Species::Human, BodyType::Female) => 20,
            (Species::Human, BodyType::Male) => 21,
            (Species::Orc, BodyType::Female) => 15,
            (Species::Orc, BodyType::Male) => 15,
            (Species::Draugr, BodyType::Female) => 15,
            (Species::Draugr, BodyType::Male) => 15,
        }
    }

    pub fn num_accessories(self, body_type: BodyType) -> u8 {
        match (self, body_type) {
            (Species::Danari, BodyType::Female) => 7,
            (Species::Danari, BodyType::Male) => 7,
            (Species::Dwarf, BodyType::Female) => 7,
            (Species::Dwarf, BodyType::Male) => 7,
            (Species::Elf, BodyType::Female) => 6,
            (Species::Elf, BodyType::Male) => 5,
            (Species::Human, BodyType::Female) => 1,
            (Species::Human, BodyType::Male) => 1,
            (Species::Orc, BodyType::Female) => 9,
            (Species::Orc, BodyType::Male) => 12,
            (Species::Draugr, BodyType::Female) => 2,
            (Species::Draugr, BodyType::Male) => 2,
        }
    }

    pub fn num_eyebrows(self, _body_type: BodyType) -> u8 { 1 }

    pub fn num_eyes(self, body_type: BodyType) -> u8 {
        match (self, body_type) {
            (Species::Danari, BodyType::Female) => 6,
            (Species::Danari, BodyType::Male) => 8,
            (Species::Dwarf, BodyType::Female) => 6,
            (Species::Dwarf, BodyType::Male) => 9,
            (Species::Elf, BodyType::Female) => 6,
            (Species::Elf, BodyType::Male) => 8,
            (Species::Human, BodyType::Female) => 6,
            (Species::Human, BodyType::Male) => 7,
            (Species::Orc, BodyType::Female) => 6,
            (Species::Orc, BodyType::Male) => 2,
            (Species::Draugr, BodyType::Female) => 3,
            (Species::Draugr, BodyType::Male) => 8,
        }
    }

    pub fn num_beards(self, body_type: BodyType) -> u8 {
        match (self, body_type) {
            (Species::Danari, BodyType::Female) => 1,
            (Species::Danari, BodyType::Male) => 16,
            (Species::Dwarf, BodyType::Female) => 1,
            (Species::Dwarf, BodyType::Male) => 23,
            (Species::Elf, BodyType::Female) => 1,
            (Species::Elf, BodyType::Male) => 8,
            (Species::Human, BodyType::Female) => 1,
            (Species::Human, BodyType::Male) => 10,
            (Species::Orc, BodyType::Female) => 1,
            (Species::Orc, BodyType::Male) => 7,
            (Species::Draugr, BodyType::Female) => 1,
            (Species::Draugr, BodyType::Male) => 6,
        }
    }
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

make_case_elim!(
    eye_color,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
    #[repr(u32)]
    pub enum EyeColor {
        AmberOrange = 0,
        AmberYellow = 1,
        BrightBrown = 2,
        CornflowerBlue = 3,
        CuriousGreen = 4,
        EmeraldGreen = 5,
        ExoticPurple = 6,
        FrozenBlue = 7,
        GhastlyYellow = 8,
        LoyalBrown = 9,
        MagicPurple = 10,
        NobleBlue = 11,
        PineGreen = 12,
        PumpkinOrange = 13,
        RubyRed = 14,
        RegalPurple = 15,
        RustBrown = 16,
        SapphireBlue = 17,
        SulfurYellow = 18,
        ToxicGreen = 19,
        ViciousRed = 20,
        VigorousBlack = 21,
    }
);

make_case_elim!(
    skin,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
    #[repr(u32)]
    pub enum Skin {
        // Humans
        HumanOne = 0,
        HumanTwo = 1,
        HumanThree = 2,
        HumanFour = 3,
        HumanFive = 4,
        HumanSix = 5,
        HumanSeven = 6,
        HumanEight = 7,
        HumanNine = 8,
        HumanTen = 9,
        HumanEleven = 10,
        HumanTwelve = 11,
        HumanThirteen = 12,
        HumanFourteen = 13,
        HumanFifteen = 14,
        HumanSixteen = 15,
        HumanSeventeen = 16,
        HumanEighteen = 17,
        // Dwarves
        DwarfOne = 18,
        DwarfTwo = 19,
        DwarfThree = 20,
        DwarfFour = 21,
        DwarfFive = 22,
        DwarfSix = 23,
        DwarfSeven = 24,
        DwarfEight = 25,
        DwarfNine = 26,
        DwarfTen = 27,
        DwarfEleven = 28,
        DwarfTwelve = 29,
        DwarfThirteen = 30,
        DwarfFourteen = 31,
        // Elves
        ElfOne = 32,
        ElfTwo = 33,
        ElfThree = 34,
        ElfFour = 35,
        ElfFive = 36,
        ElfSix = 37,
        ElfSeven = 38,
        ElfEight = 39,
        ElfNine = 40,
        ElfTen = 41,
        ElfEleven = 42,
        ElfTwelve = 43,
        ElfThirteen = 44,
        ElfFourteen = 45,
        ElfFifteen = 46,
        ElfSixteen = 47,
        ElfSeventeen = 48,
        ElfEighteen = 49,
        // Orcs
        OrcOne = 50,
        OrcTwo = 51,
        OrcThree = 52,
        OrcFour = 53,
        OrcFive = 54,
        OrcSix = 55,
        OrcSeven = 56,
        OrcEight = 57,
        // Danaris
        DanariOne = 58,
        DanariTwo = 59,
        DanariThree = 60,
        DanariFour = 61,
        DanariFive = 62,
        DanariSix = 63,
        DanariSeven = 64,
        // Draugrs
        DraugrOne = 65,
        DraugrTwo = 66,
        DraugrThree = 67,
        DraugrFour = 68,
        DraugrFive = 69,
        DraugrSix = 70,
        DraugrSeven = 71,
        DraugrEight = 72,
        DraugrNine = 73,
    }
);
