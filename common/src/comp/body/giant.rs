use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct Body {
    pub head: Head,
    pub shoulder: Shoulder,
    pub chest: Chest,
    pub hand: Hand,
    pub belt: Belt,
    pub pants: Pants,
    pub foot: Foot,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            head: *(&ALL_HEADS).choose(&mut rng).unwrap(),
            shoulder: *(&ALL_SHOULDERS).choose(&mut rng).unwrap(),
            chest: *(&ALL_CHESTS).choose(&mut rng).unwrap(),
            hand: *(&ALL_HANDS).choose(&mut rng).unwrap(),
            belt: *(&ALL_BELTS).choose(&mut rng).unwrap(),
            pants: *(&ALL_PANTS).choose(&mut rng).unwrap(),
            foot: *(&ALL_FEET).choose(&mut rng).unwrap(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Head {
    Default,
}
const ALL_HEADS: [Head; 1] = [Head::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Shoulder {
    Default,
}
const ALL_SHOULDERS: [Shoulder; 1] = [Shoulder::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Chest {
    Default,
}
const ALL_CHESTS: [Chest; 1] = [Chest::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Hand {
    Default,
}
const ALL_HANDS: [Hand; 1] = [Hand::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Belt {
    Default,
}
const ALL_BELTS: [Belt; 1] = [Belt::Default];
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Pants {
    Default,
}
const ALL_FEET: [Foot; 1] = [Foot::Default];
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Foot {
    Default,
}



