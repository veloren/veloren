use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum BiomeKind {
    #[default]
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
