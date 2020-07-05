use crate::sync::Uid;
use specs::Component;
use specs_idvs::IDVStorage;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Object {
    Bomb { owner: Option<Uid> },
}

impl Component for Object {
    type Storage = IDVStorage<Self>;
}
