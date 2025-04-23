use common_assets::{Asset, AssetCombined, AssetHandle, Concatenate, RonLoader};
use lazy_static::lazy_static;

use crate::terrain::BiomeKind;
use strum::EnumIter;

/// Spots are localised structures that spawn in the world. Conceptually, they
/// fit somewhere between the tree generator and the site generator: an attempt
/// to marry the simplicity of the former with the capability of the latter.
/// They are not globally visible to the game: this means that they do not
/// appear on the map, and cannot interact with rtsim (much).
///
/// To add a new spot, one must:
///
/// 1. Add a new variant to the [`Spot`] enum.
/// 2. Add a new entry to [`veloren-world::layer::spot::SpotGenerate::generate`]
///    that tells the system where to generate your new spot.
/// 3. Add a new arm to the `match` expression in
///    [`veloren-world::layer::spot::apply_spots_to`] that tells the generator
///    how to generate a spot, including the base structure that composes the
///    spot and the entities that should be spawned there.
///
/// Only add spots with randomly spawned NPCs here. Spots that only use
/// EntitySpawner blocks can be added in assets/world/manifests/spots.ron
#[derive(Copy, Clone, Debug, EnumIter, PartialEq)]
pub enum Spot {
    DwarvenGrave,
    SaurokAltar,
    MyrmidonTemple,
    GnarlingTotem,
    WitchHouse,
    GnomeSpring,
    WolfBurrow,
    Igloo,
    //BanditCamp,
    //EnchantedRock,
    //TowerRuin,
    //WellOfLight,
    //MerchantOutpost,
    //RuinedHuntingCabin, <-- Bears!
    // *Random world objects*
    LionRock,
    TreeStumpForest,
    DesertBones,
    Arch,
    AirshipCrash,
    FruitTree,
    Shipwreck,
    Shipwreck2,
    FallenTree,
    GraveSmall,
    JungleTemple,
    SaurokTotem,
    JungleOutpost,
    #[strum(disabled)]
    RonFile(&'static SpotProperties),
}

#[derive(serde::Deserialize, Clone, Debug, PartialEq)]
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

#[derive(serde::Deserialize, Clone, Debug, PartialEq)]
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
