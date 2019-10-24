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
    pub leg_l: LegL,
    pub leg_r: LegR,
    pub foot_l: FootL,
    pub foot_r: FootR,
}
impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            head: *(&ALL_HEADS).choose(&mut rng).unwrap(),
            upper_torso: *(&ALL_UPPER_TORSOS).choose(&mut rng).unwrap(),
            lower_torso: *(&ALL_LOWER_TORSOS).choose(&mut rng).unwrap(),
            shoulder_l: *(&ALL_SHOULDER_LS).choose(&mut rng).unwrap(),
            shoulder_r: *(&ALL_SHOULDER_RS).choose(&mut rng).unwrap(),
            hand_l: *(&ALL_HAND_LS).choose(&mut rng).unwrap(),
            hand_r: *(&ALL_HAND_RS).choose(&mut rng).unwrap(),
            leg_l: *(&ALL_LEG_LS).choose(&mut rng).unwrap(),
            leg_r: *(&ALL_LEG_RS).choose(&mut rng).unwrap(),
            foot_l: *(&ALL_FOOT_LS).choose(&mut rng).unwrap(),
            foot_r: *(&ALL_FOOT_RS).choose(&mut rng).unwrap(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Head {
    Default,
}
const ALL_HEADS: [Head; 1] = [Head::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UpperTorso {
    Default,
}
const ALL_UPPER_TORSOS: [UpperTorso; 1] = [UpperTorso::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LowerTorso {
    Default,
}
const ALL_LOWER_TORSOS: [LowerTorso; 1] = [LowerTorso::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShoulderL {
    Default,
}
const ALL_SHOULDER_LS: [ShoulderL; 1] = [ShoulderL::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShoulderR {
    Default,
}
const ALL_SHOULDER_RS: [ShoulderR; 1] = [ShoulderR::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HandL {
    Default,
}
const ALL_HAND_LS: [HandL; 1] = [HandL::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HandR {
    Default,
}
const ALL_HAND_RS: [HandR; 1] = [HandR::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegL {
    Default,
}
const ALL_LEG_LS: [LegL; 1] = [LegL::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegR {
    Default,
}
const ALL_LEG_RS: [LegR; 1] = [LegR::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FootL {
    Default,
}
const ALL_FOOT_LS: [FootL; 1] = [FootL::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FootR {
    Default,
}
const ALL_FOOT_RS: [FootR; 1] = [FootR::Default];