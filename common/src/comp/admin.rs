use clap::arg_enum;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, VecStorage};

arg_enum! {
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
    pub enum AdminRole {
        Moderator = 0,
        Admin = 1,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Admin(pub AdminRole);

impl Component for Admin {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}
