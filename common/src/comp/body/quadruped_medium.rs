use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct Body {
    pub head_upper: HeadUpper,
    pub jaw: Jaw,
    pub head_lower: HeadLower,
    pub tail: Tail,
    pub torso_back: TorsoBack,
    pub torso_mid: TorsoMid,
    pub ears: Ears,
    pub foot_lf: FootLF,
    pub foot_rf: FootRF,
    pub foot_lb: FootLB,
    pub foot_rb: FootRB,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            head_upper: *(&ALL_HEADS_UPPER).choose(&mut rng).unwrap(),
            jaw: *(&ALL_JAWS).choose(&mut rng).unwrap(),
            head_lower: *(&ALL_HEADS_LOWER).choose(&mut rng).unwrap(),
            tail: *(&ALL_TAILS).choose(&mut rng).unwrap(),
            torso_back: *(&ALL_TORSOS_BACK).choose(&mut rng).unwrap(),
            torso_mid: *(&ALL_TORSOS_MID).choose(&mut rng).unwrap(),
            ears: *(&ALL_EARS).choose(&mut rng).unwrap(),
            foot_lf: *(&ALL_FEETS_LF).choose(&mut rng).unwrap(),
            foot_rf: *(&ALL_FEETS_RF).choose(&mut rng).unwrap(),
            foot_lb: *(&ALL_FEETS_LB).choose(&mut rng).unwrap(),
            foot_rb: *(&ALL_FEETS_RB).choose(&mut rng).unwrap(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum HeadUpper {
    Default,
}
const ALL_HEADS_UPPER: [HeadUpper; 1] = [HeadUpper::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Jaw {
    Default,
}
const ALL_JAWS: [Jaw; 1] = [Jaw::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum HeadLower {
    Default,
}
const ALL_HEADS_LOWER: [HeadLower; 1] = [HeadLower::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Tail {
    Default,
}
const ALL_TAILS: [Tail; 1] = [Tail::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum TorsoBack {
    Default,
}
const ALL_TORSOS_BACK: [TorsoBack; 1] = [TorsoBack::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum TorsoMid {
    Default,
}
const ALL_TORSOS_MID: [TorsoMid; 1] = [TorsoMid::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Ears {
    Default,
}
const ALL_EARS: [Ears; 1] = [Ears::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum FootLF {
    Default,
}
const ALL_FEETS_LF: [FootLF; 1] = [FootLF::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum FootRF {
    Default,
}
const ALL_FEETS_RF: [FootRF; 1] = [FootRF::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum FootLB {
    Default,
}
const ALL_FEETS_LB: [FootLB; 1] = [FootLB::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum FootRB {
    Default,
}
const ALL_FEETS_RB: [FootRB; 1] = [FootRB::Default];
