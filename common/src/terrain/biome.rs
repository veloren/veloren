use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum BiomeKind {
    Void,
    Lake,
    Grassland,
    Ocean,
    Mountain,
    Snowland,
    Desert,
    Swamp,
    Jungle,
    Forest,
    Savannah,
    Taiga,
}

impl Default for BiomeKind {
    fn default() -> BiomeKind { BiomeKind::Void }
}
