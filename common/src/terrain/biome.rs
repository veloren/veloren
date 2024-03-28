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

impl BiomeKind {
    /// Roughly represents the difficulty of a biome (value between 1 and 5)
    pub fn difficulty(&self) -> i32 {
        match self {
            BiomeKind::Void => 1,
            BiomeKind::Lake => 1,
            BiomeKind::Grassland => 2,
            BiomeKind::Ocean => 1,
            BiomeKind::Mountain => 1,
            BiomeKind::Snowland => 2,
            BiomeKind::Desert => 5,
            BiomeKind::Swamp => 2,
            BiomeKind::Jungle => 3,
            BiomeKind::Forest => 1,
            BiomeKind::Savannah => 2,
            BiomeKind::Taiga => 2,
        }
    }
}

#[cfg(test)]
#[test]
fn test_biome_difficulty() {
    use strum::IntoEnumIterator;

    for biome_kind in BiomeKind::iter() {
        assert!(
            (1..=5).contains(&biome_kind.difficulty()),
            "Biome {biome_kind:?} has invalid difficulty {}",
            biome_kind.difficulty()
        );
    }
}
