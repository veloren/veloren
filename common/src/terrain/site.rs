use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum SitesKind {
    Dungeon,
    Cave,
    Settlement,
    Castle,
    Void,
}
