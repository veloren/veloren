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
    Pig = 0,
    Fox = 1,
    Sheep = 2,
    Boar = 3,
    Jackalope = 4,
    Skunk = 5,
    Cat = 6,
    Batfox = 7,
    Raccoon = 8,
    Quokka = 9,
    Dodarock = 10,
    Holladon = 11,
}
pub const ALL_SPECIES: [Species; 12] = [
    Species::Pig,
    Species::Fox,
    Species::Sheep,
    Species::Boar,
    Species::Jackalope,
    Species::Skunk,
    Species::Cat,
    Species::Batfox,
    Species::Raccoon,
    Species::Quokka,
    Species::Dodarock,
    Species::Holladon,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum BodyType {
    Female = 0,
    Male = 1,
}
pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
