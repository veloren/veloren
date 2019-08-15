use specs::{Component, NullStorage};

#[derive(Default)]
pub struct Admin;

impl Component for Admin {
    type Storage = NullStorage<Self>;
}
