use crate::make_case_elim;
use enum_iterator::IntoEnumIterator;
use lazy_static::lazy_static;
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryFrom, fmt};

make_case_elim!(
    sprite_kind,
    #[derive(
        Copy,
        Clone,
        Debug,
        Hash,
        Eq,
        PartialEq,
        Serialize,
        Deserialize,
        IntoEnumIterator,
        FromPrimitive,
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
        ShinyGem = 0x48,
        DropGate = 0x49,
        DropGateBottom = 0x4A,
        GrassSnow = 0x4B,
        Reed = 0x4C,
        Beehive = 0x4D,
        LargeCactus = 0x4E,
    }
);

impl SpriteKind {
    pub fn solid_height(&self) -> Option<f32> {
        // Beware: the height *must* be <= `MAX_HEIGHT` or the collision system will not
        // properly detect it!
        Some(match self {
            SpriteKind::Tomato => 1.65,
            SpriteKind::LargeCactus => 2.5,
            SpriteKind::Scarecrow => 3.0,
            SpriteKind::Turnip => 0.36,
            SpriteKind::Pumpkin => 0.81,
            SpriteKind::Cabbage => 0.45,
            SpriteKind::Chest => 1.09,
            SpriteKind::StreetLamp => 3.0,
            SpriteKind::Carrot => 0.18,
            SpriteKind::Radish => 0.18,
            // TODO: Uncomment this when we have a way to open doors
            // SpriteKind::Door => 3.0,
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
            // TODO: Find suitable heights.
            SpriteKind::BarrelCactus
            | SpriteKind::RoundCactus
            | SpriteKind::ShortCactus
            | SpriteKind::MedFlatCactus
            | SpriteKind::ShortFlatCactus
            | SpriteKind::Apple
            | SpriteKind::Velorite
            | SpriteKind::VeloriteFrag
            | SpriteKind::Coconut
            | SpriteKind::StreetLampTall
            | SpriteKind::Window1
            | SpriteKind::Window2
            | SpriteKind::Window3
            | SpriteKind::Window4
            | SpriteKind::DropGate => 1.0,
            // TODO: Figure out if this should be solid or not.
            SpriteKind::Shelf => 1.0,
            _ => return None,
        })
    }

    pub fn is_collectible(&self) -> bool {
        match self {
            SpriteKind::BlueFlower => false,
            SpriteKind::PinkFlower => false,
            SpriteKind::PurpleFlower => false,
            SpriteKind::RedFlower => false,
            SpriteKind::WhiteFlower => false,
            SpriteKind::YellowFlower => false,
            SpriteKind::Sunflower => true,
            SpriteKind::LongGrass => false,
            SpriteKind::MediumGrass => false,
            SpriteKind::ShortGrass => false,
            SpriteKind::Apple => true,
            SpriteKind::Mushroom => true,
            SpriteKind::Velorite => true,
            SpriteKind::VeloriteFrag => true,
            SpriteKind::Chest => true,
            SpriteKind::Coconut => true,
            SpriteKind::Stones => true,
            SpriteKind::Twigs => true,
            SpriteKind::ShinyGem => true,
            SpriteKind::Crate => true,
            SpriteKind::Beehive => true,
            _ => false,
        }
    }

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
                | SpriteKind::Planter
                | SpriteKind::Shelf
                | SpriteKind::TableSide
                | SpriteKind::TableDining
                | SpriteKind::TableDouble
                | SpriteKind::WardrobeSingle
                | SpriteKind::WardrobeDouble
                | SpriteKind::Pot
                | SpriteKind::Chest
                | SpriteKind::DropGate
                | SpriteKind::DropGateBottom
                | SpriteKind::Door
                | SpriteKind::Beehive
        )
    }
}

impl fmt::Display for SpriteKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{:?}", self) }
}

lazy_static! {
    pub static ref SPRITE_KINDS: HashMap<String, SpriteKind> = SpriteKind::into_enum_iter()
        .map(|sk| (sk.to_string(), sk))
        .collect();
}

impl<'a> TryFrom<&'a str> for SpriteKind {
    type Error = ();

    fn try_from(s: &'a str) -> Result<Self, Self::Error> { SPRITE_KINDS.get(s).copied().ok_or(()) }
}
