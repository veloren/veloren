use specs::{Component, VecStorage, FlaggedStorage};
use vek::*;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Race {
    Danari,
    Dwarf,
    Elf,
    Human,
    Orc,
    Undead,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Gender {
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

impl Character {
    // TODO: Remove this
    pub fn test() -> Self {
        Self {
            race: Race::Human,
            gender: Gender::Unspecified,
            head: (),
            chest: (),
            belt: (),
            arms: (),
            feet: (),
        }
    }
}

impl Component for Character {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
