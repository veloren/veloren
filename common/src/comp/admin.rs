use specs::{Component, NullStorage};

#[derive(Clone, Copy, Default)]
pub struct Admin;

impl Component for Admin {
    type Storage = NullStorage<Self>;
}
