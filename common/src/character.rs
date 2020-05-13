use crate::comp;
use serde_derive::{Deserialize, Serialize};

/// The limit on how many characters that a player can have
pub const MAX_CHARACTERS_PER_PLAYER: usize = 8;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Character {
    pub id: Option<i32>,
    pub alias: String,
    pub tool: Option<String>, // TODO: Remove once we start persisting inventories
}

/// Represents a single character item in the character list presented during
/// character selection. This is a subset of the full character data used for
/// presentation purposes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterItem {
    pub character: Character,
    pub body: comp::Body,
    pub level: usize,
}

/// The full representation of the data we store in the database for each
/// character
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterData {
    pub body: comp::Body,
    pub stats: comp::Stats,
}
