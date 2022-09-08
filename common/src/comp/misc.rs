use super::item::Reagent;
use crate::uid::Uid;
use serde::{Deserialize, Serialize};
use specs::Component;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Object {
    Bomb {
        owner: Option<Uid>,
    },
    Firework {
        owner: Option<Uid>,
        reagent: Reagent,
    },
}

impl Component for Object {
    type Storage = specs::VecStorage<Self>;
}
