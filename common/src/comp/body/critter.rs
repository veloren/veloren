use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct Body {
    pub species: Species,
    pub body_type: BodyType,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        let species = *(&ALL_SPECIES).choose(&mut rng).unwrap();
        let body_type = *(&ALL_BODY_TYPES).choose(&mut rng).unwrap();
        Self { species, body_type }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Species {
    Rat = 0,
    Axolotl = 1,
    Gecko = 2,
    Turtle = 3,
    Squirrel = 4,
    Fungome = 5,
}
pub const ALL_SPECIES: [Species; 6] = [
    Species::Rat,
    Species::Axolotl,
    Species::Gecko,
    Species::Turtle,
    Species::Squirrel,
    Species::Fungome,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum BodyType {
    Female = 0,
    Male = 1,
}
pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
