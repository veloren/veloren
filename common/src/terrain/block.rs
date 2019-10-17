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
}

impl BlockKind {
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
            _ => true,
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

    pub fn kind(&self) -> BlockKind {
        self.kind
    }
}

impl Deref for Block {
    type Target = BlockKind;

    fn deref(&self) -> &Self::Target {
        &self.kind
    }
}

impl Vox for Block {
    fn empty() -> Self {
        Self {
            kind: BlockKind::Air,
            color: [0; 3],
        }
    }

    fn is_empty(&self) -> bool {
        self.is_air()
    }
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
