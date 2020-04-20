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

/// Represents the character data sent by the server after loading from the
/// database.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterItem {
    pub character: Character,
    pub body: comp::Body,
    pub stats: comp::Stats,
}
