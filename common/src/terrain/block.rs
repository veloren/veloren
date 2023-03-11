use super::SpriteKind;
use crate::{
    comp::{fluid_dynamics::LiquidKind, tool::ToolKind},
    consts::FRIC_GROUND,
    lottery::LootSpec,
    make_case_elim,
};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use strum::{Display, EnumIter, EnumString};
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
        FromPrimitive,
        EnumString,
        EnumIter,
        Display,
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
        Lava = 0x12,     // TODO: Reevaluate whether this should be in the rock section
        GlowingRock = 0x13,
        GlowingWeakRock = 0x14,
        // 0x12 <= x < 0x20 is reserved for future rocks
        Grass = 0x20, // Note: *not* the same as grass sprites
        Snow = 0x21,
        // 0x21 <= x < 0x30 is reserved for future grasses
        Earth = 0x30,
        Sand = 0x31,
        // 0x32 <= x < 0x40 is reserved for future earths/muds/gravels/sands/etc.
        Wood = 0x40,
        Leaves = 0x41,
        GlowingMushroom = 0x42,
        Ice = 0x43,
        // 0x43 <= x < 0x50 is reserved for future tree parts
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

    #[inline]
    pub const fn liquid_kind(&self) -> Option<LiquidKind> {
        Some(match self {
            BlockKind::Water => LiquidKind::Water,
            BlockKind::Lava => LiquidKind::Lava,
            _ => return None,
        })
    }

    /// Determine whether the block is filled (i.e: fully solid). Right now,
    /// this is the opposite of being a fluid.
    #[inline]
    pub const fn is_filled(&self) -> bool { !self.is_fluid() }

    /// Determine whether the block has an RGB color stored in the attribute
    /// fields.
    #[inline]
    pub const fn has_color(&self) -> bool { self.is_filled() }

    /// Determine whether the block is 'terrain-like'. This definition is
    /// arbitrary, but includes things like rocks, soils, sands, grass, and
    /// other blocks that might be expected to the landscape. Plant matter and
    /// snow are *not* included.
    #[inline]
    pub const fn is_terrain(&self) -> bool {
        matches!(
            self,
            BlockKind::Rock
                | BlockKind::WeakRock
                | BlockKind::GlowingRock
                | BlockKind::GlowingWeakRock
                | BlockKind::Grass
                | BlockKind::Earth
                | BlockKind::Sand
        )
    }
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
    pub const fn lava(sprite: SpriteKind) -> Self {
        Self {
            kind: BlockKind::Lava,
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
        match self.kind() {
            BlockKind::Lava => Some(24),
            BlockKind::GlowingRock | BlockKind::GlowingWeakRock => Some(10),
            BlockKind::GlowingMushroom => Some(20),
            _ => match self.get_sprite()? {
                SpriteKind::StreetLamp | SpriteKind::StreetLampTall => Some(24),
                SpriteKind::Ember => Some(20),
                SpriteKind::WallLamp
                | SpriteKind::WallLampSmall
                | SpriteKind::WallSconce
                | SpriteKind::FireBowlGround
                | SpriteKind::ChristmasOrnament
                | SpriteKind::CliffDecorBlock
                | SpriteKind::Orb => Some(16),
                SpriteKind::Velorite
                | SpriteKind::VeloriteFrag
                | SpriteKind::CavernGrassBlueShort
                | SpriteKind::CavernGrassBlueMedium
                | SpriteKind::CavernGrassBlueLong
                | SpriteKind::CavernLillypadBlue
                | SpriteKind::CavernMycelBlue
                | SpriteKind::CeilingMushroom => Some(6),
                SpriteKind::CaveMushroom
                | SpriteKind::CookingPot
                | SpriteKind::CrystalHigh
                | SpriteKind::CrystalLow => Some(10),
                SpriteKind::Amethyst
                | SpriteKind::Ruby
                | SpriteKind::Sapphire
                | SpriteKind::Diamond
                | SpriteKind::Emerald
                | SpriteKind::Topaz
                | SpriteKind::AmethystSmall
                | SpriteKind::TopazSmall
                | SpriteKind::DiamondSmall
                | SpriteKind::RubySmall
                | SpriteKind::EmeraldSmall
                | SpriteKind::SapphireSmall => Some(3),
                SpriteKind::Lantern => Some(24),
                SpriteKind::SeashellLantern => Some(16),
                SpriteKind::SeaDecorEmblem => Some(12),
                SpriteKind::SeaDecorBlock => Some(10),
                _ => None,
            },
        }
    }

    // minimum block, attenuation
    #[inline]
    pub fn get_max_sunlight(&self) -> (u8, f32) {
        match self.kind() {
            BlockKind::Water => (0, 0.4),
            BlockKind::Leaves => (9, 255.0),
            BlockKind::Wood => (6, 2.0),
            BlockKind::Snow => (6, 2.0),
            BlockKind::Ice => (4, 2.0),
            _ if self.is_opaque() => (0, 255.0),
            _ => (0, 0.0),
        }
    }

    // Filled blocks or sprites
    #[inline]
    pub fn is_solid(&self) -> bool {
        self.get_sprite()
            .map(|s| s.solid_height().is_some())
            .unwrap_or(!matches!(self.kind, BlockKind::Lava))
    }

    /// Can this block be exploded? If so, what 'power' is required to do so?
    /// Note that we don't really define what 'power' is. Consider the units
    /// arbitrary and only important when compared to one-another.
    #[inline]
    pub fn explode_power(&self) -> Option<f32> {
        // Explodable means that the terrain sprite will get removed anyway,
        // so all is good for empty fluids.
        match self.kind() {
            BlockKind::Leaves => Some(0.25),
            BlockKind::Grass => Some(0.5),
            BlockKind::WeakRock => Some(0.75),
            BlockKind::Snow => Some(0.1),
            BlockKind::Ice => Some(0.5),
            BlockKind::Lava => None,
            _ => self.get_sprite().and_then(|sprite| match sprite {
                sprite if sprite.is_container() => None,
                SpriteKind::Keyhole | SpriteKind::KeyDoor => None,
                SpriteKind::Anvil
                | SpriteKind::Cauldron
                | SpriteKind::CookingPot
                | SpriteKind::CraftingBench
                | SpriteKind::Forge
                | SpriteKind::Loom
                | SpriteKind::SpinningWheel
                | SpriteKind::DismantlingBench
                | SpriteKind::TanningRack
                | SpriteKind::Chest
                | SpriteKind::DungeonChest0
                | SpriteKind::DungeonChest1
                | SpriteKind::DungeonChest2
                | SpriteKind::DungeonChest3
                | SpriteKind::DungeonChest4
                | SpriteKind::DungeonChest5
                | SpriteKind::ChestBuried
                | SpriteKind::SeaDecorBlock
                | SpriteKind::SeaDecorChain
                | SpriteKind::SeaDecorWindowHor
                | SpriteKind::SeaDecorWindowVer
                | SpriteKind::Rope
                | SpriteKind::GlassBarrier => None,
                SpriteKind::EnsnaringVines
                | SpriteKind::EnsnaringWeb
                | SpriteKind::SeaUrchin
                | SpriteKind::IceSpike => Some(0.1),
                _ => Some(0.25),
            }),
        }
    }

    #[inline]
    pub fn collectible_id(&self) -> Option<Option<LootSpec<&'static str>>> {
        self.get_sprite()
            .map(|s| s.collectible_id())
            .unwrap_or(None)
    }

    #[inline]
    pub fn is_collectible(&self) -> bool {
        self.collectible_id().is_some() && self.mine_tool().is_none()
    }

    #[inline]
    pub fn is_bonkable(&self) -> bool {
        match self.get_sprite() {
            Some(
                SpriteKind::Apple | SpriteKind::Beehive | SpriteKind::Coconut | SpriteKind::Bomb,
            ) => self.is_solid(),
            _ => false,
        }
    }

    /// The tool required to mine this block. For blocks that cannot be mined,
    /// `None` is returned.
    #[inline]
    pub fn mine_tool(&self) -> Option<ToolKind> {
        match self.kind() {
            BlockKind::WeakRock | BlockKind::Ice | BlockKind::GlowingWeakRock => {
                Some(ToolKind::Pick)
            },
            _ => self.get_sprite().and_then(|s| s.mine_tool()),
        }
    }

    #[inline]
    pub fn is_opaque(&self) -> bool { self.kind().is_filled() }

    #[inline]
    pub fn solid_height(&self) -> f32 {
        self.get_sprite()
            .map(|s| s.solid_height().unwrap_or(0.0))
            .unwrap_or(1.0)
    }

    /// Get the friction constant used to calculate surface friction when
    /// walking/climbing. Currently has no units.
    #[inline]
    pub fn get_friction(&self) -> f32 {
        match self.kind() {
            BlockKind::Ice => FRIC_GROUND * 0.1,
            _ => FRIC_GROUND,
        }
    }

    /// Get the traction permitted by this block as a proportion of the friction
    /// applied.
    ///
    /// 1.0 = default, 0.0 = completely inhibits movement, > 1.0 = potential for
    /// infinite acceleration (in a vacuum).
    #[inline]
    pub fn get_traction(&self) -> f32 {
        match self.kind() {
            BlockKind::Snow => 0.8,
            _ => 1.0,
        }
    }

    #[inline]
    pub fn kind(&self) -> BlockKind { self.kind }

    /// If this block is a fluid, replace its sprite.
    #[inline]
    #[must_use]
    pub fn with_sprite(mut self, sprite: SpriteKind) -> Self {
        if !self.is_filled() {
            self.attr[0] = sprite as u8;
        }
        self
    }

    /// If this block can have orientation, give it a new orientation.
    #[inline]
    #[must_use]
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
    #[must_use]
    pub fn into_vacant(self) -> Self {
        if self.is_fluid() {
            Block::new(self.kind(), Rgb::zero())
        } else {
            // FIXME: Figure out if there's some sensible way to determine what medium to
            // replace a filled block with if it's removed.
            Block::air(SpriteKind::Empty)
        }
    }

    /// Attempt to convert a [`u32`] to a block
    #[inline]
    #[must_use]
    pub fn from_u32(x: u32) -> Option<Self> {
        let [bk, r, g, b] = x.to_le_bytes();
        Some(Self {
            kind: BlockKind::from_u8(bk)?,
            attr: [r, g, b],
        })
    }

    #[inline]
    pub fn to_u32(&self) -> u32 {
        u32::from_le_bytes([self.kind as u8, self.attr[0], self.attr[1], self.attr[2]])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn block_size() {
        assert_eq!(std::mem::size_of::<BlockKind>(), 1);
        assert_eq!(std::mem::size_of::<Block>(), 4);
    }

    #[test]
    fn convert_u32() {
        for bk in BlockKind::iter() {
            let block = Block::new(bk, Rgb::new(165, 90, 204)); // Pretty unique bit patterns
            if bk.is_filled() {
                assert_eq!(Block::from_u32(block.to_u32()), Some(block));
            } else {
                assert_eq!(
                    Block::from_u32(block.to_u32()),
                    Some(Block::new(bk, Rgb::zero())),
                );
            }
        }
    }
}
