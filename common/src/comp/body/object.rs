use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Body {
    Bomb,
    Scarecrow,
    Chest,
    Pumpkin,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        *(&ALL_OBJECTS).choose(&mut rng).unwrap()
    }
}

const ALL_OBJECTS: [Body; 4] = [Body::Bomb, Body::Scarecrow, Body::Chest, Body::Pumpkin];
