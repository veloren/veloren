use super::SpriteKind;
use crate::make_case_elim;
use enum_iterator::IntoEnumIterator;
use lazy_static::lazy_static;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryFrom, fmt, ops::Deref};
use vek::*;

make_case_elim!(
    block_kind,
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
    pub enum BlockKind {
        Air = 0x00, // Air counts as a fluid
        Water = 0x01,
        // 0x02 <= x < 0x10 are reserved for other fluids. These are 2^n aligned to allow bitwise
        // checking of common conditions. For example, `is_fluid` is just `block_kind &
        // 0x0F == 0` (this is a very common operation used in meshing that could do with
        // being *very* fast).
        Rock = 0x10,
        WeakRock = 0x11, // Explodable
        // 0x12 <= x < 0x20 is reserved for future rocks
        Grass = 0x20, // Note: *not* the same as grass sprites
        Snow = 0x21,
        // 0x21 <= x < 0x30 is reserved for future grasses
        Earth = 0x30,
        Sand = 0x31,
        // 0x32 <= x < 0x40 is reserved for future earths/muds/gravels/sands/etc.
        Wood = 0x40,
        Leaves = 0x41,
        // 0x42 <= x < 0x50 is reserved for future tree parts
        // Covers all other cases (we sometimes have bizarrely coloured misc blocks, and also we
        // often want to experiment with new kinds of block without allocating them a
        // dedicated block kind.
        Misc = 0xFE,
    }
);

impl BlockKind {
    #[inline]
    pub const fn is_air(&self) -> bool { matches!(self, BlockKind::Air) }

    /// Determine whether the block kind is a gas or a liquid. This does not
    /// consider any sprites that may occupy the block (the definition of
    /// fluid is 'a substance that deforms to fit containers')
    #[inline]
    pub const fn is_fluid(&self) -> bool { *self as u8 & 0xF0 == 0x00 }

    #[inline]
    pub const fn is_liquid(&self) -> bool { self.is_fluid() && !self.is_air() }

    /// Determine whether the block is filled (i.e: fully solid). Right now,
    /// this is the opposite of being a fluid.
    #[inline]
    pub const fn is_filled(&self) -> bool { !self.is_fluid() }

    /// Determine whether the block has an RGB color storaged in the attribute
    /// fields.
    #[inline]
    pub const fn has_color(&self) -> bool { self.is_filled() }
}

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

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Block {
    kind: BlockKind,
    attr: [u8; 3],
}

impl Deref for Block {
    type Target = BlockKind;

    fn deref(&self) -> &Self::Target { &self.kind }
}

impl Block {
    pub const MAX_HEIGHT: f32 = 3.0;

    #[inline]
    pub const fn new(kind: BlockKind, color: Rgb<u8>) -> Self {
        Self {
            kind,
            // Colours are only valid for non-fluids
            attr: if kind.is_filled() {
                [color.r, color.g, color.b]
            } else {
                [0; 3]
            },
        }
    }

    #[inline]
    pub const fn air(sprite: SpriteKind) -> Self {
        Self {
            kind: BlockKind::Air,
            attr: [sprite as u8, 0, 0],
        }
    }

    #[inline]
    pub const fn empty() -> Self { Self::air(SpriteKind::Empty) }

    /// TODO: See if we can generalize this somehow.
    #[inline]
    pub const fn water(sprite: SpriteKind) -> Self {
        Self {
            kind: BlockKind::Water,
            attr: [sprite as u8, 0, 0],
        }
    }

    #[inline]
    pub fn get_color(&self) -> Option<Rgb<u8>> {
        if self.has_color() {
            Some(self.attr.into())
        } else {
            None
        }
    }

    #[inline]
    pub fn get_sprite(&self) -> Option<SpriteKind> {
        if !self.is_filled() {
            SpriteKind::from_u8(self.attr[0])
        } else {
            None
        }
    }

    #[inline]
    pub fn get_ori(&self) -> Option<u8> {
        if self.get_sprite()?.has_ori() {
            // TODO: Formalise this a bit better
            Some(self.attr[1] & 0b111)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_glow(&self) -> Option<u8> {
        // TODO: When we have proper volumetric lighting
        // match self.get_sprite()? {
        //     SpriteKind::StreetLamp | SpriteKind::StreetLampTall => Some(20),
        //     SpriteKind::Velorite | SpriteKind::VeloriteFrag => Some(10),
        //     _ => None,
        // }
        None
    }

    #[inline]
    pub fn is_solid(&self) -> bool {
        self.get_sprite()
            .map(|s| s.solid_height().is_some())
            .unwrap_or(true)
    }

    #[inline]
    pub fn is_explodable(&self) -> bool {
        match self.kind() {
            BlockKind::Leaves | BlockKind::Grass | BlockKind::WeakRock => true,
            // Explodable means that the terrain sprite will get removed anyway, so all is good for
            // empty fluids.
            // TODO: Handle the case of terrain sprites we don't want to have explode
            _ => self.get_sprite().is_some(),
        }
    }

    #[inline]
    pub fn is_collectible(&self) -> bool {
        self.get_sprite()
            .map(|s| s.is_collectible())
            .unwrap_or(false)
    }

    #[inline]
    pub fn is_opaque(&self) -> bool { self.kind().is_filled() }

    #[inline]
    pub fn solid_height(&self) -> f32 {
        self.get_sprite()
            .map(|s| s.solid_height().unwrap_or(0.0))
            .unwrap_or(1.0)
    }

    #[inline]
    pub fn kind(&self) -> BlockKind { self.kind }

    /// If this block is a fluid, replace its sprite.
    #[inline]
    pub fn with_sprite(mut self, sprite: SpriteKind) -> Self {
        if !self.is_filled() {
            self.attr[0] = sprite as u8;
        }
        self
    }

    /// If this block can have orientation, give it a new orientation.
    #[inline]
    pub fn with_ori(mut self, ori: u8) -> Option<Self> {
        if self.get_sprite().map(|s| s.has_ori()).unwrap_or(false) {
            self.attr[1] = (self.attr[1] & !0b111) | (ori & 0b111);
            Some(self)
        } else {
            None
        }
    }

    /// Remove the terrain sprite or solid aspects of a block
    #[inline]
    pub fn into_vacant(self) -> Self {
        if self.is_fluid() {
            Block::new(self.kind(), Rgb::zero())
        } else {
            // FIXME: Figure out if there's some sensible way to determine what medium to
            // replace a filled block with if it's removed.
            Block::air(SpriteKind::Empty)
        }
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
