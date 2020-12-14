use super::item::Reagent;
use crate::uid::Uid;
use serde::{Deserialize, Serialize};
use specs::Component;
use specs_idvs::IdvStorage;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    type Storage = IdvStorage<Self>;
}
