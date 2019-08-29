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
    (200, 100, 100),
    (100, 100, 200),
    (100, 200, 100),
];
pub const DWARF_HAIR_COLORS: [(u8, u8, u8); 3] =
    [(200, 100, 100), (100, 100, 200), (100, 200, 100)];
pub const ELF_HAIR_COLORS: [(u8, u8, u8); 3] = [(200, 100, 100), (100, 100, 200), (100, 200, 100)];
pub const HUMAN_HAIR_COLORS: [(u8, u8, u8); 3] =
    [(200, 100, 100), (100, 100, 200), (100, 200, 100)];
pub const ORC_HAIR_COLORS: [(u8, u8, u8); 3] = [(200, 100, 100), (100, 100, 200), (100, 200, 100)];
pub const UNDEAD_HAIR_COLORS: [(u8, u8, u8); 3] =
    [(200, 100, 100), (100, 100, 200), (100, 200, 100)];
// Skin colors
pub const DANARI_SKIN_COLORS: [(u8, u8, u8); 4] = [
    (198, 169, 113),
    (200, 100, 100),
    (100, 100, 200),
    (100, 200, 100),
];
pub const DWARF_SKIN_COLORS: [(u8, u8, u8); 3] =
    [(200, 100, 100), (100, 100, 200), (100, 200, 100)];
pub const ELF_SKIN_COLORS: [(u8, u8, u8); 3] = [(200, 100, 100), (100, 100, 200), (100, 200, 100)];
pub const HUMAN_SKIN_COLORS: [(u8, u8, u8); 3] =
    [(200, 100, 100), (100, 100, 200), (100, 200, 100)];
pub const ORC_SKIN_COLORS: [(u8, u8, u8); 3] = [(200, 100, 100), (100, 100, 200), (100, 200, 100)];
pub const UNDEAD_SKIN_COLORS: [(u8, u8, u8); 3] =
    [(200, 100, 100), (100, 100, 200), (100, 200, 100)];
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
            _ => &ALL_EYE_COLORS,
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
            .unwrap_or(EyeColor::Black)
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
    White,
}
pub const ALL_EYE_COLORS: [EyeColor; 6] = [
    EyeColor::Black,
    EyeColor::Blue,
    EyeColor::Green,
    EyeColor::Brown,
    EyeColor::Red,
    EyeColor::White,
];
impl EyeColor {
    pub fn light_rgb(self) -> Rgb<u8> {
        match self {
            EyeColor::Black => Rgb::new(0, 0, 0),
            EyeColor::Blue => Rgb::new(0, 0, 200),
            EyeColor::Green => Rgb::new(0, 200, 0),
            EyeColor::Brown => Rgb::new(150, 150, 0),
            EyeColor::Red => Rgb::new(255, 0, 0),
            EyeColor::White => Rgb::new(255, 255, 255),
        }
    }
    pub fn dark_rgb(self) -> Rgb<u8> {
        match self {
            EyeColor::Black => Rgb::new(0, 0, 0),
            EyeColor::Blue => Rgb::new(0, 0, 100),
            EyeColor::Green => Rgb::new(0, 100, 0),
            EyeColor::Brown => Rgb::new(50, 50, 0),
            EyeColor::Red => Rgb::new(200, 0, 0),
            EyeColor::White => Rgb::new(255, 255, 255),
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
