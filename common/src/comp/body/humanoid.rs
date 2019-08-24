use rand::{seq::SliceRandom, thread_rng};

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
    pub hair_color: HairColor,
    pub hair_style: HairStyle,
    pub beard: Beard,
    pub skin: Skin,
    pub eyebrows: Eyebrows,
    pub eye_color: EyeColor,
    pub accessory: Accessory,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            race: *(&ALL_RACES).choose(&mut rng).unwrap(),
            body_type: *(&ALL_BODY_TYPES).choose(&mut rng).unwrap(),
            chest: *(&ALL_CHESTS).choose(&mut rng).unwrap(),
            belt: *(&ALL_BELTS).choose(&mut rng).unwrap(),
            pants: *(&ALL_PANTS).choose(&mut rng).unwrap(),
            hand: *(&ALL_HANDS).choose(&mut rng).unwrap(),
            foot: *(&ALL_FEET).choose(&mut rng).unwrap(),
            shoulder: *(&ALL_SHOULDERS).choose(&mut rng).unwrap(),
            hair_color: *(&ALL_HAIR_COLORS).choose(&mut rng).unwrap(),
            hair_style: *(&ALL_HAIR_STYLES).choose(&mut rng).unwrap(),
            beard: *(&ALL_BEARDS).choose(&mut rng).unwrap(),
            skin: *(&ALL_SKINS).choose(&mut rng).unwrap(),
            eyebrows: *(&ALL_EYEBROWS).choose(&mut rng).unwrap(),
            eye_color: *(&ALL_EYE_COLORS).choose(&mut rng).unwrap(),
            accessory: *(&ALL_ACCESSORIES).choose(&mut rng).unwrap(),
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
pub enum HairColor {
    Red,
    Green,
    Blue,
    Brown,
    Black,
}
pub const ALL_HAIR_COLORS: [HairColor; 5] = [
    HairColor::Red,
    HairColor::Green,
    HairColor::Blue,
    HairColor::Brown,
    HairColor::Black,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Skin {
    Light,
    Medium,
    Dark,
    Rainbow,
}
pub const ALL_SKINS: [Skin; 4] = [Skin::Light, Skin::Medium, Skin::Dark, Skin::Rainbow];

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
