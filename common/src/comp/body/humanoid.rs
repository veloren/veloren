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
    pub hair_style: HairStyle,
    pub beard: Beard,
    pub eyebrows: Eyebrows,
    pub accessory: Accessory,
    pub hair_color: u8,
    pub skin: u8,
    pub eye_color: u8,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        let race = *(&ALL_RACES).choose(&mut rng).unwrap();
        Self {
            race,
            body_type: *(&ALL_BODY_TYPES).choose(&mut rng).unwrap(),
            chest: *(&ALL_CHESTS).choose(&mut rng).unwrap(),
            belt: *(&ALL_BELTS).choose(&mut rng).unwrap(),
            pants: *(&ALL_PANTS).choose(&mut rng).unwrap(),
            hand: *(&ALL_HANDS).choose(&mut rng).unwrap(),
            foot: *(&ALL_FEET).choose(&mut rng).unwrap(),
            shoulder: *(&ALL_SHOULDERS).choose(&mut rng).unwrap(),
            hair_style: *(&ALL_HAIR_STYLES).choose(&mut rng).unwrap(),
            beard: *(&ALL_BEARDS).choose(&mut rng).unwrap(),
            eyebrows: *(&ALL_EYEBROWS).choose(&mut rng).unwrap(),
            accessory: *(&ALL_ACCESSORIES).choose(&mut rng).unwrap(),
            hair_color: rng.gen_range(0, race.num_hair_colors()) as u8,
            skin: rng.gen_range(0, race.num_skin_colors()) as u8,
            eye_color: rng.gen_range(0, race.num_eye_colors()) as u8,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Race {
    Danari,
    Dwarf,
    Elf,
    Human,
    Orc,
    Undead,
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
pub const DANARI_HAIR_COLORS: [(u8, u8, u8); 4] = [
    (198, 169, 113),
    (146, 32, 32),
    (199, 131, 58),
    (107, 32, 60),
];
pub const DWARF_HAIR_COLORS: [(u8, u8, u8); 3] = [(126, 26, 26), (54, 46, 38), (99, 75, 49)];
pub const ELF_HAIR_COLORS: [(u8, u8, u8); 3] = [(66, 83, 113), (13, 76, 41), (189, 185, 126)];
pub const HUMAN_HAIR_COLORS: [(u8, u8, u8); 3] = [(107, 76, 51), (161, 63, 18), (64, 32, 18)];
pub const ORC_HAIR_COLORS: [(u8, u8, u8); 3] = [(66, 66, 59), (54, 30, 26), (125, 111, 51)];
pub const UNDEAD_HAIR_COLORS: [(u8, u8, u8); 3] = [(0, 131, 122), (66, 66, 59), (111, 54, 117)];

// Skin colors
pub const DANARI_SKIN_COLORS: [(u8, u8, u8); 4] = [
    (104, 168, 196),
    (30, 149, 201),
    (57, 120, 148),
    (40, 85, 105),
];
pub const DWARF_SKIN_COLORS: [(u8, u8, u8); 3] = [(215, 175, 123), (191, 125, 94), (212, 128, 89)];
pub const ELF_SKIN_COLORS: [(u8, u8, u8); 3] = [(176, 161, 181), (132, 139, 161), (138, 119, 201)];
pub const HUMAN_SKIN_COLORS: [(u8, u8, u8); 3] =
    [(255, 200, 159), (186, 140, 104), (87, 57, 34)];
pub const ORC_SKIN_COLORS: [(u8, u8, u8); 3] = [(77, 150, 51), (82, 117, 36), (71, 94, 42)];
pub const UNDEAD_SKIN_COLORS: [(u8, u8, u8); 3] =
    [(255, 255, 255), (178, 178, 178), (145, 135, 121)];

// Eye colors
pub const DANARI_EYE_COLORS: [EyeColor; 7] = [
    EyeColor::Black,
    EyeColor::Blue,
    EyeColor::Green,
    EyeColor::Brown,
    EyeColor::Red,
    EyeColor::White,
    EyeColor::Orange,
];
pub const DWARF_EYE_COLORS: [EyeColor; 7] = [
    EyeColor::Black,
    EyeColor::Blue,
    EyeColor::Green,
    EyeColor::Brown,
    EyeColor::Red,
    EyeColor::White,
    EyeColor::Orange,
];
pub const ELF_EYE_COLORS: [EyeColor; 7] = [
    EyeColor::Black,
    EyeColor::Blue,
    EyeColor::Green,
    EyeColor::Brown,
    EyeColor::Red,
    EyeColor::White,
    EyeColor::Orange,
];
pub const HUMAN_EYE_COLORS: [EyeColor; 7] = [
    EyeColor::Black,
    EyeColor::Blue,
    EyeColor::Green,
    EyeColor::Brown,
    EyeColor::Red,
    EyeColor::White,
    EyeColor::Orange,
];
pub const ORC_EYE_COLORS: [EyeColor; 7] = [
    EyeColor::Black,
    EyeColor::Blue,
    EyeColor::Green,
    EyeColor::Brown,
    EyeColor::Red,
    EyeColor::White,
    EyeColor::Orange,
];
pub const UNDEAD_EYE_COLORS: [EyeColor; 7] = [
    EyeColor::Black,
    EyeColor::Blue,
    EyeColor::Green,
    EyeColor::Brown,
    EyeColor::Red,
    EyeColor::White,
    EyeColor::Orange,
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
    fn skin_colors(self) -> &'static [(u8, u8, u8)] {
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
    pub fn num_hair_colors(self) -> usize {
        self.hair_colors().len()
    }
    pub fn skin_color(self, val: u8) -> Rgb<u8> {
        self.skin_colors()
            .get(val as usize)
            .copied()
            .unwrap_or((0, 0, 0))
            .into()
    }
    pub fn num_skin_colors(self) -> usize {
        self.skin_colors().len()
    }
    pub fn eye_color(self, val: u8) -> EyeColor {
        self.eye_colors()
            .get(val as usize)
            .copied()
            .unwrap_or(EyeColor::Blue)
    }
    pub fn num_eye_colors(self) -> usize {
        self.eye_colors().len()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyType {
    Female,
    Male,
}
pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chest {
    Blue,
    Brown,
    Dark,
    Green,
    Orange,
}
pub const ALL_CHESTS: [Chest; 5] = [
    Chest::Blue,
    Chest::Brown,
    Chest::Dark,
    Chest::Green,
    Chest::Orange,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Belt {
    Dark,
}
pub const ALL_BELTS: [Belt; 1] = [
    //Belt::Default,
    Belt::Dark,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pants {
    Blue,
    Brown,
    Dark,
    Green,
    Orange,
}
pub const ALL_PANTS: [Pants; 5] = [
    Pants::Blue,
    Pants::Brown,
    Pants::Dark,
    Pants::Green,
    Pants::Orange,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hand {
    Default,
}
pub const ALL_HANDS: [Hand; 1] = [Hand::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Foot {
    Dark,
}
pub const ALL_FEET: [Foot; 1] = [Foot::Dark];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Shoulder {
    None,
    Brown1,
}
pub const ALL_SHOULDERS: [Shoulder; 2] = [Shoulder::None, Shoulder::Brown1];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HairStyle {
    None,
    Temp1,
    Temp2,
}
pub const ALL_HAIR_STYLES: [HairStyle; 3] = [HairStyle::None, HairStyle::Temp1, HairStyle::Temp2];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Eyebrows {
    Yup,
}
pub const ALL_EYEBROWS: [Eyebrows; 1] = [Eyebrows::Yup];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EyeColor {
    Black,
    Blue,
    Green,
    Brown,
    Red,
    Orange,
    White,
}
pub const ALL_EYE_COLORS: [EyeColor; 7] = [
    EyeColor::Black,
    EyeColor::Blue,
    EyeColor::Green,
    EyeColor::Brown,
    EyeColor::Red,
    EyeColor::White,
    EyeColor::Orange,
];
impl EyeColor {
    pub fn light_rgb(self) -> Rgb<u8> {
        match self {
            EyeColor::Black => Rgb::new(71, 59, 49),
            EyeColor::Blue => Rgb::new(75, 158, 191),
            EyeColor::Green => Rgb::new(110, 167, 113),
            EyeColor::Brown => Rgb::new(73, 42, 36),
            EyeColor::Red => Rgb::new(182, 0, 0),
            EyeColor::White => Rgb::new(255, 255, 255),
            EyeColor::Orange => Rgb::new(161, 69, 0),
        }
    }
    pub fn dark_rgb(self) -> Rgb<u8> {
        match self {
            EyeColor::Black => Rgb::new(32, 32, 32),
            EyeColor::Blue => Rgb::new(62, 130, 159),
            EyeColor::Green => Rgb::new(81, 124, 84),
            EyeColor::Brown => Rgb::new(54, 30, 26),
            EyeColor::Red => Rgb::new(148, 0, 0),
            EyeColor::White => Rgb::new(255, 255, 255),
            EyeColor::Orange => Rgb::new(148, 64, 0),
        }
    }
    pub fn white_rgb(self) -> Rgb<u8> {
        match self {
            EyeColor::White => Rgb::new(0, 0, 0),
            _ => Rgb::new(255, 255, 255),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Accessory {
    Nothing,
    Something,
}
pub const ALL_ACCESSORIES: [Accessory; 2] = [Accessory::Nothing, Accessory::Something];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Beard {
    None,
    Some,
}
pub const ALL_BEARDS: [Beard; 2] = [Beard::None, Beard::Some];
