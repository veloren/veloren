//! Structs representing a playable Character

use crate::comp;
use serde_derive::{Deserialize, Serialize};

/// The limit on how many characters that a player can have
pub const MAX_CHARACTERS_PER_PLAYER: usize = 8;

// TODO: Since loadout persistence came a few weeks after character persistence,
// we stored their main weapon in the `tool` field here. While loadout
// persistence is still new, saved characters may not have an associated loadout
// entry in the DB, so we use this `tool` field to create an entry the first
// time they enter the game.
//
// Once we are happy that all characters have a loadout, or we manually
// update/delete those that don't, it's no longer necessary and we can
// remove this from here, as well as in the DB schema and persistence code.

/// The minimum character data we need to create a new character on the server.
/// The `tool` field was historically used to persist the character's weapon
/// before Loadouts were persisted, and will be removed in the future.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub id: Option<i32>,
    pub alias: String,
    pub tool: Option<String>,
}

/// Data needed to render a single character item in the character list
/// presented during character selection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterItem {
    pub character: Character,
    pub body: comp::Body,
    pub level: usize,
    pub loadout: comp::Loadout,
}
