use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Body {
    pub head: Head,
    pub torso: Torso,
    pub neck: Neck,
    pub leg_lf: LegLF,
    pub leg_rf: LegRF,
    pub leg_lb: LegLB,
    pub leg_rb: LegRB,
    pub foot_lf: FootLF,
    pub foot_rf: FootRF,
    pub foot_lb: FootLB,
    pub foot_rb: FootRB,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            head: *(&ALL_HEADS).choose(&mut rng).unwrap(),
            torso: *(&ALL_TORSOS).choose(&mut rng).unwrap(),
            neck: *(&ALL_NECKS).choose(&mut rng).unwrap(),
            leg_lf: *(&ALL_LEGS_LF).choose(&mut rng).unwrap(),
            leg_rf: *(&ALL_LEGS_RF).choose(&mut rng).unwrap(),
            leg_lb: *(&ALL_LEGS_LB).choose(&mut rng).unwrap(),
            leg_rb: *(&ALL_LEGS_RB).choose(&mut rng).unwrap(),
            foot_lf: *(&ALL_FEETS_LF).choose(&mut rng).unwrap(),
            foot_rf: *(&ALL_FEETS_RF).choose(&mut rng).unwrap(),
            foot_lb: *(&ALL_FEETS_LB).choose(&mut rng).unwrap(),
            foot_rb: *(&ALL_FEETS_RB).choose(&mut rng).unwrap(),
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
pub enum Neck {
    Default,
}
const ALL_NECKS: [Neck; 1] = [Neck::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegLF {
    Default,
}
const ALL_LEGS_LF: [LegLF; 1] = [LegLF::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegRF {
    Default,
}
const ALL_LEGS_RF: [LegRF; 1] = [LegRF::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegLB {
    Default,
}
const ALL_LEGS_LB: [LegLB; 1] = [LegLB::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegRB {
    Default,
}
const ALL_LEGS_RB: [LegRB; 1] = [LegRB::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FootLF {
    Default,
}
const ALL_FEETS_LF: [FootLF; 1] = [FootLF::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FootRF {
    Default,
}
const ALL_FEETS_RF: [FootRF; 1] = [FootRF::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FootLB {
    Default,
}
const ALL_FEETS_LB: [FootLB; 1] = [FootLB::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FootRB {
    Default,
}
const ALL_FEETS_RB: [FootRB; 1] = [FootRB::Default];
