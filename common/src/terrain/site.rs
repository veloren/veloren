use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SiteKindMeta {
    Dungeon(DungeonKindMeta),
    Cave,
    Settlement(SettlementKindMeta),
    Castle,
    #[default]
    Void,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DungeonKindMeta {
    Gnarling,
    Adlet,
    Haniwa,
    SeaChapel,
    Terracotta,
    Cultist,
    Sahagin,
    Myrmidon,
    VampireCastle,
    DwarvenMine,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SettlementKindMeta {
    Default,
    CliffTown,
    DesertCity,
    SavannahPit,
    CoastalTown,
}
