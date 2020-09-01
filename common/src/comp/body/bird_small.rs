use crate::{make_case_elim, make_proj_elim};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

make_proj_elim!(
    body,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Body {
        pub head: Head,
        pub torso: Torso,
        pub wing_l: WingL,
        pub wing_r: WingR,
    }
);

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            head: *(&ALL_HEADS).choose(&mut rng).unwrap(),
            torso: *(&ALL_TORSOS).choose(&mut rng).unwrap(),
            wing_l: *(&ALL_WING_LS).choose(&mut rng).unwrap(),
            wing_r: *(&ALL_WING_RS).choose(&mut rng).unwrap(),
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
    wing_l,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum WingL {
        Default = 0,
    }
);
const ALL_WING_LS: [WingL; 1] = [WingL::Default];

make_case_elim!(
    wing_r,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum WingR {
        Default = 0,
    }
);
const ALL_WING_RS: [WingR; 1] = [WingR::Default];
