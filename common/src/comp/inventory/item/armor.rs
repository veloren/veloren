#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Chest {
    Blue = 1,
    Brown = 2,
    Dark = 3,
    Green = 4,
    Orange = 5,
    Midnight = 6,
    Kimono = 7,
    Assassin = 8,
    PlateGreen0 = 9,
    Leather0 = 10,
    ClothPurple0 = 11,
    ClothBlue0 = 12,
    ClothGreen0 = 13,
    Rugged0 = 14,
    WorkerGreen0 = 15,
    WorkerGreen1 = 16,
    WorkerRed0 = 17,
    WorkerRed1 = 18,
    WorkerPurple0 = 19,
    WorkerPurple1 = 20,
    WorkerYellow0 = 21,
    WorkerYellow1 = 22,
    WorkerOrange0 = 23,
    WorkerOrange1 = 24,
    CultistPurple = 25,
    CultistBlue = 26,
    Steel0 = 27,
    Leather2 = 28,
}
pub const ALL_CHESTS: [Chest; 28] = [
    Chest::Blue,
    Chest::Brown,
    Chest::Dark,
    Chest::Green,
    Chest::Orange,
    Chest::Midnight,
    Chest::Kimono,
    Chest::Assassin,
    Chest::PlateGreen0,
    Chest::Leather0,
    Chest::ClothPurple0,
    Chest::ClothBlue0,
    Chest::ClothGreen0,
    Chest::Rugged0,
    Chest::WorkerGreen0,
    Chest::WorkerGreen1,
    Chest::WorkerRed0,
    Chest::WorkerRed1,
    Chest::WorkerPurple0,
    Chest::WorkerPurple1,
    Chest::WorkerYellow0,
    Chest::WorkerYellow1,
    Chest::WorkerOrange0,
    Chest::WorkerOrange1,
    Chest::CultistPurple,
    Chest::CultistBlue,
    Chest::Steel0,
    Chest::Leather2,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Belt {
    None = 0,
    Dark = 1,
    TurqCloth = 2,
    BloodCloth = 3,
    BlackCloth = 4,
    Assassin = 5,
    Plate0 = 6,
    Leather0 = 7,
    ClothPurple0 = 8,
    ClothBlue0 = 9,
    ClothGreen0 = 10,
    Cultist = 11,
    Leather2 = 12,
    Steel0 = 13,
}

pub const ALL_BELTS: [Belt; 14] = [
    Belt::None,
    Belt::Dark,
    Belt::TurqCloth,
    Belt::BloodCloth,
    Belt::BlackCloth,
    Belt::Assassin,
    Belt::Plate0,
    Belt::Leather0,
    Belt::ClothPurple0,
    Belt::ClothBlue0,
    Belt::ClothGreen0,
    Belt::Cultist,
    Belt::Leather2,
    Belt::Steel0,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Pants {
    None = 0,
    Blue = 1,
    Brown = 2,
    Dark = 3,
    Green = 4,
    Orange = 5,
    Kimono = 6,
    Assassin = 7,
    PlateGreen0 = 8,
    Leather0 = 9,
    ClothPurple0 = 10,
    ClothBlue0 = 11,
    ClothGreen0 = 12,
    Rugged0 = 13,
    WorkerBlue0 = 14,
    CultistPurple = 15,
    CultistBlue = 16,
    Steel0 = 17,
    Leather2 = 18,
}
pub const ALL_PANTS: [Pants; 19] = [
    Pants::None,
    Pants::Blue,
    Pants::Brown,
    Pants::Dark,
    Pants::Green,
    Pants::Orange,
    Pants::Kimono,
    Pants::Assassin,
    Pants::PlateGreen0,
    Pants::Leather0,
    Pants::ClothPurple0,
    Pants::ClothBlue0,
    Pants::ClothGreen0,
    Pants::Rugged0,
    Pants::WorkerBlue0,
    Pants::CultistPurple,
    Pants::CultistBlue,
    Pants::Steel0,
    Pants::Leather2,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Hand {
    Cloth = 1,
    Assassin = 2,
    Plate0 = 3,
    Leather0 = 4,
    ClothPurple0 = 5,
    ClothBlue0 = 6,
    ClothGreen0 = 7,
    CultistPurple = 8,
    CultistBlue = 9,
    Steel0 = 10,
    Leather2 = 11,
}
pub const ALL_HANDS: [Hand; 11] = [
    Hand::Cloth,
    Hand::Assassin,
    Hand::Plate0,
    Hand::Leather0,
    Hand::ClothPurple0,
    Hand::ClothBlue0,
    Hand::ClothGreen0,
    Hand::CultistPurple,
    Hand::CultistBlue,
    Hand::Steel0,
    Hand::Leather2,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Foot {
    Dark = 1,
    Sandal0 = 2,
    Jester = 3,
    Assassin = 4,
    Plate0 = 5,
    Leather0 = 6,
    ClothPurple0 = 7,
    ClothBlue0 = 8,
    ClothGreen0 = 9,
    Cultist = 10,
    Steel0 = 11,
    Leather2 = 12,
    JackalopeSlips = 13,
}

pub const ALL_FEET: [Foot; 13] = [
    Foot::Dark,
    Foot::Sandal0,
    Foot::Jester,
    Foot::Assassin,
    Foot::Plate0,
    Foot::Leather0,
    Foot::ClothPurple0,
    Foot::ClothBlue0,
    Foot::ClothGreen0,
    Foot::Cultist,
    Foot::Steel0,
    Foot::Leather2,
    Foot::JackalopeSlips,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Shoulder {
    Brown1 = 1,
    Chain = 2,
    Assassin = 3,
    Plate0 = 4,
    Leather0 = 5,
    Leather1 = 6,
    ClothPurple0 = 7,
    ClothBlue0 = 8,
    ClothGreen0 = 9,
    CultistPurple = 10,
    CultistBlue = 11,
    Steel0 = 12,
    Leather2 = 13,
}
pub const ALL_SHOULDERS: [Shoulder; 13] = [
    Shoulder::Brown1,
    Shoulder::Chain,
    Shoulder::Assassin,
    Shoulder::Plate0,
    Shoulder::Leather0,
    Shoulder::Leather1,
    Shoulder::ClothPurple0,
    Shoulder::ClothBlue0,
    Shoulder::ClothGreen0,
    Shoulder::CultistPurple,
    Shoulder::CultistBlue,
    Shoulder::Steel0,
    Shoulder::Leather2,
];
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Back {
    Short0 = 1,
    Admin = 2,
}
pub const ALL_BACKS: [Back; 2] = [Back::Short0, Back::Admin];
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Ring {
    Ring0 = 1,
}
pub const ALL_RINGS: [Ring; 1] = [Ring::Ring0];
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Neck {
    Neck0 = 1,
}
pub const ALL_NECKS: [Neck; 1] = [Neck::Neck0];
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Head {
    Leather0 = 1,
    AssaMask0 = 2,
}
pub const ALL_HEADS: [Head; 2] = [Head::Leather0, Head::AssaMask0];
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Tabard {
    Admin = 1,
}
pub const ALL_TABARDS: [Tabard; 1] = [Tabard::Admin];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Armor {
    Shoulder(Shoulder),
    Chest(Chest),
    Belt(Belt),
    Hand(Hand),
    Pants(Pants),
    Foot(Foot),
    Back(Back),
    Ring(Ring),
    Neck(Neck),
    Head(Head),
    Tabard(Tabard),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stats(pub u32);
