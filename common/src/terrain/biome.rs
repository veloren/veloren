use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum BiomeKind {
    Void,
    Grassland,
    Ocean,
    Mountain,
    Snowlands,
    Desert,
    Swamp,
    Forest,
}
