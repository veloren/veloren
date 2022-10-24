use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SiteKindMeta {
    Dungeon(DungeonKindMeta),
    Cave,
    Settlement(SettlementKindMeta),
    Castle,
    Void,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DungeonKindMeta {
    Old,
    Gnarling,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SettlementKindMeta {
    Default,
    CliffTown,
    DesertCity,
    SavannahPit,
}

impl Default for SiteKindMeta {
    fn default() -> SiteKindMeta { SiteKindMeta::Void }
}
