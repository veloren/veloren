use rand::{seq::SliceRandom, thread_rng, Rng};
use vek::Rgb;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Body {
    pub race: Race,
    pub body_type: BodyType,
    pub chest: Chest,
    pub belt: Belt,
    pub pants: Pants,
    pub hand: Hand,
    pub foot: Foot,
    pub shoulder: Shoulder,
    pub hair_style: u8,
    pub beard: u8,
    pub eyebrows: Eyebrows,
    pub accessory: u8,
    pub hair_color: u8,
    pub skin: u8,
    pub eye_color: u8,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        let race = *(&ALL_RACES).choose(&mut rng).unwrap();
        let body_type = *(&ALL_BODY_TYPES).choose(&mut rng).unwrap();
        Self {
            race,
            body_type,
            chest: *(&ALL_CHESTS).choose(&mut rng).unwrap(),
            belt: *(&ALL_BELTS).choose(&mut rng).unwrap(),
            pants: *(&ALL_PANTS).choose(&mut rng).unwrap(),
            hand: *(&ALL_HANDS).choose(&mut rng).unwrap(),
            foot: *(&ALL_FEET).choose(&mut rng).unwrap(),
            shoulder: *(&ALL_SHOULDERS).choose(&mut rng).unwrap(),
            hair_style: rng.gen_range(0, race.num_hair_styles(body_type)),
            beard: rng.gen_range(0, race.num_beards(body_type)),
            eyebrows: *(&ALL_EYEBROWS).choose(&mut rng).unwrap(),
            accessory: rng.gen_range(0, race.num_accessories(body_type)),
            hair_color: rng.gen_range(0, race.num_hair_colors()) as u8,
            skin: rng.gen_range(0, race.num_skin_colors()) as u8,
            eye_color: rng.gen_range(0, race.num_eye_colors()) as u8,
        }
    }

    pub fn validate(&mut self) {
        self.hair_style = self
            .hair_style
            .min(self.race.num_hair_styles(self.body_type) - 1);
        self.beard = self.beard.min(self.race.num_beards(self.body_type) - 1);
        self.hair_color = self.hair_color.min(self.race.num_hair_colors() - 1);
        self.skin = self.skin.min(self.race.num_skin_colors() - 1);
        self.eye_color = self.hair_style.min(self.race.num_eye_colors() - 1);
        self.accessory = self
            .accessory
            .min(self.race.num_accessories(self.body_type) - 1);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Race {
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

impl<SpeciesMeta> core::ops::Index<Race> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    fn index(&self, index: Race) -> &Self::Output {
        match index {
            Race::Danari => &self.danari,
            Race::Dwarf => &self.dwarf,
            Race::Elf => &self.elf,
            Race::Human => &self.human,
            Race::Orc => &self.orc,
            Race::Undead => &self.undead,
        }
    }
}

pub const ALL_RACES: [Race; 6] = [
    Race::Danari,
    Race::Dwarf,
    Race::Elf,
    Race::Human,
    Race::Orc,
    Race::Undead,
];

// Hair Colors
pub const DANARI_HAIR_COLORS: [(u8, u8, u8); 11] = [
    (198, 169, 113), // Philosopher's Grey
    //(245, 232, 175), // Cream Blonde
    //(228, 208, 147), // Gold Blonde
    //(228, 223, 141), // Platinum Blonde
    (199, 131, 58), // Summer Blonde
    (107, 76, 51),  // Oak Brown
    //(203, 154, 98),  // Light Brown
    (64, 32, 18),  // Chocolate Brown
    (86, 72, 71),  // Ash Brown
    (57, 56, 61),  // Raven Black
    (101, 83, 95), // Matte Purple
    (101, 57, 90), // Witch Purple
    (107, 32, 60), // Grape Purple
    (135, 38, 39), // Dark Red
    (88, 26, 29),  // Wine Red
                   //(146, 32, 32), // Autumn Red
];
pub const DWARF_HAIR_COLORS: [(u8, u8, u8); 20] = [
    (245, 232, 175), // Cream Blonde
    (228, 208, 147), // Gold Blonde
    (228, 223, 141), // Platinum Blonde
    (199, 131, 58),  // Summer Blonde
    (107, 76, 51),   // Oak Brown
    (203, 154, 98),  // Light Brown
    (64, 32, 18),    // Chocolate Brown
    (86, 72, 71),    // Ash Brown
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
];
pub const ELF_HAIR_COLORS: [(u8, u8, u8); 23] = [
    (66, 83, 113),   // Mysterious Blue
    (13, 76, 41),    // Rainforest Green
    (245, 232, 175), // Cream Blonde
    (228, 208, 147), // Gold Blonde
    (228, 223, 141), // Platinum Blonde
    (199, 131, 58),  // Summer Blonde
    (107, 76, 51),   // Oak Brown
    (203, 154, 98),  // Light Brown
    (64, 32, 18),    // Chocolate Brown
    (86, 72, 71),    // Ash Brown
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
];
pub const HUMAN_HAIR_COLORS: [(u8, u8, u8); 21] = [
    (245, 232, 175), // Cream Blonde
    (228, 208, 147), // Gold Blonde
    (228, 223, 141), // Platinum Blonde
    (199, 131, 58),  // Summer Blonde
    (107, 76, 51),   // Oak Brown
    (203, 154, 98),  // Light Brown
    (64, 32, 18),    // Chocolate Brown
    (86, 72, 71),    // Ash Brown
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
];
pub const ORC_HAIR_COLORS: [(u8, u8, u8); 10] = [
    (66, 66, 59), // Wise Grey
    //(107, 76, 51),  // Oak Brown
    //(203, 154, 98), // Light Brown
    (64, 32, 18),  // Chocolate Brown
    (54, 30, 26),  // Dark Chocolate
    (86, 72, 71),  // Ash Brown
    (57, 56, 61),  // Raven Black
    (101, 83, 95), // Matte Purple
    (101, 57, 90), // Witch Purple
    (135, 38, 39), // Dark Red
    (88, 26, 29),  // Wine Red
    (66, 83, 113), // Mysterious Blue
];
pub const UNDEAD_HAIR_COLORS: [(u8, u8, u8); 21] = [
    //(245, 232, 175), // Cream Blonde
    (228, 208, 147), // Gold Blonde
    //(228, 223, 141), // Platinum Blonde
    (199, 131, 58),  // Summer Blonde
    (107, 76, 51),   // Oak Brown
    (203, 154, 98),  // Light Brown
    (64, 32, 18),    // Chocolate Brown
    (86, 72, 71),    // Ash Brown
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
];

// Skin colors
pub const DANARI_SKIN_COLORS: [Skin; 4] = [
    Skin::DanariOne,
    Skin::DanariTwo,
    Skin::DanariThree,
    Skin::DanariFour,
];
pub const DWARF_SKIN_COLORS: [Skin; 5] = [
    Skin::Pale,
    Skin::White,
    Skin::Tanned,
    Skin::Iron,
    Skin::Steel,
];
pub const ELF_SKIN_COLORS: [Skin; 7] = [
    Skin::Pale,
    Skin::ElfOne,
    Skin::ElfTwo,
    Skin::ElfThree,
    Skin::White,
    Skin::Tanned,
    Skin::TannedBrown,
];
pub const HUMAN_SKIN_COLORS: [Skin; 5] = [
    Skin::Pale,
    Skin::White,
    Skin::Tanned,
    Skin::TannedBrown,
    Skin::TannedDarkBrown,
];
pub const ORC_SKIN_COLORS: [Skin; 4] = [Skin::OrcOne, Skin::OrcTwo, Skin::OrcThree, Skin::Brown];
pub const UNDEAD_SKIN_COLORS: [Skin; 3] = [Skin::UndeadOne, Skin::UndeadTwo, Skin::UndeadThree];

// Eye colors
pub const DANARI_EYE_COLORS: [EyeColor; 3] = [
    EyeColor::CuriousGreen,
    EyeColor::LoyalBrown,
    EyeColor::ViciousRed,
];
pub const DWARF_EYE_COLORS: [EyeColor; 3] = [
    EyeColor::CuriousGreen,
    EyeColor::LoyalBrown,
    EyeColor::NobleBlue,
];
pub const ELF_EYE_COLORS: [EyeColor; 3] = [
    EyeColor::NobleBlue,
    EyeColor::CuriousGreen,
    EyeColor::LoyalBrown,
];
pub const HUMAN_EYE_COLORS: [EyeColor; 3] = [
    EyeColor::NobleBlue,
    EyeColor::CuriousGreen,
    EyeColor::LoyalBrown,
];
pub const ORC_EYE_COLORS: [EyeColor; 2] = [EyeColor::LoyalBrown, EyeColor::ExoticPurple];
pub const UNDEAD_EYE_COLORS: [EyeColor; 5] = [
    EyeColor::ViciousRed,
    EyeColor::PumpkinOrange,
    EyeColor::GhastlyYellow,
    EyeColor::MagicPurple,
    EyeColor::ToxicGreen,
];

impl Race {
    fn hair_colors(self) -> &'static [(u8, u8, u8)] {
        match self {
            Race::Danari => &DANARI_HAIR_COLORS,
            Race::Dwarf => &DWARF_HAIR_COLORS,
            Race::Elf => &ELF_HAIR_COLORS,
            Race::Human => &HUMAN_HAIR_COLORS,
            Race::Orc => &ORC_HAIR_COLORS,
            Race::Undead => &UNDEAD_HAIR_COLORS,
        }
    }

    fn skin_colors(self) -> &'static [Skin] {
        match self {
            Race::Danari => &DANARI_SKIN_COLORS,
            Race::Dwarf => &DWARF_SKIN_COLORS,
            Race::Elf => &ELF_SKIN_COLORS,
            Race::Human => &HUMAN_SKIN_COLORS,
            Race::Orc => &ORC_SKIN_COLORS,
            Race::Undead => &UNDEAD_SKIN_COLORS,
        }
    }

    fn eye_colors(self) -> &'static [EyeColor] {
        match self {
            Race::Danari => &DANARI_EYE_COLORS,
            Race::Dwarf => &DWARF_EYE_COLORS,
            Race::Elf => &ELF_EYE_COLORS,
            Race::Human => &HUMAN_EYE_COLORS,
            Race::Orc => &ORC_EYE_COLORS,
            Race::Undead => &UNDEAD_EYE_COLORS,
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
            .unwrap_or(Skin::Tanned)
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
            (Race::Danari, BodyType::Female) => 2,
            (Race::Danari, BodyType::Male) => 2,
            (Race::Dwarf, BodyType::Female) => 4,
            (Race::Dwarf, BodyType::Male) => 3,
            (Race::Elf, BodyType::Female) => 21,
            (Race::Elf, BodyType::Male) => 4,
            (Race::Human, BodyType::Female) => 19,
            (Race::Human, BodyType::Male) => 17,
            (Race::Orc, BodyType::Female) => 1,
            (Race::Orc, BodyType::Male) => 8,
            (Race::Undead, BodyType::Female) => 4,
            (Race::Undead, BodyType::Male) => 3,
        }
    }

    pub fn num_accessories(self, body_type: BodyType) -> u8 {
        match (self, body_type) {
            (Race::Danari, BodyType::Female) => 1,
            (Race::Danari, BodyType::Male) => 1,
            (Race::Dwarf, BodyType::Female) => 1,
            (Race::Dwarf, BodyType::Male) => 1,
            (Race::Elf, BodyType::Female) => 2,
            (Race::Elf, BodyType::Male) => 1,
            (Race::Human, BodyType::Female) => 1,
            (Race::Human, BodyType::Male) => 1,
            (Race::Orc, BodyType::Female) => 3,
            (Race::Orc, BodyType::Male) => 5,
            (Race::Undead, BodyType::Female) => 1,
            (Race::Undead, BodyType::Male) => 1,
        }
    }

    pub fn num_beards(self, body_type: BodyType) -> u8 {
        match (self, body_type) {
            (Race::Danari, BodyType::Female) => 1,
            (Race::Danari, BodyType::Male) => 2,
            (Race::Dwarf, BodyType::Female) => 1,
            (Race::Dwarf, BodyType::Male) => 20,
            (Race::Elf, BodyType::Female) => 1,
            (Race::Elf, BodyType::Male) => 1,
            (Race::Human, BodyType::Female) => 1,
            (Race::Human, BodyType::Male) => 4,
            (Race::Orc, BodyType::Female) => 1,
            (Race::Orc, BodyType::Male) => 3,
            (Race::Undead, BodyType::Female) => 1,
            (Race::Undead, BodyType::Male) => 1,
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
pub enum Chest {
    Blue = 0,
    Brown = 1,
    Dark = 2,
    Green = 3,
    Orange = 4,
    Midnight = 5,
    Kimono = 6,
}
pub const ALL_CHESTS: [Chest; 7] = [
    Chest::Blue,
    Chest::Brown,
    Chest::Dark,
    Chest::Green,
    Chest::Orange,
    Chest::Midnight,
    Chest::Kimono,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Belt {
    Dark = 0,
    Cloth = 1,
}
pub const ALL_BELTS: [Belt; 2] = [Belt::Dark, Belt::Cloth];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Pants {
    Blue = 0,
    Brown = 1,
    Dark = 2,
    Green = 3,
    Orange = 4,
    Kimono = 5,
}
pub const ALL_PANTS: [Pants; 6] = [
    Pants::Blue,
    Pants::Brown,
    Pants::Dark,
    Pants::Green,
    Pants::Orange,
    Pants::Kimono,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Hand {
    Bare = 0,
    Cloth = 1,
}
pub const ALL_HANDS: [Hand; 2] = [Hand::Bare, Hand::Cloth];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Foot {
    Bare = 0,
    Dark = 1,
    Sandal = 2,
    Jester = 3,
}
pub const ALL_FEET: [Foot; 4] = [Foot::Bare, Foot::Dark, Foot::Sandal, Foot::Jester];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Shoulder {
    None = 0,
    Brown1 = 1,
    Chain = 2,
}
pub const ALL_SHOULDERS: [Shoulder; 3] = [Shoulder::None, Shoulder::Brown1, Shoulder::Chain];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Eyebrows {
    Yup = 0,
}
pub const ALL_EYEBROWS: [Eyebrows; 1] = [Eyebrows::Yup];

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
    Pale = 0,
    White = 1,
    Tanned = 2,
    Brown = 3,
    TannedBrown = 4,
    TannedDarkBrown = 5,
    Iron = 6,
    Steel = 7,
    DanariOne = 8,
    DanariTwo = 9,
    DanariThree = 10,
    DanariFour = 11,
    ElfOne = 12,
    ElfTwo = 13,
    ElfThree = 14,
    OrcOne = 15,
    OrcTwo = 16,
    OrcThree = 17,
    UndeadOne = 18,
    UndeadTwo = 19,
    UndeadThree = 20,
}
impl Skin {
    pub fn rgb(self) -> Rgb<u8> {
        let color = match self {
            Self::Pale => (252, 211, 179),
            Self::White => (253, 195, 164),
            Self::Tanned => (222, 181, 151),
            Self::Brown => (123, 80, 45),
            Self::TannedBrown => (135, 70, 50),
            Self::TannedDarkBrown => (116, 61, 43),
            Self::Iron => (135, 113, 95),
            Self::Steel => (108, 94, 86),
            Self::DanariOne => (104, 168, 196),
            Self::DanariTwo => (30, 149, 201),
            Self::DanariThree => (57, 120, 148),
            Self::DanariFour => (40, 85, 105),
            Self::ElfOne => (178, 164, 186),
            Self::ElfTwo => (132, 139, 161),
            Self::ElfThree => (148, 128, 202),
            Self::OrcOne => (61, 130, 42),
            Self::OrcTwo => (82, 117, 36),
            Self::OrcThree => (71, 94, 42),
            Self::UndeadOne => (240, 243, 239),
            Self::UndeadTwo => (178, 178, 178),
            Self::UndeadThree => (145, 135, 121),
        };
        Rgb::from(color)
    }

    pub fn light_rgb(self) -> Rgb<u8> {
        let color = match self {
            Self::Pale => (255, 227, 193),
            Self::White => (255, 210, 180),
            Self::Tanned => (239, 197, 164),
            Self::Brown => (150, 104, 68),
            Self::TannedBrown => (148, 85, 64),
            Self::TannedDarkBrown => (132, 74, 56),
            Self::Iron => (144, 125, 106),
            Self::Steel => (120, 107, 99),
            Self::DanariOne => (116, 176, 208),
            Self::DanariTwo => (42, 158, 206),
            Self::DanariThree => (70, 133, 160),
            Self::DanariFour => (53, 96, 116),
            Self::ElfOne => (190, 176, 199), //178, 164, 186
            Self::ElfTwo => (137, 144, 167),
            Self::ElfThree => (156, 138, 209),
            Self::OrcOne => (83, 165, 56),
            Self::OrcTwo => (92, 132, 46),
            Self::OrcThree => (84, 110, 54),
            Self::UndeadOne => (254, 252, 251),
            Self::UndeadTwo => (190, 192, 191),
            Self::UndeadThree => (160, 151, 134),
        };
        Rgb::from(color)
    }

    pub fn dark_rgb(self) -> Rgb<u8> {
        let color = match self {
            Self::Pale => (229, 192, 163),
            Self::White => (239, 179, 150),
            Self::Tanned => (208, 167, 135),
            Self::Brown => (106, 63, 30),
            Self::TannedBrown => (122, 58, 40),
            Self::TannedDarkBrown => (100, 47, 32),
            Self::Iron => (124, 99, 82),
            Self::Steel => (96, 81, 72),
            Self::DanariOne => (92, 155, 183),
            Self::DanariTwo => (25, 142, 192),
            Self::DanariThree => (52, 115, 143),
            Self::DanariFour => (34, 80, 99),
            Self::ElfOne => (170, 155, 175), //170, 157, 179
            Self::ElfTwo => (126, 132, 153),
            Self::ElfThree => (137, 121, 194),
            Self::OrcOne => (55, 114, 36),
            Self::OrcTwo => (70, 104, 29),
            Self::OrcThree => (60, 83, 32),
            Self::UndeadOne => (229, 231, 230),
            Self::UndeadTwo => (165, 166, 164),
            Self::UndeadThree => (130, 122, 106),
        };
        Rgb::from(color)
    }
}
