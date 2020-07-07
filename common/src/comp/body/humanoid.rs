use rand::{seq::SliceRandom, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use vek::Rgb;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Body {
    pub species: Species,
    pub body_type: BodyType,
    pub hair_style: u8,
    pub beard: u8,
    pub eyes: u8,
    pub accessory: u8,
    pub hair_color: u8,
    pub skin: u8,
    pub eye_color: u8,
}

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
            hair_color: rng.gen_range(0, species.num_hair_colors()) as u8,
            skin: rng.gen_range(0, species.num_skin_colors()) as u8,
            eye_color: rng.gen_range(0, species.num_eye_colors()) as u8,
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
        self.eye_color = self.hair_style.min(self.species.num_eye_colors() - 1);
        self.accessory = self
            .accessory
            .min(self.species.num_accessories(self.body_type) - 1);
    }
}

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::Humanoid(body) }
}

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

// Hair Colors
pub const DANARI_HAIR_COLORS: [(u8, u8, u8); 12] = [
    (198, 169, 113), // Philosopher's Grey
    //(245, 232, 175), // Cream Blonde
    //(228, 208, 147), // Gold Blonde
    //(228, 223, 141), // Platinum Blonde
    (199, 131, 58), // Summer Blonde
    (107, 76, 51),  // Oak Skin4
    //(203, 154, 98),  // Light Skin4
    (64, 32, 18),  // Skin7 Skin4
    (86, 72, 71),  // Ash Skin4
    (57, 56, 61),  // Raven Black
    (101, 83, 95), // Matte Purple
    (101, 57, 90), // Witch Purple
    (107, 32, 60), // Grape Purple
    (135, 38, 39), // Dark Red
    (88, 26, 29),  // Wine Red
    //(146, 32, 32), // Autumn Red
    (20, 19, 17), // Black
];
pub const DWARF_HAIR_COLORS: [(u8, u8, u8); 21] = [
    (245, 232, 175), // Cream Blonde
    (228, 208, 147), // Gold Blonde
    (228, 223, 141), // Platinum Blonde
    (199, 131, 58),  // Summer Blonde
    (107, 76, 51),   // Oak Skin4
    (203, 154, 98),  // Light Skin4
    (64, 32, 18),    // Skin7 Skin4
    (86, 72, 71),    // Ash Skin4
    (57, 56, 61),    // Raven Black
    (101, 83, 95),   // Matte Purple
    (101, 57, 90),   // Witch Purple
    (135, 38, 39),   // Dark Red
    (88, 26, 29),    // Wine Red
    (191, 228, 254), // Ice NobleBlue
    (92, 80, 144),   // Kingfisher Blue
    (146, 198, 238), // Lagoon Blue
    (174, 148, 161), // Matte Pink
    (163, 186, 192), // Matte Green
    (84, 139, 107),  // Grass Green
    (48, 61, 52),    // Dark Green
    (20, 19, 17),    // Black
];
pub const ELF_HAIR_COLORS: [(u8, u8, u8); 24] = [
    (66, 83, 113),   // Mysterious Blue
    (13, 76, 41),    // Rainforest Green
    (245, 232, 175), // Cream Blonde
    (228, 208, 147), // Gold Blonde
    (228, 223, 141), // Platinum Blonde
    (199, 131, 58),  // Summer Blonde
    (107, 76, 51),   // Oak Skin4
    (203, 154, 98),  // Light Skin4
    (64, 32, 18),    // Skin7 Skin4
    (86, 72, 71),    // Ash Skin4
    (57, 56, 61),    // Raven Black
    (101, 83, 95),   // Matte Purple
    (101, 57, 90),   // Witch Purple
    (135, 38, 39),   // Dark Red
    (88, 26, 29),    // Wine Red
    (191, 228, 254), // Ice Blue
    (92, 80, 144),   // Kingfisher Blue
    (146, 198, 238), // Lagoon Blue
    (224, 182, 184), // Candy Pink
    (174, 148, 161), // Matte Pink
    (163, 186, 192), // Matte Green
    (84, 139, 107),  // Grass Green
    (48, 61, 52),    // Dark Green
    (20, 19, 17),    // Black
];
pub const HUMAN_HAIR_COLORS: [(u8, u8, u8); 22] = [
    (245, 232, 175), // Cream Blonde
    (228, 208, 147), // Gold Blonde
    (228, 223, 141), // Platinum Blonde
    (199, 131, 58),  // Summer Blonde
    (107, 76, 51),   // Oak Skin4
    (203, 154, 98),  // Light Skin4
    (64, 32, 18),    // Skin7 Skin4
    (86, 72, 71),    // Ash Skin4
    (57, 56, 61),    // Raven Black
    (101, 83, 95),   // Matte Purple
    (101, 57, 90),   // Witch Purple
    (135, 38, 39),   // Dark Red
    (88, 26, 29),    // Wine Red
    (191, 228, 254), // Ice Blue
    (92, 80, 144),   // Kingfisher Blue
    (146, 198, 238), // Lagoon Blue
    (224, 182, 184), // Candy Pink
    (174, 148, 161), // Matte Pink
    (163, 186, 192), // Matte Green
    (84, 139, 107),  // Grass Green
    (48, 61, 52),    // Dark Green
    (20, 19, 17),    // Black
];
pub const ORC_HAIR_COLORS: [(u8, u8, u8); 11] = [
    (66, 66, 59), // Wise Grey
    //(107, 76, 51),  // Oak Skin4
    //(203, 154, 98), // Light Skin4
    (64, 32, 18),  // Skin7 Skin4
    (54, 30, 26),  // Dark Skin7
    (86, 72, 71),  // Ash Skin4
    (57, 56, 61),  // Raven Black
    (101, 83, 95), // Matte Purple
    (101, 57, 90), // Witch Purple
    (135, 38, 39), // Dark Red
    (88, 26, 29),  // Wine Red
    (66, 83, 113), // Mysterious Blue
    (20, 19, 17),  // Black
];
pub const UNDEAD_HAIR_COLORS: [(u8, u8, u8); 22] = [
    //(245, 232, 175), // Cream Blonde
    (228, 208, 147), // Gold Blonde
    //(228, 223, 141), // Platinum Blonde
    (199, 131, 58),  // Summer Blonde
    (107, 76, 51),   // Oak Skin4
    (203, 154, 98),  // Light Skin4
    (64, 32, 18),    // Skin7 Skin4
    (86, 72, 71),    // Ash Skin4
    (57, 56, 61),    // Raven Black
    (101, 83, 95),   // Matte Purple
    (101, 57, 90),   // Witch Purple
    (111, 54, 117),  // Punky Purple
    (135, 38, 39),   // Dark Red
    (88, 26, 29),    // Wine Red
    (191, 228, 254), // Ice Blue
    (92, 80, 144),   // Kingfisher Blue
    (146, 198, 238), // Lagoon Blue
    (66, 66, 59),    // Decayed Grey
    //(224, 182, 184), // Candy Pink
    (174, 148, 161), // Matte Pink
    (0, 131, 122),   // Rotten Green
    (163, 186, 192), // Matte Green
    (84, 139, 107),  // Grass Green
    (48, 61, 52),    // Dark Green
    (20, 19, 17),    // Black
];

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
    fn hair_colors(self) -> &'static [(u8, u8, u8)] {
        match self {
            Species::Danari => &DANARI_HAIR_COLORS,
            Species::Dwarf => &DWARF_HAIR_COLORS,
            Species::Elf => &ELF_HAIR_COLORS,
            Species::Human => &HUMAN_HAIR_COLORS,
            Species::Orc => &ORC_HAIR_COLORS,
            Species::Undead => &UNDEAD_HAIR_COLORS,
        }
    }

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

    pub fn hair_color(self, val: u8) -> Rgb<u8> {
        self.hair_colors()
            .get(val as usize)
            .copied()
            .unwrap_or((0, 0, 0))
            .into()
    }

    pub fn num_hair_colors(self) -> u8 { self.hair_colors().len() as u8 }

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
            (Species::Dwarf, BodyType::Female) => 7,
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
            (Species::Orc, BodyType::Male) => 5,
            (Species::Undead, BodyType::Female) => 1,
            (Species::Undead, BodyType::Male) => 1,
        }
    }

    pub fn num_eyebrows(self, _body_type: BodyType) -> u8 { 1 }

    pub fn num_eyes(self, body_type: BodyType) -> u8 {
        match (self, body_type) {
            (Species::Danari, BodyType::Female) => 6,
            (Species::Danari, BodyType::Male) => 7,
            (Species::Dwarf, BodyType::Female) => 6,
            (Species::Dwarf, BodyType::Male) => 7,
            (Species::Elf, BodyType::Female) => 6,
            (Species::Elf, BodyType::Male) => 7,
            (Species::Human, BodyType::Female) => 6,
            (Species::Human, BodyType::Male) => 5,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum BodyType {
    Female = 0,
    Male = 1,
}
pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
impl EyeColor {
    pub fn light_rgb(self) -> Rgb<u8> {
        match self {
            EyeColor::VigorousBlack => Rgb::new(71, 59, 49),
            EyeColor::NobleBlue => Rgb::new(75, 158, 191),
            EyeColor::CuriousGreen => Rgb::new(110, 167, 113),
            EyeColor::LoyalBrown => Rgb::new(73, 42, 36),
            EyeColor::ViciousRed => Rgb::new(182, 0, 0),
            EyeColor::PumpkinOrange => Rgb::new(220, 156, 19),
            EyeColor::GhastlyYellow => Rgb::new(221, 225, 31),
            EyeColor::MagicPurple => Rgb::new(137, 4, 177),
            EyeColor::ToxicGreen => Rgb::new(1, 223, 1),
            EyeColor::ExoticPurple => Rgb::new(95, 32, 111),
            EyeColor::SulfurYellow => Rgb::new(235, 198, 94),
            EyeColor::AmberOrange => Rgb::new(137, 46, 1),
            EyeColor::PineGreen => Rgb::new(0, 78, 56),
            EyeColor::CornflowerBlue => Rgb::new(18, 66, 90),
        }
    }

    pub fn dark_rgb(self) -> Rgb<u8> {
        match self {
            EyeColor::VigorousBlack => Rgb::new(32, 32, 32),
            EyeColor::NobleBlue => Rgb::new(62, 130, 159),
            EyeColor::CuriousGreen => Rgb::new(81, 124, 84),
            EyeColor::LoyalBrown => Rgb::new(54, 30, 26),
            EyeColor::ViciousRed => Rgb::new(148, 0, 0),
            EyeColor::PumpkinOrange => Rgb::new(209, 145, 18),
            EyeColor::GhastlyYellow => Rgb::new(205, 212, 29),
            EyeColor::MagicPurple => Rgb::new(110, 3, 143),
            EyeColor::ToxicGreen => Rgb::new(1, 185, 1),
            EyeColor::ExoticPurple => Rgb::new(69, 23, 80),
            EyeColor::SulfurYellow => Rgb::new(209, 176, 84),
            EyeColor::AmberOrange => Rgb::new(112, 40, 1),
            EyeColor::PineGreen => Rgb::new(0, 54, 38),
            EyeColor::CornflowerBlue => Rgb::new(13, 47, 64),
        }
    }

    pub fn white_rgb(self) -> Rgb<u8> { Rgb::new(255, 255, 255) }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Accessory {
    Nothing = 0,
    Some = 1,
}
pub const ALL_ACCESSORIES: [Accessory; 2] = [Accessory::Nothing, Accessory::Some];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
impl Skin {
    pub fn rgb(self) -> Rgb<u8> {
        let color = match self {
            Self::Skin1 => (255, 229, 200),
            Self::Skin2 => (255, 218, 190),
            Self::Skin3 => (255, 206, 180),
            Self::Skin4 => (255, 195, 170),
            Self::Skin5 => (240, 184, 160),
            Self::Skin6 => (225, 172, 150),
            Self::Skin7 => (210, 161, 140),
            Self::Skin8 => (195, 149, 130),
            Self::Skin9 => (180, 138, 120),
            Self::Skin10 => (165, 126, 110),
            Self::Skin11 => (150, 114, 100),
            Self::Skin12 => (135, 103, 90),
            Self::Skin13 => (120, 92, 80),
            Self::Skin14 => (105, 80, 70),
            Self::Skin15 => (90, 69, 60),
            Self::Skin16 => (75, 57, 50),
            Self::Skin17 => (60, 46, 40),
            Self::Skin18 => (45, 34, 30),
            Self::Iron => (135, 113, 95),
            Self::Steel => (108, 94, 86),
            Self::DanariOne => (104, 168, 196),
            Self::DanariTwo => (30, 149, 201),
            Self::DanariThree => (57, 120, 148),
            Self::DanariFour => (40, 85, 105),
            Self::ElfOne => (178, 164, 186),
            Self::ElfTwo => (132, 139, 161),
            //Self::ElfThree => (230, 188, 198),
            Self::OrcOne => (61, 130, 42),
            Self::OrcTwo => (82, 117, 36),
            Self::OrcThree => (71, 94, 42),
            Self::OrcFour => (97, 54, 29),
            Self::UndeadOne => (240, 243, 239),
            Self::UndeadTwo => (178, 178, 178),
            Self::UndeadThree => (145, 135, 121),
        };
        Rgb::from(color)
    }

    pub fn light_rgb(self) -> Rgb<u8> {
        let color = match self {
            Self::Skin1 => (255, 229, 200),
            Self::Skin2 => (255, 218, 190),
            Self::Skin3 => (255, 206, 180),
            Self::Skin4 => (255, 195, 170),
            Self::Skin5 => (240, 184, 160),
            Self::Skin6 => (225, 172, 150),
            Self::Skin7 => (210, 161, 140),
            Self::Skin8 => (195, 149, 130),
            Self::Skin9 => (180, 138, 120),
            Self::Skin10 => (165, 126, 110),
            Self::Skin11 => (150, 114, 100),
            Self::Skin12 => (135, 103, 90),
            Self::Skin13 => (120, 92, 80),
            Self::Skin14 => (105, 80, 70),
            Self::Skin15 => (90, 69, 60),
            Self::Skin16 => (75, 57, 50),
            Self::Skin17 => (60, 46, 40),
            Self::Skin18 => (45, 34, 30),
            Self::Iron => (144, 125, 106),
            Self::Steel => (120, 107, 99),
            Self::DanariOne => (116, 176, 208),
            Self::DanariTwo => (42, 158, 206),
            Self::DanariThree => (70, 133, 160),
            Self::DanariFour => (53, 96, 116),
            Self::ElfOne => (190, 176, 199), //178, 164, 186
            Self::ElfTwo => (137, 144, 167),
            //Self::ElfThree => (242, 199, 209),
            Self::OrcOne => (83, 165, 56),
            Self::OrcTwo => (92, 132, 46),
            Self::OrcThree => (84, 110, 54),
            Self::OrcFour => (97, 54, 29),
            Self::UndeadOne => (254, 252, 251),
            Self::UndeadTwo => (190, 192, 191),
            Self::UndeadThree => (160, 151, 134),
        };
        Rgb::from(color)
    }

    pub fn dark_rgb(self) -> Rgb<u8> {
        let color = match self {
            Self::Skin1 => (242, 217, 189),
            Self::Skin2 => (242, 207, 189),
            Self::Skin3 => (242, 197, 172),
            Self::Skin4 => (242, 186, 162),
            Self::Skin5 => (212, 173, 150),
            Self::Skin6 => (212, 163, 142),
            Self::Skin7 => (196, 151, 132),
            Self::Skin8 => (181, 139, 121),
            Self::Skin9 => (168, 129, 113),
            Self::Skin10 => (153, 117, 103),
            Self::Skin11 => (138, 105, 92),
            Self::Skin12 => (122, 93, 82),
            Self::Skin13 => (107, 82, 72),
            Self::Skin14 => (92, 70, 62),
            Self::Skin15 => (77, 59, 51),
            Self::Skin16 => (61, 47, 41),
            Self::Skin17 => (48, 37, 32),
            Self::Skin18 => (33, 25, 22),
            Self::Iron => (124, 99, 82),
            Self::Steel => (96, 81, 72),
            Self::DanariOne => (92, 155, 183),
            Self::DanariTwo => (25, 142, 192),
            Self::DanariThree => (52, 115, 143),
            Self::DanariFour => (34, 80, 99),
            Self::ElfOne => (170, 155, 175), //170, 157, 179
            Self::ElfTwo => (126, 132, 153),
            //Self::ElfThree => (217, 178, 187),
            Self::OrcOne => (55, 114, 36),
            Self::OrcTwo => (70, 104, 29),
            Self::OrcThree => (60, 83, 32),
            Self::OrcFour => (84, 47, 25),
            Self::UndeadOne => (229, 231, 230),
            Self::UndeadTwo => (165, 166, 164),
            Self::UndeadThree => (130, 122, 106),
        };
        Rgb::from(color)
    }
}
