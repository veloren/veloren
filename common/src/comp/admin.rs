use specs::{Component, VecStorage};

pub struct AdminPerms;

impl Component for AdminPerms {
    type Storage = VecStorage<Self>;
}
