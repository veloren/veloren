use crate::vol::Vox;
use serde_derive::{Deserialize, Serialize};
use std::ops::Deref;
use vek::*;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum BlockKind {
    Air,
    Normal,
    Dense,
    Water,
    LargeCactus,
    BarrelCactus,
    RoundCactus,
    ShortCactus,
    MedFlatCactus,
    ShortFlatCactus,
    BlueFlower,
    PinkFlower,
    PurpleFlower,
    RedFlower,
    WhiteFlower,
    YellowFlower,
    Sunflower,
    LongGrass,
    MediumGrass,
    ShortGrass,
    Apple,
    Mushroom,
    Liana,
    Velorite,
    VeloriteFrag,
    Chest,
    Leaves,
    Pumpkin,
    Welwitch,
    LingonBerry,
    LeafyPlant,
    Fern,
    DeadBush,
    Blueberry,
    Ember,
    Corn,
    WheatYellow,
    WheatGreen,
    Cabbage,
    Flax,
    Carrot,
    Tomato,
    Radish,
    Coconut,
    Turnip,
    Window1,
    Window2,
    Window3,
    Window4,
    Scarecrow,
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
            _ => false,
        }
    }

    pub fn is_fluid(&self) -> bool {
        match self {
            BlockKind::Water => true,
            _ => false,
        }
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
            BlockKind::Tomato => true,
            BlockKind::Radish => false,
            BlockKind::Turnip => false,
            BlockKind::Coconut => false,
            BlockKind::Scarecrow => true,
            _ => true,
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
    pub fn new(kind: BlockKind, color: Rgb<u8>) -> Self {
        Self {
            kind,
            color: color.into_array(),
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
            BlockKind::Window1 | BlockKind::Window2 | BlockKind::Window3 | BlockKind::Window4 => {
                Some(self.color[0] & 0b111)
            },
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
