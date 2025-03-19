//! Here's the deal.
//!
//! Blocks are always 4 bytes. The first byte is the [`BlockKind`]. For filled
//! blocks, the remaining 3 sprites are the block colour. For unfilled sprites
//! (air, water, etc.) the remaining 3 bytes correspond to sprite data. That's
//! not a lot to work with! As a result, we're pulling every rabbit out of the
//! bit-twiddling hat to squash as much information as possible into those 3
//! bytes.
//!
//! Fundamentally, sprites are composed of one or more elements: the
//! [`SpriteKind`], which tells us what the sprite *is*, and a list of
//! attributes that define extra properties that the sprite has. Some examples
//! of attributes might include:
//!
//! - the orientation of the sprite (with respect to the volume it sits within)
//! - whether the sprite has snow cover on it
//! - a 'variation seed' that allows frontends to pseudorandomly customise the
//!   appearance of the sprite in a manner that's consistent across clients
//! - Whether doors are open, closed, or permanently locked
//! - The stage of growth of a plant
//! - The kind of plant that sits in pots/planters/vessels
//! - The colour of the sprite
//! - The material of the sprite
//!
//! # Category
//!
//! The first of the three bytes is the sprite 'category'. As much as possible,
//! we should try to have the properties of each sprite within a category be
//! consistent with others in the category, to improve performance.
//!
//! Since a single byte is not enough to disambiguate the [`SpriteKind`] (we
//! have more than 256 kinds, so there's not enough space), the category also
//! corresponds to a 'kind mask': a bitmask that, when applied to the first two
//! of the three bytes gives us the [`SpriteKind`].

mod magic;
//use inline_tweak::tweak_fn;
pub use self::magic::{Attribute, AttributeError};
use crate::{
    attributes,
    comp::{item::ItemDefinitionIdOwned, tool::ToolKind},
    lottery::LootSpec,
    make_case_elim, sprites,
    terrain::Block,
};
use common_i18n::Content;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::{
    convert::{Infallible, TryFrom},
    fmt,
};
use strum::EnumIter;
use vek::*;

/// A sprite that can be deserialized with all its attributes.
///
/// Say we have created the sprites:
/// ```ignore
/// sprites! {
///    Furniture = 0 has Ori, MirrorX {
///       Chair,
///       Table,
///    }
/// }
/// ```
/// And given we're deserializing from ron we could deserialize an array
/// of `StructureSprite` that look like this:
/// ```ignore
/// [
///    // This will be a `SpriteKind::Chair` with default attributes
///    Chair(),
///    // This will be a `SpriteKind::Chair` with the given attributes `Ori(2)` and `MirrorX(true)`.
///    Chair(Ori(2), MirrorX(true)),
///    // This will be a `SpriteKind::Table` with the given attribute `Ori(2)` and the rest of its
///    // attributes set to default.
///    Table(Ori(4)),
/// ]
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct StructureSprite(StructureSpriteKind);

impl StructureSprite {
    pub fn get_block(self, with_sprite: impl FnMut(SpriteKind) -> Block) -> Block {
        self.0.get_block(with_sprite)
    }
}

sprites! {
    Void = 0 {
        Empty = 0,
    },
    // Generic collection of sprites, no attributes but anything goes
    Misc = 1 {
        Ember      = 0x00,
        SmokeDummy = 0x01,
        Bomb       = 0x02,
        FireBlock  = 0x03, // FireBlock for Burning Buff
        HotSurface = 0x04,
        Stones2    = 0x05, // Same as `Stones` but not collectible
    },
    // Furniture. In the future, we might add an attribute to customise material
    // TODO: Remove sizes and variants, represent with attributes
    Furniture = 2 has Ori, MirrorX {
        // Indoor
        BookshelfArabic    = 0x0D,
        WallTableArabic    = 0x0E,
        TableArabicLarge   = 0x0F,
        TableArabicSmall   = 0x10,
        CupboardArabic     = 0x11,
        OvenArabic         = 0x12,
        CushionArabic      = 0x13,
        CanapeArabic       = 0x14,
        Shelf              = 0x15,
        Planter            = 0x16,
        BedMesa            = 0x18,
        WallTableMesa      = 0x19,
        MirrorMesa         = 0x1A,
        WardrobeSingleMesa = 0x1B,
        WardrobeDoubleMesa = 0x1C,
        CupboardMesa       = 0x1D,
        TableCoastalLarge  = 0x1E,
        BenchCoastal       = 0x1F,
        // Crafting
        CraftingBench    = 0x20,
        Forge            = 0x21,
        Cauldron         = 0x22,
        Anvil            = 0x23,
        CookingPot       = 0x24,
        SpinningWheel    = 0x25,
        TanningRack      = 0x26,
        Loom             = 0x27,
        DismantlingBench = 0x28,
        RepairBench      = 0x29,
        // Wall
        HangingBasket     = 0x50,
        HangingSign       = 0x51,
        ChristmasOrnament = 0x52,
        ChristmasWreath   = 0x53,
        WallLampWizard    = 0x54,
        WallLamp          = 0x55,
        WallLampSmall     = 0x56,
        WallSconce        = 0x57,
        DungeonWallDecor  = 0x58,
        WallLampMesa      = 0x59,
        // Outdoor
        Tent          = 0x60,
        Bedroll       = 0x61,
        BedrollSnow   = 0x62,
        BedrollPirate = 0x63,
        Sign          = 0x64,
        Helm          = 0x65,
        // Misc
        Scarecrow      = 0x70,
        FountainArabic = 0x71,
        Hearth         = 0x72,
        ChestWoodDouble= 0x73,
        LanternpostWoodUpper = 0x74,
        LanternpostWoodBase = 0x75,
        LampMetalBase = 0x76,
        BlacksmithBellows = 0x77,
        CarpenterTable = 0x78,
        CarpenterCrateWoodS = 0x79,
        CarpenterCrateWoodL = 0x7A,
        CarpenterToolsWall = 0x7B,
        CarpenterLogCutter = 0x7C,
        BarrelWoodCoal = 0x7D,
        BarrelWoodWater = 0x7E,
        BasketWovenL = 0x7F,
        BasketWovenM = 0x80,
        BasketWovenS = 0x81,
        BonfireMLit = 0x82,
        BonfireMUnlit = 0x83,
        BucketWoodM = 0x84,
        MirrorWoodM = 0x85,
        SackLeatherM = 0x86,
        TrophyframeWoodBear = 0x87,
        TrophyframeWoodDeer = 0x88,
        JugClayM = 0x89,
        LogsWoodBranchS = 0x8A,
        DiningtableWoodCorner = 0x8B,
        DiningtableWoodBody = 0x8C,
        BenchWoodEnd = 0x8D,
        BenchWoodMiddle = 0x8E,
        LogsWoodCoreEnd = 0x8F,
        LogsWoodCoreMiddle = 0x90,
        LogsWoodBarkEnd = 0x91,
        LogsWoodBarkMiddle = 0x92,
        LogsWoodBranchEnd = 0x93,
        LogsWoodBranchMiddle = 0x94,
        SeatWoodBlueMiddle = 0x95,
        SeatWoodBlueSide = 0x96,
        RopeCoilM = 0x97,
        BedWoodWoodlandHead = 0x99,
        BedWoodWoodlandMiddle = 0x9A,
        BedWoodWoodlandTail = 0x9B,
        BenchWoodWoodlandGreen1 = 0x9C,
        BenchWoodWoodlandGreen2 = 0x9D,
        BenchWoodWoodlandGreen3 = 0x9E,
        BenchWoodWoodland = 0xA0,
        ChairWoodWoodland = 0xA1,
        ChairWoodWoodland2 = 0xA2,
        CoatrackMetalWoodland = 0xA3,
        CoatrackWoodWoodland = 0xA4,
        DrawerWoodWoodlandL1 = 0xA5,
        DrawerWoodWoodlandL2 = 0xA6,
        DrawerWoodWoodlandM1 = 0xA7,
        DrawerWoodWoodlandM2 = 0xA8,
        DrawerWoodWoodlandS = 0xA9,
        HandCartWoodHead = 0xAA,
        HandCartWoodMiddle = 0xAB,
        HandCartWoodTail = 0xAC,
        FlowerpotWoodWoodlandS = 0xAD,
        DiningtableWoodWoodlandRound = 0xAE,
        DiningtableWoodWoodlandSquare = 0xAF,
        TableWoodFancyWoodlandCorner = 0xB0,
        TableWoodFancyWoodlandBody = 0xB1,
        WardrobedoubleWoodWoodland = 0xB2,
        WardrobedoubleWoodWoodland2 = 0xB3,
        WardrobesingleWoodWoodland = 0xB4,
        WardrobesingleWoodWoodland2 = 0xB5,
        BedCliffHead = 0xB6,
        BedCliffMiddle = 0xB7,
        BedCliffTail = 0xB8,
        BedCoastalHead = 0xB9,
        BedCoastalMiddle = 0xBA,
        BedCoastalTail = 0xBB,
        BedDesertHead = 0xBC,
        BedDesertMiddle = 0xBD,
        BedDesertTail = 0xBE,
        BedSavannahHead = 0xBF,
        BedSavannahMiddle = 0xC0,
        BedSavannahTail = 0xC1,
        Ladder = 0xC2,
        BookshelfEnd = 0xC3,
        BookshelfMiddle = 0xC4,
    },
    // Sprites representing plants that may grow over time (this does not include plant parts, like fruit).
    Plant = 3 has Growth, Owned, SnowCovered {
        // Cacti
        BarrelCactus    = 0x00,
        RoundCactus     = 0x01,
        ShortCactus     = 0x02,
        MedFlatCactus   = 0x03,
        ShortFlatCactus = 0x04,
        LargeCactus     = 0x05,
        TallCactus      = 0x06,
        // Flowers
        BlueFlower    = 0x10,
        PinkFlower    = 0x11,
        PurpleFlower  = 0x12,
        RedFlower     = 0x13,
        WhiteFlower   = 0x14,
        YellowFlower  = 0x15,
        Sunflower     = 0x16,
        Moonbell      = 0x17,
        Pyrebloom     = 0x18,
        LushFlower    = 0x19,
        LanternFlower = 0x1A,
        // Grasses, ferns, and other 'wild' plants/fungi
        // TODO: remove sizes, make part of the `Growth` attribute
        LongGrass          = 0x20,
        MediumGrass        = 0x21,
        ShortGrass         = 0x22,
        Fern               = 0x23,
        LargeGrass         = 0x24,
        Reed               = 0x25,
        TaigaGrass         = 0x26,
        GrassBlue          = 0x27,
        SavannaGrass       = 0x28,
        TallSavannaGrass   = 0x29,
        RedSavannaGrass    = 0x2A,
        SavannaBush        = 0x2B,
        Welwitch           = 0x2C,
        LeafyPlant         = 0x2D,
        DeadBush           = 0x2E,
        JungleFern         = 0x2F,
        GrassBlueShort     = 0x30,
        GrassBlueMedium    = 0x31,
        GrassBlueLong      = 0x32,
        CavernLillypadBlue = 0x33,
        EnsnaringVines     = 0x34,
        LillyPads          = 0x35,
        JungleLeafyPlant   = 0x36,
        JungleRedGrass     = 0x37,
        LanternPlant       = 0x38,
        SporeReed          = 0x39,
        DeadPlant          = 0x3A,
        // Crops, berries, and fungi
        Corn          = 0x41,
        WheatYellow   = 0x42,
        WheatGreen    = 0x43, // TODO: Remove `WheatGreen`, make part of the `Growth` attribute
        LingonBerry   = 0x44,
        Blueberry     = 0x45,
        Cabbage       = 0x46,
        Pumpkin       = 0x47,
        Carrot        = 0x48,
        Tomato        = 0x49,
        Radish        = 0x4A,
        Turnip        = 0x4B,
        Flax          = 0x4C,
        Mushroom      = 0x4D,
        CaveMushroom  = 0x4E,
        Cotton        = 0x4F,
        WildFlax      = 0x50,
        SewerMushroom = 0x51,
        LushMushroom  = 0x52,
        RockyMushroom = 0x53,
        GlowMushroom  = 0x54,
        // Seaweeds, corals, and other underwater plants
        StonyCoral       = 0x61,
        SoftCoral        = 0x62,
        SeaweedTemperate = 0x63,
        SeaweedTropical  = 0x64,
        GiantKelp        = 0x65,
        BullKelp         = 0x66,
        WavyAlgae        = 0x67,
        SeaGrapes        = 0x68,
        MermaidsFan      = 0x69,
        SeaAnemone       = 0x6A,
        Seagrass         = 0x6B,
        RedAlgae         = 0x6C,
        // Danglying ceiling plants/fungi
        Liana                   = 0x71,
        MycelBlue               = 0x72,
        CeilingMushroom         = 0x73,
        Mold                    = 0x74,
        Root                    = 0x75,
        CeilingLanternPlant     = 0x76,
        CeilingLanternFlower    = 0x77,
        CeilingJungleLeafyPlant = 0x78,
    },
    // Solid resources
    // TODO: Remove small variants, make deposit size be an attribute
    Resource = 4 has Owned, SnowCovered {
        // Gems and ores
        // Woods and twigs
        Twigs     = 0x00,
        Wood      = 0x01,
        Bamboo    = 0x02,
        Hardwood  = 0x03,
        Ironwood  = 0x04,
        Frostwood = 0x05,
        Eldwood   = 0x06,
        // Other
        Apple       = 0x20,
        Coconut     = 0x21,
        Stones      = 0x22,
        Seashells   = 0x23,
        Beehive     = 0x24,
        Bowl        = 0x25,
        PotionMinor = 0x26,
        PotionDummy = 0x27,
        VialEmpty   = 0x28,
    },
    MineableResource = 5 has Damage {
        Amethyst      = 0x00,
        Ruby          = 0x01,
        Sapphire      = 0x02,
        Emerald       = 0x03,
        Topaz         = 0x04,
        Diamond       = 0x05,
        Bloodstone    = 0x06,
        Coal          = 0x07,
        Cobalt        = 0x08,
        Copper        = 0x09,
        Iron          = 0x0A,
        Tin           = 0x0B,
        Silver        = 0x0C,
        Gold          = 0x0D,
        Velorite      = 0x0E,
        VeloriteFrag  = 0x0F,
        Mud           = 0x10,
        Grave         = 0x11,
    },
    // Structural elements including doors and building parts
    Structural = 6 has Ori {
        // Doors and keyholes
        Door         = 0x00,
        DoorDark     = 0x01,
        DoorWide     = 0x02,
        BoneKeyhole  = 0x03,
        BoneKeyDoor  = 0x04,
        Keyhole      = 0x05,
        KeyDoor      = 0x06,
        GlassKeyhole = 0x07,
        KeyholeBars  = 0x08,
        HaniwaKeyDoor = 0x09,
        HaniwaKeyhole = 0x0A,
        TerracottaKeyDoor = 0x0B,
        TerracottaKeyhole = 0x0C,
        SahaginKeyhole = 0x0D,
        SahaginKeyDoor = 0x0E,
        VampireKeyDoor = 0x0F,
        VampireKeyhole = 0x10,
        MyrmidonKeyDoor = 0x11,
        MyrmidonKeyhole = 0x12,
        MinotaurKeyhole = 0x13,

        // Windows
        Window1      = 0x14,
        Window2      = 0x15,
        Window3      = 0x16,
        Window4      = 0x17,
        WitchWindow  = 0x18,
        WindowArabic = 0x19,
        // Walls
        GlassBarrier    = 0x20,
        SeaDecorBlock   = 0x21,
        CliffDecorBlock = 0x22,
        MagicalBarrier  = 0x23,
        OneWayWall      = 0x24,
        // Gates and grates
        SeaDecorWindowHor = 0x30,
        SeaDecorWindowVer = 0x31,
        DropGate          = 0x32,
        DropGateBottom    = 0x33,
        WoodBarricades    = 0x34,
        // Misc
        Rope          = 0x40,
        SeaDecorChain = 0x41,
        IronSpike     = 0x42,
        DoorBars      = 0x43,
        HaniwaTrap    = 0x44,
        HaniwaTrapTriggered = 0x45,
        TerracottaStatue = 0x46,
        TerracottaBlock = 0x47,
        MetalChain = 0x48,
    },
    // Decorative items, both natural and artificial
    Decor = 7 has Ori {
        // Natural
        Bones          = 0x00,
        IceCrystal     = 0x01,
        GlowIceCrystal = 0x02,
        CrystalHigh    = 0x03,
        CrystalLow     = 0x04,
        UnderwaterVent = 0x05,
        SeaUrchin      = 0x06,
        IceSpike       = 0x07,
        Orb            = 0x08,
        EnsnaringWeb   = 0x09,
        DiamondLight   = 0x0A,

        // Artificial
        Gravestone        = 0x10,
        Melon             = 0x11,
        ForgeTools        = 0x12,
        JugAndBowlArabic  = 0x13,
        JugArabic         = 0x14,
        DecorSetArabic    = 0x15,
        SepareArabic      = 0x16,
        Candle            = 0x17,
        SmithingTable     = 0x18,
        Forge0            = 0x19,
        GearWheel0        = 0x1A,
        Quench0           = 0x1B,
        SeaDecorEmblem    = 0x1C,
        SeaDecorPillar    = 0x1D,
        MagicalSeal       = 0x1E,
        JugAndCupsCoastal = 0x1F,
    },
    Lamp = 8 has Ori, LightEnabled {
        // Standalone lights
        Lantern         = 0x00,
        StreetLamp      = 0x01,
        StreetLampTall  = 0x02,
        SeashellLantern = 0x03,
        FireBowlGround  = 0x04,
        MesaLantern     = 0x05,
        LanternpostWoodLantern = 0x06,
        LampMetalShinglesRed = 0x07,
        LampTerracotta = 0x08,
        LampMetalShinglesCyan = 0x09,
        LanternAirshipWallBlackS = 0x0A,
        LanternAirshipWallBrownS = 0x0B,
        LanternAirshipWallChestnutS = 0x0C,
        LanternAirshipWallRedS = 0x0D,
        LanternAirshipGroundBlackS = 0x0E,
        LanternAirshipGroundBrownS = 0x0F,
        LanternAirshipGroundChestnutS = 0x10,
        LanternAirshipGroundRedS = 0x11,
    },
    Container = 9 has Ori, Owned {
        Chest             = 0x00,
        DungeonChest0     = 0x01,
        DungeonChest1     = 0x02,
        DungeonChest2     = 0x03,
        DungeonChest3     = 0x04,
        DungeonChest4     = 0x05,
        DungeonChest5     = 0x06,
        CoralChest        = 0x07,
        HaniwaUrn         = 0x08,
        TerracottaChest   = 0x09,
        SahaginChest      = 0x0A,
        CommonLockedChest = 0x0B,
        ChestBuried       = 0x0C,
        Crate             = 0x0D,
        Barrel            = 0x0E,
        CrateBlock        = 0x0F,
    },
    Modular = 10 has Ori, AdjacentType {
        Fence = 0x00,
    }
}

attributes! {
    Ori { bits: 3, err: Infallible, from: |bits| Ok(Self(bits as u8)), into: |Ori(x)| x as u16 },
    MirrorX { bits: 1, err: Infallible, from: |bits| Ok(Self(bits == 1)), into: |MirrorX(x)| x as u16 },
    MirrorY { bits: 1, err: Infallible, from: |bits| Ok(Self(bits == 1)), into: |MirrorY(x)| x as u16 },
    MirrorZ { bits: 1, err: Infallible, from: |bits| Ok(Self(bits == 1)), into: |MirrorZ(x)| x as u16 },
    Growth { bits: 4, err: Infallible, from: |bits| Ok(Self(bits as u8)), into: |Growth(x)| x as u16 },
    LightEnabled { bits: 1, err: Infallible, from: |bits| Ok(Self(bits == 1)), into: |LightEnabled(x)| x as u16 },
    Damage { bits: 3, err: Infallible, from: |bits| Ok(Self(bits as u8)), into: |Damage(x)| x as u16 },
    Owned { bits: 1, err: Infallible, from: |bits| Ok(Self(bits == 1)), into: |Owned(x)| x as u16 },
    AdjacentType { bits: 3, err: Infallible, from: |bits| Ok(Self(bits as u8)), into: |AdjacentType(x)| x as u16 },
    SnowCovered { bits: 1, err: Infallible, from: |bits| Ok(Self(bits == 1)), into: |SnowCovered(x)| x as u16 },
}

// The orientation of the sprite, 0..16
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Deserialize)]
pub struct Ori(pub u8);

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Deserialize)]
pub struct MirrorX(pub bool);

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Deserialize)]
pub struct MirrorY(pub bool);

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Deserialize)]
pub struct MirrorZ(pub bool);

// The growth of the plant, 0..16
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Growth(pub u8);

impl Default for Growth {
    fn default() -> Self { Self(15) }
}

// Whether a light has been toggled on or off.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct LightEnabled(pub bool);

impl Default for LightEnabled {
    fn default() -> Self { Self(true) }
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Owned(pub bool);

/** Relative Neighbor Position:
    an enum to determine the exact sprite for AdjacentType sprites
    I - Straight - 0
    L - Corner - 1
    T - Junction - 2
    X - Intersection - 3
    End - single connection - 4
**/

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Deserialize, FromPrimitive, Hash)]
#[repr(u8)]
pub enum RelativeNeighborPosition {
    #[default]
    I,
    L,
    T,
    X,
    End,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(from = "RelativeNeighborPosition")]
pub struct AdjacentType(pub u8);

impl From<RelativeNeighborPosition> for AdjacentType {
    fn from(value: RelativeNeighborPosition) -> Self { Self(value as u8) }
}

impl Default for AdjacentType {
    fn default() -> Self { Self::from(RelativeNeighborPosition::I) }
}

// Damage of an ore
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Deserialize)]
pub struct Damage(pub u8);

// Whether a sprite has snow on it
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Deserialize)]
pub struct SnowCovered(pub bool);

impl SpriteKind {
    #[inline]
    //#[tweak_fn]
    pub fn solid_height(&self) -> Option<f32> {
        // Beware: the height *must* be <= `MAX_HEIGHT` or the collision system will not
        // properly detect it!
        Some(match self {
            SpriteKind::Bedroll => 0.3,
            SpriteKind::BedrollSnow => 0.4,
            SpriteKind::BedrollPirate => 0.3,
            SpriteKind::Tomato => 1.65,
            SpriteKind::BarrelCactus => 0.909,
            SpriteKind::LargeCactus => 3.0,
            SpriteKind::TallCactus => 2.63,
            SpriteKind::Scarecrow => 3.0,
            SpriteKind::Turnip => 0.36,
            SpriteKind::Pumpkin => 0.81,
            SpriteKind::Cabbage => 0.45,
            SpriteKind::Chest => 1.09,
            SpriteKind::CommonLockedChest => 1.09,
            SpriteKind::DungeonChest0 => 1.09,
            SpriteKind::DungeonChest1 => 1.09,
            SpriteKind::DungeonChest2 => 1.09,
            SpriteKind::DungeonChest3 => 1.09,
            SpriteKind::DungeonChest4 => 1.09,
            SpriteKind::DungeonChest5 => 1.09,
            SpriteKind::CoralChest => 1.09,
            SpriteKind::HaniwaUrn => 1.09,
            SpriteKind::SahaginChest => 1.09,
            SpriteKind::TerracottaChest => 1.09,
            SpriteKind::TerracottaStatue => 5.29,
            SpriteKind::TerracottaBlock => 1.00,
            SpriteKind::Fence => 1.09,
            SpriteKind::SeaDecorChain => 1.09,
            SpriteKind::SeaDecorBlock => 1.00,
            SpriteKind::SeaDecorWindowHor => 0.55,
            SpriteKind::SeaDecorWindowVer => 1.09,
            SpriteKind::SeaDecorPillar => 2.55,
            SpriteKind::SeashellLantern => 2.09,
            SpriteKind::MesaLantern => 1.3,
            SpriteKind::Rope => 1.09,
            SpriteKind::MetalChain => 1.09,
            SpriteKind::StreetLamp => 2.65,
            SpriteKind::Carrot => 0.18,
            SpriteKind::Radish => 0.18,
            SpriteKind::FireBowlGround => 0.55,
            SpriteKind::BedMesa => 0.82,
            SpriteKind::DungeonWallDecor => 1.0,
            SpriteKind::Planter => 1.09,
            SpriteKind::WardrobeSingleMesa => 2.0,
            SpriteKind::WardrobeDoubleMesa => 2.0,
            SpriteKind::MirrorMesa => 2.0,
            SpriteKind::Mud => 0.36,
            SpriteKind::ChestBuried => 0.91,
            SpriteKind::StonyCoral => 1.4,
            SpriteKind::CraftingBench => 1.18,
            SpriteKind::Forge => 1.818,
            SpriteKind::Cauldron => 1.27,
            SpriteKind::SpinningWheel => 1.454,
            SpriteKind::TanningRack => 1.454,
            SpriteKind::Loom => 1.636,
            SpriteKind::Anvil => 1.18,
            SpriteKind::CookingPot => 1.090,
            SpriteKind::DismantlingBench => 1.18,
            SpriteKind::IceSpike => 1.0,
            SpriteKind::RepairBench => 1.2,
            SpriteKind::RoundCactus => 0.72,
            SpriteKind::ShortCactus => 1.36,
            SpriteKind::MedFlatCactus => 1.36,
            SpriteKind::ShortFlatCactus => 0.91,
            // TODO: Find suitable heights.
            SpriteKind::Apple
            | SpriteKind::Beehive
            | SpriteKind::Velorite
            | SpriteKind::VeloriteFrag
            | SpriteKind::Coconut
            | SpriteKind::StreetLampTall
            | SpriteKind::Window1
            | SpriteKind::Window2
            | SpriteKind::Window3
            | SpriteKind::Window4
            | SpriteKind::DropGate
            | SpriteKind::WitchWindow
            | SpriteKind::SeaUrchin
            | SpriteKind::IronSpike
            | SpriteKind::GlassBarrier
            | SpriteKind::GlassKeyhole
            | SpriteKind::Keyhole
            | SpriteKind::KeyDoor
            | SpriteKind::BoneKeyhole
            | SpriteKind::BoneKeyDoor
            | SpriteKind::HaniwaKeyhole
            | SpriteKind::HaniwaKeyDoor
            | SpriteKind::SahaginKeyhole
            | SpriteKind::SahaginKeyDoor
            | SpriteKind::VampireKeyhole
            | SpriteKind::VampireKeyDoor
            | SpriteKind::HaniwaTrap
            | SpriteKind::HaniwaTrapTriggered
            | SpriteKind::TerracottaKeyDoor
            | SpriteKind::TerracottaKeyhole
            | SpriteKind::MyrmidonKeyDoor
            | SpriteKind::MyrmidonKeyhole
            | SpriteKind::MinotaurKeyhole
            | SpriteKind::Bomb
            | SpriteKind::OneWayWall
            | SpriteKind::DoorBars
            | SpriteKind::KeyholeBars
            | SpriteKind::WoodBarricades
            | SpriteKind::DiamondLight => 1.0,
            // TODO: Figure out if this should be solid or not.
            SpriteKind::Shelf => 1.0,
            SpriteKind::Lantern => 0.9,
            SpriteKind::CrystalHigh | SpriteKind::CrystalLow => 1.5,
            SpriteKind::Bloodstone
            | SpriteKind::Coal
            | SpriteKind::Cobalt
            | SpriteKind::Copper
            | SpriteKind::Iron
            | SpriteKind::Tin
            | SpriteKind::Silver
            | SpriteKind::Gold => 0.6,
            SpriteKind::EnsnaringVines
            | SpriteKind::CavernLillypadBlue
            | SpriteKind::EnsnaringWeb => 0.15,
            SpriteKind::LillyPads => 0.1,
            SpriteKind::WindowArabic | SpriteKind::BookshelfArabic => 1.9,
            SpriteKind::DecorSetArabic => 2.6,
            SpriteKind::SepareArabic => 2.2,
            SpriteKind::CushionArabic => 0.4,
            SpriteKind::JugArabic => 1.4,
            SpriteKind::TableArabicSmall => 0.9,
            SpriteKind::TableArabicLarge => 1.0,
            SpriteKind::TableCoastalLarge => 1.0,
            SpriteKind::BenchCoastal => 1.0,
            SpriteKind::CanapeArabic => 1.2,
            SpriteKind::CupboardArabic => 4.5,
            SpriteKind::WallTableArabic => 2.3,
            SpriteKind::JugAndBowlArabic => 1.4,
            SpriteKind::JugAndCupsCoastal => 1.4,
            SpriteKind::Melon => 0.7,
            SpriteKind::OvenArabic => 3.2,
            SpriteKind::FountainArabic => 2.4,
            SpriteKind::Hearth => 2.3,
            SpriteKind::ForgeTools => 2.8,
            SpriteKind::CliffDecorBlock | SpriteKind::FireBlock => 1.0,
            SpriteKind::Wood
            | SpriteKind::Hardwood
            | SpriteKind::Ironwood
            | SpriteKind::Frostwood
            | SpriteKind::Eldwood => 7.0 / 11.0,
            SpriteKind::Bamboo => 9.0 / 11.0,
            SpriteKind::MagicalBarrier => 3.0,
            SpriteKind::MagicalSeal => 1.0,
            SpriteKind::Helm => 1.909,
            SpriteKind::Sign => 16.0 / 11.0,
            SpriteKind::SmithingTable => 13.0 / 11.0,
            SpriteKind::Forge0 => 17.0 / 11.0,
            SpriteKind::GearWheel0 => 3.0 / 11.0,
            SpriteKind::Quench0 => 8.0 / 11.0,
            SpriteKind::HotSurface => 0.01,
            SpriteKind::Barrel => 1.0,
            SpriteKind::CrateBlock => 1.0,
            SpriteKind::BarrelWoodWater | SpriteKind::BarrelWoodCoal => 1.545,
            SpriteKind::LanternpostWoodLantern | SpriteKind::LanternpostWoodUpper => 2.000,
            SpriteKind::LanternpostWoodBase => 3.000,
            SpriteKind::LampMetalShinglesRed => 1.000,
            SpriteKind::LampMetalShinglesCyan => 1.000,
            SpriteKind::LampMetalBase => 2.818,
            SpriteKind::LampTerracotta => 1.727,
            SpriteKind::BlacksmithBellows => 0.545,
            SpriteKind::CarpenterTable => 2.000,
            SpriteKind::CarpenterCrateWoodS => 0.727,
            SpriteKind::CarpenterCrateWoodL => 1.273,
            SpriteKind::CarpenterLogCutter => 1.545,
            SpriteKind::BasketWovenL | SpriteKind::JugClayM => 1.000,
            SpriteKind::BasketWovenM => 0.909,
            SpriteKind::BasketWovenS => 0.818,
            SpriteKind::BonfireMLit | SpriteKind::BonfireMUnlit => 2.273,
            SpriteKind::BucketWoodM | SpriteKind::SackLeatherM => 1.091,
            SpriteKind::MirrorWoodM => 1.364,
            SpriteKind::TrophyframeWoodBear => 1.455,
            SpriteKind::TrophyframeWoodDeer => 1.727,
            SpriteKind::ChestWoodDouble => 1.182,
            SpriteKind::DiningtableWoodCorner => 1.273,
            SpriteKind::DiningtableWoodBody => 1.273,
            SpriteKind::BenchWoodEnd => 0.636,
            SpriteKind::BenchWoodMiddle => 0.636,
            SpriteKind::LogsWoodCoreEnd => 0.818,
            SpriteKind::LogsWoodCoreMiddle => 0.818,
            SpriteKind::LogsWoodBarkEnd => 1.091,
            SpriteKind::LogsWoodBarkMiddle => 1.091,
            SpriteKind::LogsWoodBranchEnd => 1.091,
            SpriteKind::LogsWoodBranchMiddle => 1.091,
            SpriteKind::LogsWoodBranchS => 1.091,
            SpriteKind::SeatWoodBlueMiddle => 1.818,
            SpriteKind::SeatWoodBlueSide => 1.818,
            SpriteKind::LanternAirshipWallBlackS
            | SpriteKind::LanternAirshipWallBrownS
            | SpriteKind::LanternAirshipWallChestnutS
            | SpriteKind::LanternAirshipWallRedS => 1.182,
            SpriteKind::LanternAirshipGroundBlackS
            | SpriteKind::LanternAirshipGroundBrownS
            | SpriteKind::LanternAirshipGroundChestnutS
            | SpriteKind::LanternAirshipGroundRedS => 0.909,
            SpriteKind::RopeCoilM => 0.363,
            SpriteKind::BedCliffHead => 0.636,
            SpriteKind::BedCliffMiddle => 0.636,
            SpriteKind::BedCliffTail => 0.636,
            SpriteKind::BedCoastalHead => 0.636,
            SpriteKind::BedCoastalMiddle => 0.636,
            SpriteKind::BedCoastalTail => 0.636,
            SpriteKind::BedDesertHead => 0.545,
            SpriteKind::BedDesertMiddle => 0.545,
            SpriteKind::BedDesertTail => 0.545,
            SpriteKind::BedSavannahHead => 0.545,
            SpriteKind::BedSavannahMiddle => 0.545,
            SpriteKind::BedSavannahTail => 0.545,
            SpriteKind::BedWoodWoodlandHead => 0.727,
            SpriteKind::BedWoodWoodlandMiddle => 0.727,
            SpriteKind::BedWoodWoodlandTail => 0.727,
            SpriteKind::BookshelfEnd => 3.0,
            SpriteKind::BookshelfMiddle => 3.0,
            SpriteKind::BenchWoodWoodlandGreen1 => 1.545,
            SpriteKind::BenchWoodWoodlandGreen2 => 1.545,
            SpriteKind::BenchWoodWoodlandGreen3 => 1.545,
            SpriteKind::BenchWoodWoodland => 1.545,
            SpriteKind::ChairWoodWoodland => 1.636,
            SpriteKind::ChairWoodWoodland2 => 1.727,
            SpriteKind::CoatrackMetalWoodland => 2.364,
            SpriteKind::CoatrackWoodWoodland => 2.364,
            SpriteKind::Crate => 0.909,
            SpriteKind::DrawerWoodWoodlandS => 1.000,
            SpriteKind::DrawerWoodWoodlandM1 => 0.909,
            SpriteKind::DrawerWoodWoodlandM2 => 0.909,
            SpriteKind::DrawerWoodWoodlandL1 => 1.273,
            SpriteKind::DrawerWoodWoodlandL2 => 1.273,
            SpriteKind::DiningtableWoodWoodlandRound => 1.273,
            SpriteKind::DiningtableWoodWoodlandSquare => 1.273,
            SpriteKind::TableWoodFancyWoodlandCorner => 1.273,
            SpriteKind::TableWoodFancyWoodlandBody => 1.273,
            SpriteKind::WardrobesingleWoodWoodland => 2.364,
            SpriteKind::WardrobesingleWoodWoodland2 => 2.364,
            SpriteKind::WardrobedoubleWoodWoodland => 2.364,
            SpriteKind::WardrobedoubleWoodWoodland2 => 2.364,
            SpriteKind::FlowerpotWoodWoodlandS => 0.455,
            SpriteKind::HandCartWoodHead => 1.091,
            SpriteKind::HandCartWoodMiddle => 1.091,
            SpriteKind::HandCartWoodTail => 1.091,
            _ => return None,
        })
    }

    pub fn valid_collision_dir(
        &self,
        entity_aabb: Aabb<f32>,
        block_aabb: Aabb<f32>,
        move_dir: Vec3<f32>,
        parent: &Block,
    ) -> bool {
        match self {
            SpriteKind::OneWayWall => {
                // Find the intrusion vector of the collision
                let dir = entity_aabb.collision_vector_with_aabb(block_aabb);

                // Determine an appropriate resolution vector (i.e: the minimum distance
                // needed to push out of the block)
                let max_axis = dir.map(|e| e.abs()).reduce_partial_min();
                let resolve_dir = -dir.map(|e| {
                    if e.abs().to_bits() == max_axis.to_bits() {
                        e.signum()
                    } else {
                        0.0
                    }
                });

                let is_moving_into = move_dir.dot(resolve_dir) <= 0.0;

                is_moving_into
                    && parent.get_attr().is_ok_and(|Ori(ori)| {
                        Vec2::new(
                            0.0,
                            parent.get_attr::<MirrorY>().map_or(1.0, |m| match m.0 {
                                true => -1.0,
                                false => 1.0,
                            }),
                        )
                        .rotated_z(std::f32::consts::PI * 0.25 * ori as f32)
                        .with_z(0.0)
                        .map2(resolve_dir, |e, r| (e - r).abs() < 0.1)
                        .reduce_and()
                    })
            },
            _ => true,
        }
    }

    /// What loot table does collecting this sprite draw from?
    /// None = block cannot be collected
    /// Some(None) = block can be collected, but does not give back an item
    /// Some(Some(_)) = block can be collected and gives back an item
    #[inline]
    //#[tweak_fn]
    pub fn collectible_id(&self) -> Option<Option<LootSpec<&'static str>>> {
        let item = LootSpec::Item;
        let table = LootSpec::LootTable;
        Some(Some(match self {
            SpriteKind::Apple => item("common.items.food.apple"),
            SpriteKind::Mushroom => item("common.items.food.mushroom"),
            SpriteKind::Velorite => item("common.items.mineral.ore.velorite"),
            SpriteKind::VeloriteFrag => item("common.items.mineral.ore.veloritefrag"),
            //SpriteKind::BlueFlower => item("common.items.flowers.blue"),
            //SpriteKind::PinkFlower => item("common.items.flowers.pink"),
            //SpriteKind::PurpleFlower => item("common.items.flowers.purple"),
            SpriteKind::RedFlower => item("common.items.flowers.red"),
            //SpriteKind::WhiteFlower => item("common.items.flowers.white"),
            //SpriteKind::YellowFlower => item("common.items.flowers.yellow"),
            SpriteKind::Sunflower => item("common.items.flowers.sunflower"),
            //SpriteKind::LongGrass => item("common.items.grasses.long"),
            //SpriteKind::MediumGrass => item("common.items.grasses.medium"),
            //SpriteKind::ShortGrass => item("common.items.grasses.short"),
            SpriteKind::Coconut => item("common.items.food.coconut"),
            SpriteKind::Beehive => item("common.items.crafting_ing.honey"),
            SpriteKind::Stones => item("common.items.crafting_ing.stones"),
            SpriteKind::Twigs => item("common.items.crafting_ing.twigs"),
            SpriteKind::VialEmpty => item("common.items.crafting_ing.empty_vial"),
            SpriteKind::Bowl => item("common.items.crafting_ing.bowl"),
            SpriteKind::PotionMinor => item("common.items.consumable.potion_minor"),
            SpriteKind::Amethyst => item("common.items.mineral.gem.amethyst"),
            SpriteKind::Ruby => item("common.items.mineral.gem.ruby"),
            SpriteKind::Diamond => item("common.items.mineral.gem.diamond"),
            SpriteKind::Sapphire => item("common.items.mineral.gem.sapphire"),
            SpriteKind::Topaz => item("common.items.mineral.gem.topaz"),
            SpriteKind::Emerald => item("common.items.mineral.gem.emerald"),
            SpriteKind::Bloodstone => item("common.items.mineral.ore.bloodstone"),
            SpriteKind::Coal => item("common.items.mineral.ore.coal"),
            SpriteKind::Cobalt => item("common.items.mineral.ore.cobalt"),
            SpriteKind::Copper => item("common.items.mineral.ore.copper"),
            SpriteKind::Iron => item("common.items.mineral.ore.iron"),
            SpriteKind::Tin => item("common.items.mineral.ore.tin"),
            SpriteKind::Silver => item("common.items.mineral.ore.silver"),
            SpriteKind::Gold => item("common.items.mineral.ore.gold"),
            SpriteKind::Cotton => item("common.items.crafting_ing.cotton_boll"),
            SpriteKind::Moonbell => item("common.items.flowers.moonbell"),
            SpriteKind::Pyrebloom => item("common.items.flowers.pyrebloom"),
            SpriteKind::WildFlax => item("common.items.flowers.wild_flax"),
            SpriteKind::Seashells => item("common.items.crafting_ing.seashells"),
            SpriteKind::RoundCactus => item("common.items.crafting_ing.cactus"),
            SpriteKind::ShortFlatCactus => item("common.items.crafting_ing.cactus"),
            SpriteKind::MedFlatCactus => item("common.items.crafting_ing.cactus"),
            SpriteKind::Bomb => item("common.items.utility.bomb"),
            SpriteKind::DungeonChest0 => table("common.loot_tables.dungeon.gnarling.chest"),
            SpriteKind::DungeonChest1 => table("common.loot_tables.dungeon.adlet.chest"),
            SpriteKind::DungeonChest2 => table("common.loot_tables.dungeon.sahagin.chest"),
            SpriteKind::DungeonChest3 => table("common.loot_tables.dungeon.haniwa.chest"),
            SpriteKind::DungeonChest4 => table("common.loot_tables.dungeon.myrmidon.chest"),
            SpriteKind::DungeonChest5 => table("common.loot_tables.dungeon.cultist.chest"),
            SpriteKind::Chest => table("common.loot_tables.sprite.chest"),
            SpriteKind::CommonLockedChest => table("common.loot_tables.dungeon.sahagin.chest"),
            SpriteKind::ChestBuried => table("common.loot_tables.sprite.chest-buried"),
            SpriteKind::CoralChest => table("common.loot_tables.dungeon.sea_chapel.chest_coral"),
            SpriteKind::HaniwaUrn => table("common.loot_tables.dungeon.haniwa.key"),
            SpriteKind::TerracottaChest => {
                table("common.loot_tables.dungeon.terracotta.chest_terracotta")
            },
            SpriteKind::SahaginChest => table("common.loot_tables.dungeon.sahagin.key_chest"),
            SpriteKind::Mud => table("common.loot_tables.sprite.mud"),
            SpriteKind::Grave => table("common.loot_tables.sprite.mud"),
            SpriteKind::Crate => table("common.loot_tables.sprite.crate"),
            SpriteKind::Wood => item("common.items.log.wood"),
            SpriteKind::Bamboo => item("common.items.log.bamboo"),
            SpriteKind::Hardwood => item("common.items.log.hardwood"),
            SpriteKind::Ironwood => item("common.items.log.ironwood"),
            SpriteKind::Frostwood => item("common.items.log.frostwood"),
            SpriteKind::Eldwood => item("common.items.log.eldwood"),
            SpriteKind::MagicalBarrier => table("common.loot_tables.sprite.chest"),
            SpriteKind::Keyhole
            | SpriteKind::BoneKeyhole
            | SpriteKind::HaniwaKeyhole
            | SpriteKind::VampireKeyhole
            | SpriteKind::GlassKeyhole
            | SpriteKind::KeyholeBars
            | SpriteKind::SahaginKeyhole
            | SpriteKind::TerracottaKeyhole
            | SpriteKind::MyrmidonKeyhole
            | SpriteKind::MinotaurKeyhole => {
                return Some(None);
            },
            _ => return None,
        }))
    }

    /// Can this sprite be picked up to yield an item without a tool?
    #[inline]
    pub fn is_collectible(&self) -> bool {
        self.collectible_id().is_some() && self.mine_tool().is_none()
    }

    /// Is the sprite a container that will emit a mystery item?
    #[inline]
    pub fn is_container(&self) -> bool {
        matches!(self.collectible_id(), Some(Some(LootSpec::LootTable(_))))
    }

    /// Get the position and direction to mount this sprite if any.
    #[inline]
    //#[tweak_fn]
    pub fn mount_offset(&self) -> Option<(Vec3<f32>, Vec3<f32>)> {
        match self {
            SpriteKind::ChairWoodWoodland
            | SpriteKind::ChairWoodWoodland2
            | SpriteKind::BenchWoodWoodlandGreen1
            | SpriteKind::BenchWoodWoodlandGreen2
            | SpriteKind::BenchWoodWoodlandGreen3
            | SpriteKind::BenchWoodWoodland
            | SpriteKind::BenchWoodEnd
            | SpriteKind::BenchWoodMiddle
            | SpriteKind::BenchCoastal => Some((Vec3::new(0.0, 0.0, 0.5), Vec3::unit_x())),
            SpriteKind::SeatWoodBlueMiddle | SpriteKind::SeatWoodBlueSide => {
                Some((Vec3::new(0.4, 0.0, 0.5), Vec3::unit_x()))
            },
            SpriteKind::Helm => Some((Vec3::new(0.0, -1.1, 0.0), Vec3::unit_y())),
            SpriteKind::BedWoodWoodlandHead
            | SpriteKind::BedCliffHead
            | SpriteKind::BedDesertHead
            | SpriteKind::BedCoastalHead
            | SpriteKind::BedSavannahHead => Some((Vec3::new(1.4, 0.0, 0.5), Vec3::unit_x())),
            SpriteKind::BedMesa => Some((Vec3::new(0.0, 0.0, 0.6), -Vec3::unit_y())),
            SpriteKind::BedrollSnow | SpriteKind::BedrollPirate => {
                Some((Vec3::new(0.0, 0.0, 0.1), -Vec3::unit_x()))
            },
            SpriteKind::Bedroll => Some((Vec3::new(0.0, 0.0, 0.1), Vec3::unit_y())),
            _ => None,
        }
    }

    pub fn is_bed(&self) -> bool {
        matches!(
            self,
            SpriteKind::BedWoodWoodlandHead
                | SpriteKind::BedMesa
                | SpriteKind::BedCliffHead
                | SpriteKind::BedCoastalHead
                | SpriteKind::BedDesertHead
                | SpriteKind::BedSavannahHead
                | SpriteKind::Bedroll
                | SpriteKind::BedrollSnow
                | SpriteKind::BedrollPirate
        )
    }

    #[inline]
    pub fn is_mountable(&self) -> bool { self.mount_offset().is_some() }

    #[inline]
    pub fn is_controller(&self) -> bool { matches!(self, SpriteKind::Helm) }

    #[inline]
    pub fn is_door(&self) -> bool {
        matches!(
            self,
            SpriteKind::Door | SpriteKind::DoorWide | SpriteKind::DoorDark
        )
    }

    /// Which tool (if any) is needed to collect this sprite?
    #[inline]
    pub fn mine_tool(&self) -> Option<ToolKind> {
        match self {
            SpriteKind::Velorite
            | SpriteKind::VeloriteFrag
            // Gems
            | SpriteKind::Amethyst
            | SpriteKind::Ruby
            | SpriteKind::Diamond
            | SpriteKind::Sapphire
            | SpriteKind::Emerald
            | SpriteKind::Topaz
            | SpriteKind::Bloodstone
            | SpriteKind::Coal
            | SpriteKind::Cobalt
            | SpriteKind::Copper
            | SpriteKind::Iron
            | SpriteKind::Tin
            | SpriteKind::Silver
            | SpriteKind::Gold => Some(ToolKind::Pick),
            SpriteKind::Grave | SpriteKind::Mud => Some(ToolKind::Shovel),
            _ => None,
        }
    }

    pub fn required_mine_damage(&self) -> Option<u8> {
        Some(match self {
            SpriteKind::Gold => 6,
            SpriteKind::Silver => 6,
            SpriteKind::Bloodstone => 6,
            SpriteKind::Cobalt => 6,
            SpriteKind::Coal => 6,
            SpriteKind::Iron => 6,
            SpriteKind::Copper => 3,
            SpriteKind::Tin => 3,
            SpriteKind::Amethyst => 3,
            SpriteKind::Ruby => 6,
            SpriteKind::Sapphire => 3,
            SpriteKind::Emerald => 3,
            SpriteKind::Topaz => 3,
            SpriteKind::Diamond => 6,
            SpriteKind::Velorite => 3,
            SpriteKind::VeloriteFrag => 2,
            _ => return None,
        })
    }

    /// Defines how much damage it takes for a mined resource to possibly
    /// make an extra drop.
    pub fn mine_drop_interval(&self) -> u8 {
        match self {
            SpriteKind::Gold => 2,
            SpriteKind::Silver => 2,
            SpriteKind::Bloodstone => 2,
            SpriteKind::Cobalt => 2,
            SpriteKind::Coal => 2,
            SpriteKind::Iron => 2,
            SpriteKind::Copper => 1,
            SpriteKind::Tin => 1,
            SpriteKind::Emerald => 1,
            SpriteKind::Sapphire => 1,
            SpriteKind::Amethyst => 1,
            SpriteKind::Topaz => 1,
            SpriteKind::Diamond => 2,
            SpriteKind::Ruby => 2,
            SpriteKind::Velorite => 1,
            SpriteKind::VeloriteFrag => 1,
            _ => 1,
        }
    }

    /// Requires this item in the inventory to harvest, uses item_definition_id
    // TODO: Do we want to consolidate this with mine_tool at all? Main differences
    // are that mine tool requires item to be an equippable tool, be equipped, and
    // does not consume item while required_item requires that the item be in the
    // inventory and will consume the item on collecting the sprite.
    pub fn unlock_condition(&self, cfg: Option<SpriteCfg>) -> UnlockKind {
        cfg.and_then(|cfg| cfg.unlock)
            .unwrap_or_else(|| match self {
                SpriteKind::CommonLockedChest => UnlockKind::Consumes(
                    ItemDefinitionIdOwned::Simple(String::from("common.items.utility.lockpick_0")),
                ),
                SpriteKind::SahaginKeyhole => UnlockKind::Consumes(ItemDefinitionIdOwned::Simple(
                    String::from("common.items.keys.sahagin_key"),
                )),
                SpriteKind::BoneKeyhole => UnlockKind::Consumes(ItemDefinitionIdOwned::Simple(
                    String::from("common.items.keys.bone_key"),
                )),
                SpriteKind::HaniwaKeyhole => UnlockKind::Consumes(ItemDefinitionIdOwned::Simple(
                    String::from("common.items.keys.haniwa_key"),
                )),
                SpriteKind::VampireKeyhole => UnlockKind::Consumes(ItemDefinitionIdOwned::Simple(
                    String::from("common.items.keys.vampire_key"),
                )),
                SpriteKind::GlassKeyhole => UnlockKind::Consumes(ItemDefinitionIdOwned::Simple(
                    String::from("common.items.keys.glass_key"),
                )),
                SpriteKind::TerracottaChest => UnlockKind::Consumes(
                    ItemDefinitionIdOwned::Simple(String::from(
                        "common.items.keys.terracotta_key_chest",
                    ))
                    .to_owned(),
                ),
                SpriteKind::TerracottaKeyhole => UnlockKind::Consumes(
                    ItemDefinitionIdOwned::Simple(String::from(
                        "common.items.keys.terracotta_key_door",
                    ))
                    .to_owned(),
                ),
                SpriteKind::MyrmidonKeyhole => UnlockKind::Consumes(
                    ItemDefinitionIdOwned::Simple(String::from("common.items.keys.myrmidon_key"))
                        .to_owned(),
                ),
                SpriteKind::MinotaurKeyhole => UnlockKind::Consumes(
                    ItemDefinitionIdOwned::Simple(String::from("common.items.keys.minotaur_key"))
                        .to_owned(),
                ),
                _ => UnlockKind::Free,
            })
    }

    /// Get the [`Content`] that this sprite is labelled with.
    pub fn content(&self, cfg: Option<SpriteCfg>) -> Option<Content> {
        cfg.and_then(|cfg| cfg.content)
    }

    // TODO: phase out use of this method in favour of `sprite.has_attr::<Ori>()`
    #[inline]
    pub fn has_ori(&self) -> bool { self.category().has_attr::<Ori>() }
}

impl fmt::Display for SpriteKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{:?}", self) }
}

use strum::IntoEnumIterator;

lazy_static! {
    pub static ref SPRITE_KINDS: HashMap<String, SpriteKind> =
        SpriteKind::iter().map(|sk| (sk.to_string(), sk)).collect();
}

impl<'a> TryFrom<&'a str> for SpriteKind {
    type Error = ();

    #[inline]
    fn try_from(s: &'a str) -> Result<Self, Self::Error> { SPRITE_KINDS.get(s).copied().ok_or(()) }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UnlockKind {
    /// The sprite can be freely unlocked without any conditions
    Free,
    /// The sprite requires that the opening character has a given item in their
    /// inventory
    Requires(ItemDefinitionIdOwned),
    /// The sprite will consume the given item from the opening character's
    /// inventory
    Consumes(ItemDefinitionIdOwned),
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct SpriteCfg {
    pub unlock: Option<UnlockKind>,
    pub content: Option<Content>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_conv_kind() {
        for sprite in SpriteKind::all() {
            let block = Block::air(*sprite);
            assert_eq!(block.sprite_category(), Some(sprite.category()));
            assert_eq!(block.get_sprite(), Some(*sprite));
        }
    }

    #[test]
    fn sprite_attr() {
        for category in Category::all() {
            if category.has_attr::<Ori>() {
                for sprite in category.all_sprites() {
                    for i in 0..4 {
                        let block = Block::air(*sprite).with_attr(Ori(i)).unwrap();
                        assert_eq!(block.get_attr::<Ori>().unwrap(), Ori(i));
                        assert_eq!(block.get_sprite(), Some(*sprite));
                    }
                }
            }
            if category.has_attr::<Growth>() {
                for sprite in category.all_sprites() {
                    for i in 0..16 {
                        let block = Block::air(*sprite).with_attr(Growth(i)).unwrap();
                        assert_eq!(block.get_attr::<Growth>().unwrap(), Growth(i));
                        assert_eq!(block.get_sprite(), Some(*sprite));
                    }
                }
            }
        }
    }
}
