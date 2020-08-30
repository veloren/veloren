use crate::{make_case_elim, vol::Vox};
use enum_iterator::IntoEnumIterator;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryFrom, fmt, ops::Deref};
use vek::*;

make_case_elim!(
    block_kind,
    #[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize, IntoEnumIterator)]
    #[repr(u8)]
    pub enum BlockKind {
        Air = 0x00,
        Normal = 0x01,
        Dense = 0x02,
        Rock = 0x03,
        Grass = 0x04,
        Leaves = 0x05,
        Water = 0x06,
        LargeCactus = 0x07,
        BarrelCactus = 0x08,
        RoundCactus = 0x09,
        ShortCactus = 0x0A,
        MedFlatCactus = 0x0B,
        ShortFlatCactus = 0x0C,
        BlueFlower = 0x0D,
        PinkFlower = 0x0E,
        PurpleFlower = 0x0F,
        RedFlower = 0x10,
        WhiteFlower = 0x11,
        YellowFlower = 0x12,
        Sunflower = 0x13,
        LongGrass = 0x14,
        MediumGrass = 0x15,
        ShortGrass = 0x16,
        Apple = 0x17,
        Mushroom = 0x18,
        Liana = 0x19,
        Velorite = 0x1A,
        VeloriteFrag = 0x1B,
        Chest = 0x1C,
        Pumpkin = 0x1D,
        Welwitch = 0x1E,
        LingonBerry = 0x1F,
        LeafyPlant = 0x20,
        Fern = 0x21,
        DeadBush = 0x22,
        Blueberry = 0x23,
        Ember = 0x24,
        Corn = 0x25,
        WheatYellow = 0x26,
        WheatGreen = 0x27,
        Cabbage = 0x28,
        Flax = 0x29,
        Carrot = 0x2A,
        Tomato = 0x2B,
        Radish = 0x2C,
        Coconut = 0x2D,
        Turnip = 0x2E,
        Window1 = 0x2F,
        Window2 = 0x30,
        Window3 = 0x31,
        Window4 = 0x32,
        Scarecrow = 0x33,
        StreetLamp = 0x34,
        StreetLampTall = 0x35,
        Door = 0x36,
        Bed = 0x37,
        Bench = 0x38,
        ChairSingle = 0x39,
        ChairDouble = 0x3A,
        CoatRack = 0x3B,
        Crate = 0x3C,
        DrawerLarge = 0x3D,
        DrawerMedium = 0x3E,
        DrawerSmall = 0x3F,
        DungeonWallDecor = 0x40,
        HangingBasket = 0x41,
        HangingSign = 0x42,
        WallLamp = 0x43,
        Planter = 0x44,
        Shelf = 0x45,
        TableSide = 0x46,
        TableDining = 0x47,
        TableDouble = 0x48,
        WardrobeSingle = 0x49,
        WardrobeDouble = 0x4A,
        LargeGrass = 0x4B,
        Pot = 0x4C,
        Stones = 0x4D,
        Twigs = 0x4E,
        ShinyGem = 0x4F,
        DropGate = 0x50,
        DropGateBottom = 0x51,
        GrassSnow = 0x52,
        Reed = 0x53,
    }
);

impl fmt::Display for BlockKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{:?}", self) }
}

lazy_static! {
    pub static ref BLOCK_KINDS: HashMap<String, BlockKind> = BlockKind::into_enum_iter()
        .map(|bk| (bk.to_string(), bk))
        .collect();
}

impl<'a> TryFrom<&'a str> for BlockKind {
    type Error = ();

    fn try_from(s: &'a str) -> Result<Self, Self::Error> { BLOCK_KINDS.get(s).copied().ok_or(()) }
}

impl BlockKind {
    pub const MAX_HEIGHT: f32 = 3.0;

    pub fn is_tangible(&self) -> bool {
        match self {
            BlockKind::Air => false,
            kind => !kind.is_fluid(),
        }
    }

    pub fn is_air(&self) -> bool {
        match self {
            BlockKind::Air => true,
            BlockKind::LargeCactus => true,
            BlockKind::BarrelCactus => true,
            BlockKind::RoundCactus => true,
            BlockKind::ShortCactus => true,
            BlockKind::MedFlatCactus => true,
            BlockKind::ShortFlatCactus => true,
            BlockKind::BlueFlower => true,
            BlockKind::PinkFlower => true,
            BlockKind::PurpleFlower => true,
            BlockKind::RedFlower => true,
            BlockKind::WhiteFlower => true,
            BlockKind::YellowFlower => true,
            BlockKind::Sunflower => true,
            BlockKind::LongGrass => true,
            BlockKind::MediumGrass => true,
            BlockKind::ShortGrass => true,
            BlockKind::Apple => true,
            BlockKind::Mushroom => true,
            BlockKind::Liana => true,
            BlockKind::Velorite => true,
            BlockKind::VeloriteFrag => true,
            BlockKind::Chest => true,
            BlockKind::Welwitch => true,
            BlockKind::LingonBerry => true,
            BlockKind::LeafyPlant => true,
            BlockKind::Fern => true,
            BlockKind::DeadBush => true,
            BlockKind::Blueberry => true,
            BlockKind::Ember => true,
            BlockKind::Corn => true,
            BlockKind::WheatYellow => true,
            BlockKind::WheatGreen => true,
            BlockKind::Cabbage => false,
            BlockKind::Pumpkin => false,
            BlockKind::Flax => true,
            BlockKind::Carrot => true,
            BlockKind::Tomato => false,
            BlockKind::Radish => true,
            BlockKind::Turnip => true,
            BlockKind::Coconut => true,
            BlockKind::Window1 => true,
            BlockKind::Window2 => true,
            BlockKind::Window3 => true,
            BlockKind::Window4 => true,
            BlockKind::Scarecrow => true,
            BlockKind::StreetLamp => true,
            BlockKind::StreetLampTall => true,
            BlockKind::Door => false,
            BlockKind::Bed => false,
            BlockKind::Bench => false,
            BlockKind::ChairSingle => false,
            BlockKind::ChairDouble => false,
            BlockKind::CoatRack => false,
            BlockKind::Crate => false,
            BlockKind::DrawerLarge => false,
            BlockKind::DrawerMedium => false,
            BlockKind::DrawerSmall => false,
            BlockKind::DungeonWallDecor => false,
            BlockKind::HangingBasket => true,
            BlockKind::HangingSign => true,
            BlockKind::WallLamp => true,
            BlockKind::Planter => false,
            BlockKind::Shelf => true,
            BlockKind::TableSide => false,
            BlockKind::TableDining => false,
            BlockKind::TableDouble => false,
            BlockKind::WardrobeSingle => false,
            BlockKind::WardrobeDouble => false,
            BlockKind::Pot => false,
            BlockKind::Stones => true,
            BlockKind::Twigs => true,
            BlockKind::ShinyGem => true,
            BlockKind::DropGate => false,
            BlockKind::DropGateBottom => false,
            BlockKind::GrassSnow => true,
            BlockKind::Reed => true,
            _ => false,
        }
    }

    pub fn is_fluid(&self) -> bool { matches!(self, BlockKind::Water) }

    pub fn get_glow(&self) -> Option<u8> {
        // TODO: When we have proper volumetric lighting
        // match self {
        //     BlockKind::StreetLamp | BlockKind::StreetLampTall => Some(20),
        //     BlockKind::Velorite | BlockKind::VeloriteFrag => Some(10),
        //     _ => None,
        // }
        None
    }

    pub fn is_opaque(&self) -> bool {
        match self {
            BlockKind::Air => false,
            BlockKind::Water => false,
            BlockKind::LargeCactus => false,
            BlockKind::BarrelCactus => false,
            BlockKind::RoundCactus => false,
            BlockKind::ShortCactus => false,
            BlockKind::MedFlatCactus => false,
            BlockKind::ShortFlatCactus => false,
            BlockKind::BlueFlower => false,
            BlockKind::PinkFlower => false,
            BlockKind::PurpleFlower => false,
            BlockKind::RedFlower => false,
            BlockKind::WhiteFlower => false,
            BlockKind::YellowFlower => false,
            BlockKind::Sunflower => false,
            BlockKind::LongGrass => false,
            BlockKind::MediumGrass => false,
            BlockKind::ShortGrass => false,
            BlockKind::Apple => false,
            BlockKind::Mushroom => false,
            BlockKind::Liana => false,
            BlockKind::Velorite => false,
            BlockKind::VeloriteFrag => false,
            BlockKind::Chest => false,
            BlockKind::Pumpkin => false,
            BlockKind::Welwitch => false,
            BlockKind::LingonBerry => false,
            BlockKind::LeafyPlant => false,
            BlockKind::Fern => false,
            BlockKind::DeadBush => false,
            BlockKind::Blueberry => false,
            BlockKind::Ember => false,
            BlockKind::Corn => false,
            BlockKind::WheatYellow => false,
            BlockKind::WheatGreen => false,
            BlockKind::Cabbage => false,
            BlockKind::Flax => false,
            BlockKind::Carrot => false,
            BlockKind::Tomato => false,
            BlockKind::Radish => false,
            BlockKind::Turnip => false,
            BlockKind::Coconut => false,
            BlockKind::Window1 => false,
            BlockKind::Window2 => false,
            BlockKind::Window3 => false,
            BlockKind::Window4 => false,
            BlockKind::Scarecrow => false,
            BlockKind::StreetLamp => false,
            BlockKind::StreetLampTall => false,
            BlockKind::Door => false,
            BlockKind::Bed => false,
            BlockKind::Bench => false,
            BlockKind::ChairSingle => false,
            BlockKind::ChairDouble => false,
            BlockKind::CoatRack => false,
            BlockKind::Crate => false,
            BlockKind::DrawerLarge => false,
            BlockKind::DrawerMedium => false,
            BlockKind::DrawerSmall => false,
            BlockKind::DungeonWallDecor => false,
            BlockKind::HangingBasket => false,
            BlockKind::HangingSign => false,
            BlockKind::WallLamp => false,
            BlockKind::Planter => false,
            BlockKind::Shelf => false,
            BlockKind::TableSide => false,
            BlockKind::TableDining => false,
            BlockKind::TableDouble => false,
            BlockKind::WardrobeSingle => false,
            BlockKind::WardrobeDouble => false,
            BlockKind::LargeGrass => false,
            BlockKind::Pot => false,
            BlockKind::Stones => false,
            BlockKind::Twigs => false,
            BlockKind::ShinyGem => false,
            BlockKind::DropGate => false,
            BlockKind::DropGateBottom => false,
            BlockKind::GrassSnow => false,
            BlockKind::Reed => false,
            _ => true,
        }
    }

    pub fn is_solid(&self) -> bool {
        match self {
            BlockKind::Air => false,
            BlockKind::Water => false,
            BlockKind::LargeCactus => true,
            BlockKind::BarrelCactus => true,
            BlockKind::RoundCactus => true,
            BlockKind::ShortCactus => true,
            BlockKind::MedFlatCactus => true,
            BlockKind::ShortFlatCactus => true,
            BlockKind::BlueFlower => false,
            BlockKind::PinkFlower => false,
            BlockKind::PurpleFlower => false,
            BlockKind::RedFlower => false,
            BlockKind::WhiteFlower => false,
            BlockKind::YellowFlower => false,
            BlockKind::Sunflower => false,
            BlockKind::LongGrass => false,
            BlockKind::MediumGrass => false,
            BlockKind::ShortGrass => false,
            BlockKind::Apple => true,
            BlockKind::Mushroom => false,
            BlockKind::Liana => false,
            BlockKind::Chest => true,
            BlockKind::Pumpkin => true,
            BlockKind::Welwitch => false,
            BlockKind::LingonBerry => false,
            BlockKind::LeafyPlant => false,
            BlockKind::Fern => false,
            BlockKind::DeadBush => false,
            BlockKind::Blueberry => false,
            BlockKind::Ember => false,
            BlockKind::Corn => false,
            BlockKind::WheatYellow => false,
            BlockKind::WheatGreen => false,
            BlockKind::Cabbage => true,
            BlockKind::Flax => false,
            BlockKind::Carrot => true,
            BlockKind::Tomato => true,
            BlockKind::Radish => true,
            BlockKind::Turnip => true,
            BlockKind::Coconut => true,
            BlockKind::Scarecrow => true,
            BlockKind::StreetLamp => true,
            BlockKind::StreetLampTall => true,
            BlockKind::Door => false,
            BlockKind::Bed => true,
            BlockKind::Bench => true,
            BlockKind::ChairSingle => true,
            BlockKind::ChairDouble => true,
            BlockKind::CoatRack => true,
            BlockKind::Crate => true,
            BlockKind::DrawerLarge => true,
            BlockKind::DrawerMedium => true,
            BlockKind::DrawerSmall => true,
            BlockKind::DungeonWallDecor => true,
            BlockKind::HangingBasket => false,
            BlockKind::HangingSign => false,
            BlockKind::WallLamp => false,
            BlockKind::Planter => true,
            BlockKind::Shelf => false,
            BlockKind::TableSide => true,
            BlockKind::TableDining => true,
            BlockKind::TableDouble => true,
            BlockKind::WardrobeSingle => true,
            BlockKind::WardrobeDouble => true,
            BlockKind::Pot => true,
            BlockKind::Stones => false,
            BlockKind::Twigs => false,
            BlockKind::ShinyGem => false,
            BlockKind::DropGate => true,
            BlockKind::DropGateBottom => false,
            BlockKind::GrassSnow => false,
            BlockKind::Reed => false,
            _ => true,
        }
    }

    pub fn is_explodable(&self) -> bool {
        match self {
            BlockKind::Leaves | BlockKind::Grass | BlockKind::Rock | BlockKind::GrassSnow => true,
            BlockKind::Air => false,
            bk => bk.is_air(), // Temporary catch for terrain sprites
        }
    }

    // TODO: Integrate this into `is_solid` by returning an `Option<f32>`
    pub fn get_height(&self) -> f32 {
        // Beware: the height *must* be <= `MAX_HEIGHT` or the collision system will not
        // properly detect it!
        match self {
            BlockKind::Tomato => 1.65,
            BlockKind::LargeCactus => 2.5,
            BlockKind::Scarecrow => 3.0,
            BlockKind::Turnip => 0.36,
            BlockKind::Pumpkin => 0.81,
            BlockKind::Cabbage => 0.45,
            BlockKind::Chest => 1.09,
            BlockKind::StreetLamp => 3.0,
            BlockKind::Carrot => 0.18,
            BlockKind::Radish => 0.18,
            BlockKind::Door => 3.0,
            BlockKind::Bed => 1.54,
            BlockKind::Bench => 0.5,
            BlockKind::ChairSingle => 0.5,
            BlockKind::ChairDouble => 0.5,
            BlockKind::CoatRack => 2.36,
            BlockKind::Crate => 0.90,
            BlockKind::DrawerSmall => 1.0,
            BlockKind::DrawerMedium => 2.0,
            BlockKind::DrawerLarge => 2.0,
            BlockKind::DungeonWallDecor => 1.0,
            BlockKind::Planter => 1.09,
            BlockKind::TableSide => 1.27,
            BlockKind::TableDining => 1.45,
            BlockKind::TableDouble => 1.45,
            BlockKind::WardrobeSingle => 3.0,
            BlockKind::WardrobeDouble => 3.0,
            BlockKind::Pot => 0.90,
            _ => 1.0,
        }
    }

    pub fn is_collectible(&self) -> bool {
        match self {
            BlockKind::BlueFlower => false,
            BlockKind::PinkFlower => false,
            BlockKind::PurpleFlower => false,
            BlockKind::RedFlower => false,
            BlockKind::WhiteFlower => false,
            BlockKind::YellowFlower => false,
            BlockKind::Sunflower => false,
            BlockKind::LongGrass => false,
            BlockKind::MediumGrass => false,
            BlockKind::ShortGrass => false,
            BlockKind::Apple => true,
            BlockKind::Mushroom => true,
            BlockKind::Velorite => true,
            BlockKind::VeloriteFrag => true,
            BlockKind::Chest => true,
            BlockKind::Coconut => true,
            BlockKind::Stones => true,
            BlockKind::Twigs => true,
            BlockKind::ShinyGem => true,
            BlockKind::Crate => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[repr(packed)]
pub struct Block {
    kind: BlockKind,
    color: [u8; 3],
}

impl Block {
    pub const fn new(kind: BlockKind, color: Rgb<u8>) -> Self {
        Self {
            kind,
            color: [color.r, color.g, color.b],
        }
    }

    pub fn get_color(&self) -> Option<Rgb<u8>> {
        if !self.is_air() {
            Some(self.color.into())
        } else {
            None
        }
    }

    pub fn get_ori(&self) -> Option<u8> {
        match self.kind {
            BlockKind::Window1
            | BlockKind::Window2
            | BlockKind::Window3
            | BlockKind::Window4
            | BlockKind::Bed
            | BlockKind::Bench
            | BlockKind::ChairSingle
            | BlockKind::ChairDouble
            | BlockKind::CoatRack
            | BlockKind::Crate
            | BlockKind::DrawerLarge
            | BlockKind::DrawerMedium
            | BlockKind::DrawerSmall
            | BlockKind::DungeonWallDecor
            | BlockKind::HangingBasket
            | BlockKind::HangingSign
            | BlockKind::WallLamp
            | BlockKind::Planter
            | BlockKind::Shelf
            | BlockKind::TableSide
            | BlockKind::TableDining
            | BlockKind::TableDouble
            | BlockKind::WardrobeSingle
            | BlockKind::WardrobeDouble
            | BlockKind::Pot
            | BlockKind::Chest
            | BlockKind::DropGate
            | BlockKind::DropGateBottom
            | BlockKind::Door => Some(self.color[0] & 0b111),
            _ => None,
        }
    }

    pub fn kind(&self) -> BlockKind { self.kind }
}

impl Deref for Block {
    type Target = BlockKind;

    fn deref(&self) -> &Self::Target { &self.kind }
}

impl Vox for Block {
    fn empty() -> Self {
        Self {
            kind: BlockKind::Air,
            color: [0; 3],
        }
    }

    fn is_empty(&self) -> bool { self.is_air() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_size() {
        assert_eq!(std::mem::size_of::<BlockKind>(), 1);
        assert_eq!(std::mem::size_of::<Block>(), 4);
    }
}
