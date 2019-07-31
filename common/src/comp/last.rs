use specs::{Component, VecStorage};
use std::{fmt::Debug, marker::Send, ops::Deref};

#[derive(Copy, Clone, Debug, Hash, Serialize, Deserialize)]
pub struct Last<C: Component + PartialEq>(pub C);

impl<C: Component + Send + Sync + PartialEq> Component for Last<C> {
    type Storage = VecStorage<Self>;
}

impl<C: Component + PartialEq> PartialEq<C> for Last<C> {
    fn eq(&self, other: &C) -> bool {
        self.0 == *other
    }
}
