use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Body {
    pub head: Head,
    pub upper_torso: UpperTorso,
    pub lower_torso: LowerTorso,
    pub shoulder_l: ShoulderL,
    pub shoulder_r: ShoulderR,
    pub hand_l: HandL,
    pub hand_r: HandR,
    pub feet: Feet,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
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
