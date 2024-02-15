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

pub use self::magic::{Attribute, AttributeError};

use crate::{
    attributes,
    comp::{
        item::{ItemDefinitionId, ItemDefinitionIdOwned},
        tool::ToolKind,
    },
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
        Mine       = 0x04,
        HotSurface = 0x05,
    },
    // Furniture. In the future, we might add an attribute to customise material
    // TODO: Remove sizes and variants, represent with attributes
    Furniture = 2 has Ori {
        // Indoor
        CoatRack         = 0x00,
        Bed              = 0x01,
        Bench            = 0x02,
        ChairSingle      = 0x03,
        ChairDouble      = 0x04,
        DrawerLarge      = 0x05,
        DrawerMedium     = 0x06,
        DrawerSmall      = 0x07,
        TableSide        = 0x08,
        TableDining      = 0x09,
        TableDouble      = 0x0A,
        WardrobeSingle   = 0x0B,
        WardrobeDouble   = 0x0C,
        BookshelfArabic  = 0x0D,
        WallTableArabic  = 0x0E,
        TableArabicLarge = 0x0F,
        TableArabicSmall = 0x10,
        CupboardArabic   = 0x11,
        OvenArabic       = 0x12,
        CushionArabic    = 0x13,
        CanapeArabic     = 0x14,
        Shelf            = 0x15,
        Planter          = 0x16,
        Pot              = 0x17,
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
        // Containers
        Chest             = 0x30,
        DungeonChest0     = 0x31,
        DungeonChest1     = 0x32,
        DungeonChest2     = 0x33,
        DungeonChest3     = 0x34,
        DungeonChest4     = 0x35,
        DungeonChest5     = 0x36,
        CoralChest        = 0x37,
        HaniwaUrn         = 0x38,
        TerracottaChest   = 0x39,
        CommonLockedChest = 0x3A,
        ChestBuried       = 0x3B,
        Crate             = 0x3C,
        Barrel            = 0x3D,
        CrateBlock        = 0x3E,
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
    },
    // Sprites representing plants that may grow over time (this does not include plant parts, like fruit).
    Plant = 3 has Growth {
        // Cacti
        BarrelCactus    = 0x00,
        RoundCactus     = 0x01,
        ShortCactus     = 0x02,
        MedFlatCactus   = 0x03,
        ShortFlatCactus = 0x04,
        LargeCactus     = 0x05,
        TallCactus      = 0x06,
        // Flowers
        BlueFlower   = 0x10,
        PinkFlower   = 0x11,
        PurpleFlower = 0x12,
        RedFlower    = 0x13,
        WhiteFlower  = 0x14,
        YellowFlower = 0x15,
        Sunflower    = 0x16,
        Moonbell     = 0x17,
        Pyrebloom    = 0x18,
        // Grasses, ferns, and other 'wild' plants/fungi
        // TODO: remove sizes, make part of the `Growth` attribute
        LongGrass             = 0x20,
        MediumGrass           = 0x21,
        ShortGrass            = 0x22,
        Fern                  = 0x23,
        LargeGrass            = 0x24,
        GrassSnow             = 0x25,
        Reed                  = 0x26,
        GrassBlue             = 0x27,
        SavannaGrass          = 0x28,
        TallSavannaGrass      = 0x29,
        RedSavannaGrass       = 0x2A,
        SavannaBush           = 0x2B,
        Welwitch              = 0x2C,
        LeafyPlant            = 0x2D,
        DeadBush              = 0x2E,
        JungleFern            = 0x2F,
        CavernGrassBlueShort  = 0x30,
        CavernGrassBlueMedium = 0x31,
        CavernGrassBlueLong   = 0x32,
        CavernLillypadBlue    = 0x33,
        EnsnaringVines        = 0x34,
        LillyPads             = 0x35,
        JungleLeafyPlant      = 0x36,
        JungleRedGrass        = 0x37,
        // Crops, berries, and fungi
        Corn          = 0x40,
        WheatYellow   = 0x41,
        WheatGreen    = 0x42, // TODO: Remove `WheatGreen`, make part of the `Growth` attribute
        LingonBerry   = 0x43,
        Blueberry     = 0x44,
        Cabbage       = 0x45,
        Pumpkin       = 0x46,
        Carrot        = 0x47,
        Tomato        = 0x48,
        Radish        = 0x49,
        Turnip        = 0x4A,
        Flax          = 0x4B,
        Mushroom      = 0x4C,
        CaveMushroom  = 0x4D,
        Cotton        = 0x4E,
        WildFlax      = 0x4F,
        SewerMushroom = 0x50,
        // Seaweeds, corals, and other underwater plants
        StonyCoral       = 0x60,
        SoftCoral        = 0x61,
        SeaweedTemperate = 0x62,
        SeaweedTropical  = 0x63,
        GiantKelp        = 0x64,
        BullKelp         = 0x65,
        WavyAlgae        = 0x66,
        SeaGrapes        = 0x67,
        MermaidsFan      = 0x68,
        SeaAnemone       = 0x69,
        Seagrass         = 0x6A,
        RedAlgae         = 0x6B,
        // Danglying ceiling plants/fungi
        Liana           = 0x70,
        CavernMycelBlue = 0x71,
        CeilingMushroom = 0x72,
    },
    // Solid resources
    // TODO: Remove small variants, make deposit size be an attribute
    Resources = 4 {
        // Gems and ores
        Amethyst      = 0x00,
        AmethystSmall = 0x01,
        Ruby          = 0x02,
        RubySmall     = 0x03,
        Sapphire      = 0x04,
        SapphireSmall = 0x05,
        Emerald       = 0x06,
        EmeraldSmall  = 0x07,
        Topaz         = 0x08,
        TopazSmall    = 0x09,
        Diamond       = 0x0A,
        DiamondSmall  = 0x0B,
        Bloodstone    = 0x0C,
        Coal          = 0x0D,
        Cobalt        = 0x0E,
        Copper        = 0x0F,
        Iron          = 0x10,
        Tin           = 0x11,
        Silver        = 0x12,
        Gold          = 0x13,
        Velorite      = 0x14,
        VeloriteFrag  = 0x15,
        // Woods and twigs
        Twigs     = 0x20,
        Wood      = 0x21,
        Bamboo    = 0x22,
        Hardwood  = 0x23,
        Ironwood  = 0x24,
        Frostwood = 0x25,
        Eldwood   = 0x26,
        // Other
        Apple       = 0x30,
        Coconut     = 0x31,
        Stones      = 0x32,
        Seashells   = 0x33,
        Beehive     = 0x34,
        Bowl        = 0x35,
        PotionMinor = 0x36,
        PotionDummy = 0x37,
        VialEmpty   = 0x38,
    },
    // Structural elements including doors and building parts
    Structural = 5 has Ori {
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
        // Windows
        Window1      = 0x10,
        Window2      = 0x11,
        Window3      = 0x12,
        Window4      = 0x13,
        WitchWindow  = 0x14,
        WindowArabic = 0x15,
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
    },
    // Decorative items, both natural and artificial
    Decor = 6 has Ori {
        // Natural
        Bones          = 0x00,
        IceCrystal     = 0x01,
        GlowIceCrystal = 0x02,
        CrystalHigh    = 0x03,
        CrystalLow     = 0x04,
        UnderwaterVent = 0x05,
        SeaUrchin      = 0x06,
        IceSpike       = 0x07,
        Mud            = 0x08,
        Orb            = 0x09,
        EnsnaringWeb   = 0x0A,
        DiamondLight   = 0x0B,
        // Artificial
        Grave            = 0x10,
        Gravestone       = 0x11,
        MelonCut         = 0x12,
        ForgeTools       = 0x13,
        JugAndBowlArabic = 0x14,
        JugArabic        = 0x15,
        DecorSetArabic   = 0x16,
        SepareArabic     = 0x17,
        Candle           = 0x18,
        SmithingTable    = 0x19,
        Forge0           = 0x1A,
        GearWheel0       = 0x1B,
        Quench0          = 0x1C,
        SeaDecorEmblem   = 0x1D,
        SeaDecorPillar   = 0x1E,
        MagicalSeal      = 0x1F,
    },
    Lamp = 7 has Ori, LightEnabled {
        // Standalone lights
        Lantern         = 0,
        StreetLamp      = 1,
        StreetLampTall  = 2,
        SeashellLantern = 3,
        FireBowlGround  = 4,
    },
}

attributes! {
    Ori { bits: 4, err: Infallible, from: |bits| Ok(Self(bits as u8)), into: |Ori(x)| x as u16 },
    Growth { bits: 4, err: Infallible, from: |bits| Ok(Self(bits as u8)), into: |Growth(x)| x as u16 },
    LightEnabled { bits: 1, err: Infallible, from: |bits| Ok(Self(bits == 1)), into: |LightEnabled(x)| x as u16 },
}

// The orientation of the sprite, 0..16
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct Ori(pub u8);

// The growth of the plant, 0..16
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Growth(pub u8);

impl Default for Growth {
    fn default() -> Self { Self(15) }
}

// Whether a light has been toggled on or off.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LightEnabled(pub bool);

impl Default for LightEnabled {
    fn default() -> Self { Self(true) }
}

impl SpriteKind {
    #[inline]
    pub fn solid_height(&self) -> Option<f32> {
        // Beware: the height *must* be <= `MAX_HEIGHT` or the collision system will not
        // properly detect it!
        Some(match self {
            SpriteKind::Bedroll => 0.3,
            SpriteKind::BedrollSnow => 0.4,
            SpriteKind::BedrollPirate => 0.3,
            SpriteKind::Tomato => 1.65,
            SpriteKind::BarrelCactus => 1.0,
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
            SpriteKind::TerracottaChest => 1.09,
            SpriteKind::TerracottaStatue => 5.29,
            SpriteKind::TerracottaBlock => 1.09,
            SpriteKind::SeaDecorChain => 1.09,
            SpriteKind::SeaDecorBlock => 1.00,
            SpriteKind::SeaDecorWindowHor => 0.55,
            SpriteKind::SeaDecorWindowVer => 1.09,
            SpriteKind::SeaDecorPillar => 2.55,
            SpriteKind::SeashellLantern => 2.09,
            SpriteKind::Rope => 1.09,
            SpriteKind::StreetLamp => 2.65,
            SpriteKind::Carrot => 0.18,
            SpriteKind::Radish => 0.18,
            SpriteKind::FireBowlGround => 0.55,
            SpriteKind::Bed => 0.72,
            SpriteKind::Bench => 0.5,
            SpriteKind::ChairSingle => 0.5,
            SpriteKind::ChairDouble => 0.5,
            SpriteKind::CoatRack => 2.36,
            SpriteKind::Crate => 0.90,
            SpriteKind::DrawerSmall => 1.0,
            SpriteKind::DrawerMedium => 2.0,
            SpriteKind::DrawerLarge => 2.0,
            SpriteKind::DungeonWallDecor => 1.0,
            SpriteKind::Planter => 1.09,
            SpriteKind::TableSide => 1.27,
            SpriteKind::TableDining => 1.45,
            SpriteKind::TableDouble => 1.45,
            SpriteKind::WardrobeSingle => 3.0,
            SpriteKind::WardrobeDouble => 3.0,
            SpriteKind::Pot => 0.90,
            SpriteKind::Mud => 0.36,
            SpriteKind::ChestBuried => 0.91,
            SpriteKind::StonyCoral => 1.4,
            SpriteKind::CraftingBench => 1.18,
            SpriteKind::Forge => 2.7,
            SpriteKind::Cauldron => 1.27,
            SpriteKind::SpinningWheel => 1.6,
            SpriteKind::TanningRack => 2.2,
            SpriteKind::Loom => 1.27,
            SpriteKind::Anvil => 1.1,
            SpriteKind::CookingPot => 1.36,
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
            | SpriteKind::HaniwaTrap
            | SpriteKind::HaniwaTrapTriggered
            | SpriteKind::TerracottaKeyDoor
            | SpriteKind::TerracottaKeyhole
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
            | SpriteKind::EnsnaringWeb => 0.1,
            SpriteKind::LillyPads => 0.1,
            SpriteKind::WindowArabic | SpriteKind::BookshelfArabic => 1.9,
            SpriteKind::DecorSetArabic => 2.6,
            SpriteKind::SepareArabic => 2.2,
            SpriteKind::CushionArabic => 0.4,
            SpriteKind::JugArabic => 1.4,
            SpriteKind::TableArabicSmall => 0.9,
            SpriteKind::TableArabicLarge => 1.0,
            SpriteKind::CanapeArabic => 1.2,
            SpriteKind::CupboardArabic => 4.5,
            SpriteKind::WallTableArabic => 2.3,
            SpriteKind::JugAndBowlArabic => 1.4,
            SpriteKind::MelonCut => 0.7,
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
            SpriteKind::Helm => 1.7,
            SpriteKind::Sign => 17.0 / 11.0,
            SpriteKind::Mine => 2.0 / 11.0,
            SpriteKind::SmithingTable => 13.0 / 11.0,
            SpriteKind::Forge0 => 17.0 / 11.0,
            SpriteKind::GearWheel0 => 3.0 / 11.0,
            SpriteKind::Quench0 => 8.0 / 11.0,
            SpriteKind::HotSurface => 0.01,
            SpriteKind::Barrel => 1.0,
            SpriteKind::CrateBlock => 1.0,
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
                    && parent.get_ori().map_or(false, |ori| {
                        Vec2::unit_y()
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
            SpriteKind::AmethystSmall => item("common.items.mineral.gem.amethyst"),
            SpriteKind::TopazSmall => item("common.items.mineral.gem.topaz"),
            SpriteKind::DiamondSmall => item("common.items.mineral.gem.diamond"),
            SpriteKind::RubySmall => item("common.items.mineral.gem.ruby"),
            SpriteKind::EmeraldSmall => item("common.items.mineral.gem.emerald"),
            SpriteKind::SapphireSmall => item("common.items.mineral.gem.sapphire"),
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
            | SpriteKind::GlassKeyhole
            | SpriteKind::KeyholeBars
            | SpriteKind::TerracottaKeyhole => {
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
    pub fn mount_offset(&self) -> Option<(Vec3<f32>, Vec3<f32>)> {
        match self {
            SpriteKind::ChairSingle | SpriteKind::ChairDouble | SpriteKind::Bench => {
                Some((Vec3::new(0.0, 0.0, 0.5), -Vec3::unit_y()))
            },
            SpriteKind::Helm => Some((Vec3::new(0.0, -1.0, 0.0), Vec3::unit_y())),
            SpriteKind::Bed => Some((Vec3::new(0.0, 0.0, 0.6), -Vec3::unit_y())),
            SpriteKind::BedrollSnow | SpriteKind::BedrollPirate => {
                Some((Vec3::new(0.0, 0.0, 0.1), -Vec3::unit_x()))
            },
            SpriteKind::Bedroll => Some((Vec3::new(0.0, 0.0, 0.1), Vec3::unit_y())),
            _ => None,
        }
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
            | SpriteKind::AmethystSmall
            | SpriteKind::TopazSmall
            | SpriteKind::DiamondSmall
            | SpriteKind::RubySmall
            | SpriteKind::EmeraldSmall
            | SpriteKind::Bloodstone
            | SpriteKind::Coal
            | SpriteKind::Cobalt
            | SpriteKind::Copper
            | SpriteKind::Iron
            | SpriteKind::Tin
            | SpriteKind::Silver
            | SpriteKind::Gold
            | SpriteKind::SapphireSmall => Some(ToolKind::Pick),
            SpriteKind::Grave | SpriteKind::Mud => Some(ToolKind::Shovel),
            _ => None,
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
                // Example, do not let this merge with twigs requiring cheese to pick up
                SpriteKind::CommonLockedChest => UnlockKind::Consumes(
                    ItemDefinitionId::Simple("common.items.utility.lockpick_0").to_owned(),
                ),
                SpriteKind::BoneKeyhole => UnlockKind::Consumes(
                    ItemDefinitionId::Simple("common.items.keys.bone_key").to_owned(),
                ),
                SpriteKind::HaniwaKeyhole => UnlockKind::Consumes(
                    ItemDefinitionId::Simple("common.items.keys.haniwa_key").to_owned(),
                ),
                SpriteKind::GlassKeyhole => UnlockKind::Consumes(
                    ItemDefinitionId::Simple("common.items.keys.glass_key").to_owned(),
                ),
                SpriteKind::TerracottaChest => UnlockKind::Consumes(
                    ItemDefinitionId::Simple("common.items.keys.terracotta_key_chest").to_owned(),
                ),
                SpriteKind::TerracottaKeyhole => UnlockKind::Consumes(
                    ItemDefinitionId::Simple("common.items.keys.terracotta_key_door").to_owned(),
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
    // TODO: use ItemKey here?
    Requires(ItemDefinitionIdOwned),
    /// The sprite will consume the given item from the opening character's
    /// inventory
    // TODO: use ItemKey here?
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
