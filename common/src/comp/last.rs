use serde::{Deserialize, Serialize};
use specs::{Component, VecStorage};
use std::marker::Send;

#[derive(Copy, Clone, Debug, Hash, Serialize, Deserialize)]
pub struct Last<C: Component + PartialEq>(pub C);

impl<C: Component + Send + Sync + PartialEq> Component for Last<C> {
    type Storage = VecStorage<Self>;
}
