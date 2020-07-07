use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
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
