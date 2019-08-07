use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Body {
    pub race: Race,
    pub head: Head,
    pub upper_torso: Upper_Torso,
    pub lower_torso: Lower_Torso,
    pub shoulder_l: Shoulder_L,
    pub shoulder_r: Shoulder_R,
    pub hand_l: Hand_L,
    pub hand_r: Hand_R,
    pub feet: Feet,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            race: *(&ALL_RACES).choose(&mut rng).unwrap(),
            head: *(&ALL_HEADS).choose(&mut rng).unwrap(),
            upper_torso: *(&ALL_UPPER_TORSOS).choose(&mut rng).unwrap(),
            lower_torso: *(&ALL_LOWER_TORSOS).choose(&mut rng).unwrap(),
            shoulder_l: *(&ALL_SHOULDER_L).choose(&mut rng).unwrap(),
            shoulder_r: *(&ALL_SHOULDER_R).choose(&mut rng).unwrap(),
            hand_l: *(&ALL_HANDS_L).choose(&mut rng).unwrap(),
            hand_r: *(&ALL_HANDS_R).choose(&mut rng).unwrap(),
            feet: *(&ALL_FEET).choose(&mut rng).unwrap(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Race {
    Ice,
    Earth,
    Fire,
    Rock,
    //Wind,
    //Death,
    //Storm,
}
pub const ALL_RACES: [Race; 4] = [
    Race::Ice,
    Race::Earth,
    Race::Fire,
    Race::Rock,
    //Race::Wind,
    //Race::Death,
    //Race::Storm,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Head {
    Default,
}
const ALL_HEADS: [Head; 1] = [Head::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Upper_Torso {
    Default,
}
const ALL_UPPER_TORSOS: [Upper_Torso; 1] = [Upper_Torso::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lower_Torso {
    Default,
}
const ALL_LOWER_TORSOS: [Lower_Torso; 1] = [Lower_Torso::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Shoulder_L {
    Default,
}
const ALL_SHOULDER_L: [Shoulder_L; 1] = [Shoulder_L::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Shoulder_R {
    Default,
}
const ALL_SHOULDER_R: [Shoulder_R; 1] = [Shoulder_R::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hand_L {
    Default,
}
const ALL_HANDS_L: [Hand_L; 1] = [Hand_L::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hand_R {
    Default,
}
const ALL_HANDS_R: [Hand_R; 1] = [Hand_R::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Feet {
    Default,
}
const ALL_FEET: [Feet; 1] = [Feet::Default];
