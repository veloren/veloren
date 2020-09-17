//! Structs representing a playable Character

use crate::comp;
use serde::{Deserialize, Serialize};

/// The limit on how many characters that a player can have
pub const MAX_CHARACTERS_PER_PLAYER: usize = 8;
pub type CharacterId = i64;

/// The minimum character data we need to create a new character on the server.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub id: Option<CharacterId>,
    pub alias: String,
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
