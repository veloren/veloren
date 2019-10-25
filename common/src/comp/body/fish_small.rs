use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Body {
    pub torso: Torso,
    pub tail: Tail,
}
impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        Self {
            torso: *(&ALL_TORSOS).choose(&mut rng).unwrap(),
            tail: *(&ALL_TAILS).choose(&mut rng).unwrap(),
        }
    }
}

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
