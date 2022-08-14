use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum SiteKindMeta {
    Dungeon(DungeonKindMeta),
    Cave,
    Settlement(SettlementKindMeta),
    Castle,
    Void,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum DungeonKindMeta {
    Old,
    Gnarling,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum SettlementKindMeta {
    Default,
    Cliff,
    Desert,
}

impl Default for SiteKindMeta {
    fn default() -> SiteKindMeta { SiteKindMeta::Void }
}
