use crate::sync::Uid;
use specs::Component;
use specs_idvs::IDVStorage;
use std::time::Duration;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Object {
    Bomb {
        timeout: Duration,
        owner: Option<Uid>,
    },
}

impl Component for Object {
    type Storage = IDVStorage<Self>;
}
