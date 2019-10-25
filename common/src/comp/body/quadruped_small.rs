use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Body {
    pub head: Head,
    pub chest: Chest,
    pub leg_l: LegL,
    pub leg_r: LegR,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            head: *(&ALL_HEADS).choose(&mut rng).unwrap(),
            chest: *(&ALL_CHESTS).choose(&mut rng).unwrap(),
            leg_l: *(&ALL_LEGS_L).choose(&mut rng).unwrap(),
            leg_r: *(&ALL_LEGS_R).choose(&mut rng).unwrap(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Head {
    Default,
}
const ALL_HEADS: [Head; 1] = [Head::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chest {
    Default,
}
const ALL_CHESTS: [Chest; 1] = [Chest::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegL {
    Default,
}
const ALL_LEGS_L: [LegL; 1] = [LegL::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegR {
    Default,
}
const ALL_LEGS_R: [LegR; 1] = [LegR::Default];
