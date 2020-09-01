use crate::{make_case_elim, make_proj_elim};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

make_proj_elim!(
    body,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Body {
        pub head: Head,
        pub torso: Torso,
        pub rear: Rear,
        pub tail: Tail,
        pub fin_l: FinL,
        pub fin_r: FinR,
    }
);

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

make_case_elim!(
    head,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Head {
        Default = 0,
    }
);

const ALL_HEADS: [Head; 1] = [Head::Default];

make_case_elim!(
    torso,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Torso {
        Default = 0,
    }
);
const ALL_TORSOS: [Torso; 1] = [Torso::Default];

make_case_elim!(
    rear,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Rear {
        Default = 0,
    }
);
const ALL_REARS: [Rear; 1] = [Rear::Default];

make_case_elim!(
    tail,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Tail {
        Default = 0,
    }
);
const ALL_TAILS: [Tail; 1] = [Tail::Default];

make_case_elim!(
    fin_l,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum FinL {
        Default = 0,
    }
);
const ALL_FIN_LS: [FinL; 1] = [FinL::Default];

make_case_elim!(
    fin_r,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum FinR {
        Default = 0,
    }
);
const ALL_FIN_RS: [FinR; 1] = [FinR::Default];
