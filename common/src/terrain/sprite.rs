use crate::{
    comp::{
        item::{ItemDefinitionId, ItemDefinitionIdOwned},
        tool::ToolKind,
    },
    lottery::LootSpec,
    make_case_elim,
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fmt};
use strum::EnumIter;

make_case_elim!(
    sprite_kind,
    #[derive(
        Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize, EnumIter, FromPrimitive,
    )]
    #[repr(u8)]
    pub enum SpriteKind {
        // Note that the values of these should be linearly contiguous to allow for quick
        // bounds-checking when casting to a u8.
        Empty = 0x00,
        BarrelCactus = 0x01,
        RoundCactus = 0x02,
        ShortCactus = 0x03,
        MedFlatCactus = 0x04,
        ShortFlatCactus = 0x05,
        BlueFlower = 0x06,
        PinkFlower = 0x07,
        PurpleFlower = 0x08,
        RedFlower = 0x09,
        WhiteFlower = 0x0A,
        YellowFlower = 0x0B,
        Sunflower = 0x0C,
        LongGrass = 0x0D,
        MediumGrass = 0x0E,
        ShortGrass = 0x0F,
        Apple = 0x10,
        Mushroom = 0x11,
        Liana = 0x12,
        Velorite = 0x13,
        VeloriteFrag = 0x14,
        Chest = 0x15,
        Pumpkin = 0x16,
        Welwitch = 0x17,
        LingonBerry = 0x18,
        LeafyPlant = 0x19,
        Fern = 0x1A,
        DeadBush = 0x1B,
        Blueberry = 0x1C,
        Ember = 0x1D,
        Corn = 0x1E,
        WheatYellow = 0x1F,
        WheatGreen = 0x20,
        Cabbage = 0x21,
        Flax = 0x22,
        Carrot = 0x23,
        Tomato = 0x24,
        Radish = 0x25,
        Coconut = 0x26,
        Turnip = 0x27,
        Window1 = 0x28,
        Window2 = 0x29,
        Window3 = 0x2A,
        Window4 = 0x2B,
        Scarecrow = 0x2C,
        StreetLamp = 0x2D,
        StreetLampTall = 0x2E,
        Door = 0x2F,
        Bed = 0x30,
        Bench = 0x31,
        ChairSingle = 0x32,
        ChairDouble = 0x33,
        CoatRack = 0x34,
        Crate = 0x35,
        DrawerLarge = 0x36,
        DrawerMedium = 0x37,
        DrawerSmall = 0x38,
        DungeonWallDecor = 0x39,
        HangingBasket = 0x3A,
        HangingSign = 0x3B,
        WallLamp = 0x3C,
        Planter = 0x3D,
        Shelf = 0x3E,
        TableSide = 0x3F,
        TableDining = 0x40,
        TableDouble = 0x41,
        WardrobeSingle = 0x42,
        WardrobeDouble = 0x43,
        LargeGrass = 0x44,
        Pot = 0x45,
        Stones = 0x46,
        Twigs = 0x47,
        DropGate = 0x48,
        DropGateBottom = 0x49,
        GrassSnow = 0x4A,
        Reed = 0x4B,
        Beehive = 0x4C,
        LargeCactus = 0x4D,
        VialEmpty = 0x4E,
        PotionMinor = 0x4F,
        GrassBlue = 0x50,
        ChestBuried = 0x51,
        Mud = 0x52,
        FireBowlGround = 0x53,
        CaveMushroom = 0x54,
        Bowl = 0x55,
        SavannaGrass = 0x56,
        TallSavannaGrass = 0x57,
        RedSavannaGrass = 0x58,
        SavannaBush = 0x59,
        Amethyst = 0x5A,
        Ruby = 0x5B,
        Sapphire = 0x5C,
        Emerald = 0x5D,
        Topaz = 0x5E,
        Diamond = 0x5F,
        AmethystSmall = 0x60,
        TopazSmall = 0x61,
        DiamondSmall = 0x62,
        RubySmall = 0x63,
        EmeraldSmall = 0x64,
        SapphireSmall = 0x65,
        WallLampSmall = 0x66,
        WallSconce = 0x67,
        StonyCoral = 0x68,
        SoftCoral = 0x69,
        SeaweedTemperate = 0x6A,
        SeaweedTropical = 0x6B,
        GiantKelp = 0x6C,
        BullKelp = 0x6D,
        WavyAlgae = 0x6E,
        SeaGrapes = 0x6F,
        MermaidsFan = 0x70,
        SeaAnemone = 0x71,
        Seashells = 0x72,
        Seagrass = 0x73,
        RedAlgae = 0x74,
        UnderwaterVent = 0x75,
        Lantern = 0x76,
        CraftingBench = 0x77,
        Forge = 0x78,
        Cauldron = 0x79,
        Anvil = 0x7A,
        CookingPot = 0x7B,
        DungeonChest0 = 0x7C,
        DungeonChest1 = 0x7D,
        DungeonChest2 = 0x7E,
        DungeonChest3 = 0x7F,
        DungeonChest4 = 0x80,
        DungeonChest5 = 0x81,
        Loom = 0x82,
        SpinningWheel = 0x83,
        CrystalHigh = 0x84,
        Bloodstone = 0x85,
        Coal = 0x86,
        Cobalt = 0x87,
        Copper = 0x88,
        Iron = 0x89,
        Tin = 0x8A,
        Silver = 0x8B,
        Gold = 0x8C,
        Cotton = 0x8D,
        Moonbell = 0x8E,
        Pyrebloom = 0x8F,
        TanningRack = 0x90,
        WildFlax = 0x91,
        CrystalLow = 0x92,
        CeilingMushroom = 0x93,
        Orb = 0x94,
        EnsnaringVines = 0x95,
        WitchWindow = 0x96,
        SmokeDummy = 0x97,
        Bones = 0x98,
        CavernGrassBlueShort = 0x99,
        CavernGrassBlueMedium = 0x9A,
        CavernGrassBlueLong = 0x9B,
        CavernLillypadBlue = 0x9C,
        CavernMycelBlue = 0x9D,
        DismantlingBench = 0x9E,
        JungleFern = 0x9F,
        LillyPads = 0xA0,
        JungleLeafyPlant = 0xA1,
        JungleRedGrass = 0xA2,
        Bomb = 0xA3,
        ChristmasOrnament = 0xA4,
        ChristmasWreath = 0xA5,
        EnsnaringWeb = 0xA6,
        WindowArabic = 0xA7,
        MelonCut = 0xA8,
        BookshelfArabic = 0xA9,
        DecorSetArabic = 0xAA,
        SepareArabic = 0xAB,
        CushionArabic = 0xAC,
        JugArabic = 0xAD,
        TableArabicSmall = 0xAE,
        TableArabicLarge = 0xAF,
        CanapeArabic = 0xB0,
        CupboardArabic = 0xB1,
        WallTableArabic = 0xB2,
        JugAndBowlArabic = 0xB3,
        OvenArabic = 0xB4,
        FountainArabic = 0xB5,
        Hearth = 0xB6,
        ForgeTools = 0xB7,
        CliffDecorBlock = 0xB8,
        Wood = 0xB9,
        Bamboo = 0xBA,
        Hardwood = 0xBB,
        Ironwood = 0xBC,
        Frostwood = 0xBD,
        Eldwood = 0xBE,
        SeaUrchin = 0xBF,
        GlassBarrier = 0xC0,
        CoralChest = 0xC1,
        SeaDecorChain = 0xC2,
        SeaDecorBlock = 0xC3,
        SeaDecorWindowHor = 0xC4,
        SeaDecorWindowVer = 0xC5,
        SeaDecorEmblem = 0xC6,
        SeaDecorPillar = 0xC7,
        SeashellLantern = 0xC8,
        Rope = 0xC9,
        IceSpike = 0xCA,
        Bedroll = 0xCB,
        BedrollSnow = 0xCC,
        BedrollPirate = 0xCD,
        Tent = 0xCE,
        Grave = 0xCF,
        Gravestone = 0xD0,
        PotionDummy = 0xD1,
        DoorDark = 0xD2,
        MagicalBarrier = 0xD3,
        MagicalSeal = 0xD4,
        WallLampWizard = 0xD5,
        Candle = 0xD6,
        Keyhole = 0xD7,
        KeyDoor = 0xD8,
        CommonLockedChest = 0xD9,
    }
);

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
            SpriteKind::LargeCactus => 2.5,
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
            SpriteKind::Bed => 1.54,
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
            // TODO: Find suitable heights.
            SpriteKind::BarrelCactus
            | SpriteKind::RoundCactus
            | SpriteKind::ShortCactus
            | SpriteKind::MedFlatCactus
            | SpriteKind::ShortFlatCactus
            | SpriteKind::Apple
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
            | SpriteKind::GlassBarrier
            | SpriteKind::Keyhole
            | SpriteKind::KeyDoor
            | SpriteKind::Bomb => 1.0,
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
            SpriteKind::CliffDecorBlock => 1.0,
            SpriteKind::Wood
            | SpriteKind::Hardwood
            | SpriteKind::Ironwood
            | SpriteKind::Frostwood
            | SpriteKind::Eldwood => 7.0 / 11.0,
            SpriteKind::Bamboo => 9.0 / 11.0,
            SpriteKind::MagicalBarrier => 3.0,
            SpriteKind::MagicalSeal => 1.0,
            _ => return None,
        })
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
            SpriteKind::DungeonChest0 => table("common.loot_tables.dungeon.tier-0.chest"),
            SpriteKind::DungeonChest1 => table("common.loot_tables.dungeon.tier-1.chest"),
            SpriteKind::DungeonChest2 => table("common.loot_tables.dungeon.tier-2.chest"),
            SpriteKind::DungeonChest3 => table("common.loot_tables.dungeon.tier-3.chest"),
            SpriteKind::DungeonChest4 => table("common.loot_tables.dungeon.tier-4.chest"),
            SpriteKind::DungeonChest5 => table("common.loot_tables.dungeon.tier-5.chest"),
            SpriteKind::Chest => table("common.loot_tables.sprite.chest"),
            SpriteKind::CommonLockedChest => table("common.loot_tables.dungeon.tier-2.chest"),
            SpriteKind::ChestBuried => table("common.loot_tables.sprite.chest-buried"),
            SpriteKind::CoralChest => table("common.loot_tables.dungeon.sea_chapel.chest_coral"),
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
            SpriteKind::Keyhole => return Some(None),
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
            | SpriteKind::SapphireSmall
            | SpriteKind::GlassBarrier => Some(ToolKind::Pick),
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
                _ => UnlockKind::Free,
            })
    }

    #[inline]
    pub fn has_ori(&self) -> bool {
        matches!(
            self,
            SpriteKind::Window1
                | SpriteKind::Window2
                | SpriteKind::Window3
                | SpriteKind::Window4
                | SpriteKind::Bed
                | SpriteKind::Bench
                | SpriteKind::ChairSingle
                | SpriteKind::ChairDouble
                | SpriteKind::CoatRack
                | SpriteKind::Crate
                | SpriteKind::DrawerLarge
                | SpriteKind::DrawerMedium
                | SpriteKind::DrawerSmall
                | SpriteKind::DungeonWallDecor
                | SpriteKind::HangingBasket
                | SpriteKind::HangingSign
                | SpriteKind::WallLamp
                | SpriteKind::WallLampSmall
                | SpriteKind::WallSconce
                | SpriteKind::Planter
                | SpriteKind::Shelf
                | SpriteKind::TableSide
                | SpriteKind::TableDining
                | SpriteKind::TableDouble
                | SpriteKind::WardrobeSingle
                | SpriteKind::WardrobeDouble
                | SpriteKind::Pot
                | SpriteKind::Chest
                | SpriteKind::DungeonChest0
                | SpriteKind::DungeonChest1
                | SpriteKind::DungeonChest2
                | SpriteKind::DungeonChest3
                | SpriteKind::DungeonChest4
                | SpriteKind::DungeonChest5
                | SpriteKind::CoralChest
                | SpriteKind::SeaDecorWindowVer
                | SpriteKind::SeaDecorEmblem
                | SpriteKind::DropGate
                | SpriteKind::DropGateBottom
                | SpriteKind::Door
                | SpriteKind::DoorDark
                | SpriteKind::Beehive
                | SpriteKind::PotionMinor
                | SpriteKind::PotionDummy
                | SpriteKind::Bowl
                | SpriteKind::VialEmpty
                | SpriteKind::FireBowlGround
                | SpriteKind::Lantern
                | SpriteKind::CraftingBench
                | SpriteKind::Forge
                | SpriteKind::Cauldron
                | SpriteKind::Anvil
                | SpriteKind::CookingPot
                | SpriteKind::SpinningWheel
                | SpriteKind::TanningRack
                | SpriteKind::Loom
                | SpriteKind::DismantlingBench
                | SpriteKind::ChristmasOrnament
                | SpriteKind::ChristmasWreath
                | SpriteKind::WindowArabic
                | SpriteKind::BookshelfArabic
                | SpriteKind::TableArabicLarge
                | SpriteKind::CanapeArabic
                | SpriteKind::CupboardArabic
                | SpriteKind::WallTableArabic
                | SpriteKind::JugAndBowlArabic
                | SpriteKind::JugArabic
                | SpriteKind::MelonCut
                | SpriteKind::OvenArabic
                | SpriteKind::Hearth
                | SpriteKind::ForgeTools
                | SpriteKind::Tent
                | SpriteKind::Bedroll
                | SpriteKind::Grave
                | SpriteKind::Gravestone
                | SpriteKind::MagicalBarrier
        )
    }
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
}
