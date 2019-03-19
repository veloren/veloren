// Library
use specs::{Component, VecStorage};
use vek::*;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
enum Race {
    Danari,
    Dwarf,
    Elf,
    Human,
    Orc,
    Undead,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Gender {
    Female,
    Male,
    Unspecified,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Character {
    race: Race,
    gender: Gender,
    head: (),
    chest: (),
    belt: (),
    arms: (),
    feet: (),
}

impl Component for Character {
    type Storage = VecStorage<Self>;
}
