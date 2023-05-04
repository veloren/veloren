use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, VecStorage};
use strum::EnumVariantNames;

// EnumVariantNames is used by bins for clap only, but using strum here gets rid
// of the clap dependency
#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, EnumVariantNames,
)]
pub enum AdminRole {
    Moderator = 0,
    Admin = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Admin(pub AdminRole);

impl Component for Admin {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}
