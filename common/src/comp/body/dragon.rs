use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Body {
    pub head: Head,
    pub chest_front: ChestFront,
    pub chest_rear: ChestRear,
    pub tail_front: TailFront,
    pub tail_rear: TailRear,
    pub wing_in_l: WingInL,
    pub wing_in_r: WingInR,
    pub wing_out_l: WingOutL,
    pub wing_out_r: WingOutR,
    pub foot_fl: FootFL,
    pub foot_fr: FootFR,
    pub foot_bl: FootBL,
    pub foot_br: FootBR,
}
impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            head: *(&ALL_HEADS).choose(&mut rng).unwrap(),
            chest_front: *(&ALL_CHEST_FRONTS).choose(&mut rng).unwrap(),
            chest_rear: *(&ALL_CHEST_REARS).choose(&mut rng).unwrap(),
            tail_front: *(&ALL_TAIL_FRONTS).choose(&mut rng).unwrap(),
            tail_rear: *(&ALL_TAIL_REARS).choose(&mut rng).unwrap(),
            wing_in_l: *(&ALL_WING_IN_LS).choose(&mut rng).unwrap(),
            wing_in_r: *(&ALL_WING_IN_RS).choose(&mut rng).unwrap(),
            wing_out_l: *(&ALL_WING_OUT_LS).choose(&mut rng).unwrap(),
            wing_out_r: *(&ALL_WING_OUT_RS).choose(&mut rng).unwrap(),
            foot_fl: *(&ALL_FOOT_FLS).choose(&mut rng).unwrap(),
            foot_fr: *(&ALL_FOOT_FRS).choose(&mut rng).unwrap(),
            foot_bl: *(&ALL_FOOT_BLS).choose(&mut rng).unwrap(),
            foot_br: *(&ALL_FOOT_BRS).choose(&mut rng).unwrap(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Head {
    Default,
}
const ALL_HEADS: [Head; 1] = [Head::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChestFront {
    Default,
}
const ALL_CHEST_FRONTS: [ChestFront; 1] = [ChestFront::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChestRear {
    Default,
}
const ALL_CHEST_REARS: [ChestRear; 1] = [ChestRear::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TailFront {
    Default,
}
const ALL_TAIL_FRONTS: [TailFront; 1] = [TailFront::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TailRear {
    Default,
}
const ALL_TAIL_REARS: [TailRear; 1] = [TailRear::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WingInL {
    Default,
}
const ALL_WING_IN_LS: [WingInL; 1] = [WingInL::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WingInR {
    Default,
}
const ALL_WING_IN_RS: [WingInR; 1] = [WingInR::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WingOutL {
    Default,
}
const ALL_WING_OUT_LS: [WingOutL; 1] = [WingOutL::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WingOutR {
    Default,
}
const ALL_WING_OUT_RS: [WingOutR; 1] = [WingOutR::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FootFL {
    Default,
}
const ALL_FOOT_FLS: [FootFL; 1] = [FootFL::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FootFR {
    Default,
}
const ALL_FOOT_FRS: [FootFR; 1] = [FootFR::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FootBL {
    Default,
}
const ALL_FOOT_BLS: [FootBL; 1] = [FootBL::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FootBR {
    Default,
}
const ALL_FOOT_BRS: [FootBR; 1] = [FootBR::Default];
