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
    Wheat,
    LongGrass,
    Flowers,
}

impl BlockKind {
    pub fn is_air(&self) -> bool {
        match self {
            BlockKind::Air => true,
            BlockKind::Wheat => true,
            BlockKind::LongGrass => true,
            BlockKind::Flowers => true,
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
            BlockKind::Wheat => false,
            BlockKind::LongGrass => false,
            BlockKind::Flowers => false,
            _ => true,
        }
    }

    pub fn is_solid(&self) -> bool {
        match self {
            BlockKind::Air => false,
            BlockKind::Water => false,
            BlockKind::Wheat => false,
            BlockKind::LongGrass => false,
            BlockKind::Flowers => false,
            _ => true,
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
