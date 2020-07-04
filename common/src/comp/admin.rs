use specs::{Component, NullStorage};
use std::ops::Deref;

#[derive(Clone, Copy, Default)]
pub struct Admin;

impl Component for Admin {
    type Storage = NullStorage<Self>;
}

/// List of admin usernames. This is stored as a specs resource so that the list
/// can be read by specs systems.
pub struct AdminList(pub Vec<String>);
impl Deref for AdminList {
    type Target = Vec<String>;

    fn deref(&self) -> &Vec<String> { &self.0 }
}
