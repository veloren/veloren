use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Body {
    pub head: Head,
    pub torso: Torso,
    pub tail: Tail,
    pub wing_l: WingL,
    pub wing_r: WingR,
    pub leg_l: LegL,
    pub leg_r: LegR,
}
impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            head: *(&ALL_HEADS).choose(&mut rng).unwrap(),
            torso: *(&ALL_TORSOS).choose(&mut rng).unwrap(),
            tail: *(&ALL_TAILS).choose(&mut rng).unwrap(),
            wing_l: *(&ALL_WING_LS).choose(&mut rng).unwrap(),
            wing_r: *(&ALL_WING_RS).choose(&mut rng).unwrap(),
            leg_l: *(&ALL_LEG_LS).choose(&mut rng).unwrap(),
            leg_r: *(&ALL_LEG_RS).choose(&mut rng).unwrap(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Head {
    Default,
}
const ALL_HEADS: [Head; 1] = [Head::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Torso {
    Default,
}
const ALL_TORSOS: [Torso; 1] = [Torso::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tail {
    Default,
}
const ALL_TAILS: [Tail; 1] = [Tail::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WingL {
    Default,
}
const ALL_WING_LS: [WingL; 1] = [WingL::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WingR {
    Default,
}
const ALL_WING_RS: [WingR; 1] = [WingR::Default];

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