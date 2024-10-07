use common_assets::{Asset, AssetCombined, AssetHandle, Concatenate, RonLoader};
use lazy_static::lazy_static;

use crate::terrain::BiomeKind;

#[derive(serde::Deserialize, Clone, Debug)]
pub enum SpotCondition {
    MaxGradient(f32),
    Biome(Vec<BiomeKind>),
    NearCliffs,
    NearRiver,
    IsWay,
    IsUnderwater,

    /// no cliffs, no river, no way
    Typical,
    /// implies IsUnderwater
    MinWaterDepth(f32),

    Not(Box<SpotCondition>),
    All(Vec<SpotCondition>),
    Any(Vec<SpotCondition>),
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct SpotProperties {
    pub base_structures: String,
    pub freq: f32,
    pub condition: SpotCondition,
    pub spawn: bool,
}

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct RonSpots(pub Vec<SpotProperties>);

impl Asset for RonSpots {
    type Loader = RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl Concatenate for RonSpots {
    fn concatenate(self, b: Self) -> Self { Self(self.0.concatenate(b.0)) }
}

lazy_static! {
    pub static ref RON_SPOT_PROPERTIES: RonSpots = {
        let spots: AssetHandle<RonSpots> =
            RonSpots::load_expect_combined_static("world.manifests.spots");
        RonSpots(spots.read().0.to_vec())
    };
}
