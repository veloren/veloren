use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct Body {
    pub head: Head,
    pub torso: Torso,
    pub rear: Rear,
    pub tail: Tail,
    pub fin_l: FinL,
    pub fin_r: FinR,
}
impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            head: *(&ALL_HEADS).choose(&mut rng).unwrap(),
            torso: *(&ALL_TORSOS).choose(&mut rng).unwrap(),
            rear: *(&ALL_REARS).choose(&mut rng).unwrap(),
            tail: *(&ALL_TAILS).choose(&mut rng).unwrap(),
            fin_l: *(&ALL_FIN_LS).choose(&mut rng).unwrap(),
            fin_r: *(&ALL_FIN_RS).choose(&mut rng).unwrap(),
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
pub enum Torso {
    Default,
}
const ALL_TORSOS: [Torso; 1] = [Torso::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Rear {
    Default,
}
const ALL_REARS: [Rear; 1] = [Rear::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Tail {
    Default,
}
const ALL_TAILS: [Tail; 1] = [Tail::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum FinL {
    Default,
}
const ALL_FIN_LS: [FinL; 1] = [FinL::Default];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum FinR {
    Default,
}
const ALL_FIN_RS: [FinR; 1] = [FinR::Default];
