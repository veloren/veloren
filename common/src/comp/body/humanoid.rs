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
        Self::random_with(&mut rng, &race)
    }

    #[inline]
    pub fn random_with(rng: &mut impl Rng, &race: &Race) -> Self {
        let body_type = *(&ALL_BODY_TYPES).choose(rng).unwrap();
        Self {
            race,
            body_type,
            chest: *(&ALL_CHESTS).choose(rng).unwrap(),
            belt: *(&ALL_BELTS).choose(rng).unwrap(),
            pants: *(&ALL_PANTS).choose(rng).unwrap(),
            hand: *(&ALL_HANDS).choose(rng).unwrap(),
            foot: *(&ALL_FEET).choose(rng).unwrap(),
            shoulder: *(&ALL_SHOULDERS).choose(rng).unwrap(),
            hair_style: rng.gen_range(0, race.num_hair_styles(body_type)),
            beard: rng.gen_range(0, race.num_beards(body_type)),
            eyebrows: *(&ALL_EYEBROWS).choose(rng).unwrap(),
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

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::Humanoid(body) }
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

impl<'a, SpeciesMeta> core::ops::Index<&'a Race> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Race) -> &Self::Output {
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

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type Item = Race;

    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter { ALL_RACES.iter().copied() }
}

// Hair Colors
pub const DANARI_HAIR_COLORS: [(u8, u8, u8); 11] = [
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
];
pub const DWARF_HAIR_COLORS: [(u8, u8, u8); 20] = [
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
];
pub const ELF_HAIR_COLORS: [(u8, u8, u8); 23] = [
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
];
pub const HUMAN_HAIR_COLORS: [(u8, u8, u8); 21] = [
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
];
pub const ORC_HAIR_COLORS: [(u8, u8, u8); 10] = [
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
];
pub const UNDEAD_HAIR_COLORS: [(u8, u8, u8); 21] = [
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
pub const ELF_SKIN_COLORS: [Skin; 14] = [
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
    Skin::ElfThree,
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
            (Race::Danari, BodyType::Female) => 2,
            (Race::Danari, BodyType::Male) => 2,
            (Race::Dwarf, BodyType::Female) => 4,
            (Race::Dwarf, BodyType::Male) => 3,
            (Race::Elf, BodyType::Female) => 21,
            (Race::Elf, BodyType::Male) => 4,
            (Race::Human, BodyType::Female) => 19,
            (Race::Human, BodyType::Male) => 17,
            (Race::Orc, BodyType::Female) => 7,
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
            (Race::Orc, BodyType::Female) => 4,
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
    Assassin = 7,
}
pub const ALL_CHESTS: [Chest; 8] = [
    Chest::Blue,
    Chest::Brown,
    Chest::Dark,
    Chest::Green,
    Chest::Orange,
    Chest::Midnight,
    Chest::Kimono,
    Chest::Assassin,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Belt {
    Dark = 0,
    TurqCloth = 1,
    BloodCloth = 2,
    BlackCloth = 3,
    Assassin = 4,
}
pub const ALL_BELTS: [Belt; 5] = [
    Belt::Dark,
    Belt::TurqCloth,
    Belt::BloodCloth,
    Belt::BlackCloth,
    Belt::Assassin,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Pants {
    Blue = 0,
    Brown = 1,
    Dark = 2,
    Green = 3,
    Orange = 4,
    Kimono = 5,
    Assassin = 6,
}
pub const ALL_PANTS: [Pants; 7] = [
    Pants::Blue,
    Pants::Brown,
    Pants::Dark,
    Pants::Green,
    Pants::Orange,
    Pants::Kimono,
    Pants::Assassin,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Hand {
    Bare = 0,
    Cloth = 1,
    Assassin = 2,
}
pub const ALL_HANDS: [Hand; 3] = [Hand::Bare, Hand::Cloth, Hand::Assassin];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Foot {
    Bare = 0,
    Dark = 1,
    Sandal = 2,
    Jester = 3,
    Assassin = 4,
}
pub const ALL_FEET: [Foot; 5] = [
    Foot::Bare,
    Foot::Dark,
    Foot::Sandal,
    Foot::Jester,
    Foot::Assassin,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Shoulder {
    None = 0,
    Brown1 = 1,
    Chain = 2,
    Assassin = 3,
}
pub const ALL_SHOULDERS: [Shoulder; 4] = [
    Shoulder::None,
    Shoulder::Brown1,
    Shoulder::Chain,
    Shoulder::Assassin,
];

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
    ElfThree = 14,
    OrcOne = 15,
    OrcTwo = 16,
    OrcThree = 17,
    UndeadOne = 18,
    UndeadTwo = 19,
    UndeadThree = 20,
    Skin7 = 21,
    Skin8 = 22,
    Skin9 = 23,
    Skin10 = 24,
    Skin11 = 25,
    Skin12 = 26,
    Skin13 = 27,
    Skin14 = 28,
    Skin15 = 29,
    Skin16 = 30,
    Skin17 = 31,
    Skin18 = 32,
    OrcFour = 33,
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
            Self::ElfThree => (148, 128, 202),
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
            Self::ElfThree => (156, 138, 209),
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
            Self::ElfThree => (137, 121, 194),
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
