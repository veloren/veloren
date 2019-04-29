use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
