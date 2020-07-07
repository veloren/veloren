use crate::sync::Uid;
use serde::{Deserialize, Serialize};
use specs::Component;
use specs_idvs::IdvStorage;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Object {
    Bomb { owner: Option<Uid> },
}

impl Component for Object {
    type Storage = IdvStorage<Self>;
}
