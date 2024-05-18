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

impl core::str::FromStr for AdminRole {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mod" | "moderator" => Ok(AdminRole::Moderator),
            "admin" => Ok(AdminRole::Admin),
            _ => Err("Could not parse AdminRole"),
        }
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for AdminRole {
    fn to_string(&self) -> String {
        match self {
            AdminRole::Moderator => "moderator",
            AdminRole::Admin => "admin",
        }
        .into()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Admin(pub AdminRole);

impl Component for Admin {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}
