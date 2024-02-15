use super::{sprite, SpriteKind};
use crate::{
    comp::{fluid_dynamics::LiquidKind, tool::ToolKind},
    consts::FRIC_GROUND,
    lottery::LootSpec,
    make_case_elim, rtsim,
    vol::FilledVox,
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
        // Snow to use with sites, to not attract snowfall particles
        ArtSnow = 0x22,
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

/// # Format
///
/// ```ignore
/// BBBBBBBB CCCCCCCC AAAAAIII IIIIIIII
/// ```
/// - `0..8`  : BlockKind
/// - `8..16` : Category
/// - `16..N` : Attributes (many fields)
/// - `N..32` : Sprite ID
///
/// `N` is per-category. You can match on the category byte to find the length
/// of the ID field.
///
/// Attributes are also per-category. Each category specifies its own list of
/// attribute fields.
///
/// Why is the sprite ID at the end? Simply put, it makes masking faster and
/// easier, which is important because extracting the `SpriteKind` is a more
/// commonly performed operation than extracting attributes.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Block {
    kind: BlockKind,
    data: [u8; 3],
}

impl FilledVox for Block {
    fn default_non_filled() -> Self { Block::air(SpriteKind::Empty) }

    fn is_filled(&self) -> bool { self.kind.is_filled() }
}

impl Deref for Block {
    type Target = BlockKind;

    fn deref(&self) -> &Self::Target { &self.kind }
}

impl Block {
    pub const MAX_HEIGHT: f32 = 3.0;

    /* Constructors */

    #[inline]
    pub const fn from_raw(kind: BlockKind, data: [u8; 3]) -> Self { Self { kind, data } }

    // TODO: Rename to `filled`, make caller guarantees stronger
    #[inline]
    #[track_caller]
    pub const fn new(kind: BlockKind, color: Rgb<u8>) -> Self {
        if kind.is_filled() {
            Self::from_raw(kind, [color.r, color.g, color.b])
        } else {
            // Works because `SpriteKind::Empty` has no attributes
            let data = (SpriteKind::Empty as u32).to_be_bytes();
            Self::from_raw(kind, [data[1], data[2], data[3]])
        }
    }

    // Only valid if `block_kind` is unfilled, so this is just a private utility
    // method
    #[inline]
    pub fn unfilled(kind: BlockKind, sprite: SpriteKind) -> Self {
        #[cfg(debug_assertions)]
        assert!(!kind.is_filled());

        Self::from_raw(kind, sprite.to_initial_bytes())
    }

    #[inline]
    pub fn air(sprite: SpriteKind) -> Self { Self::unfilled(BlockKind::Air, sprite) }

    #[inline]
    pub const fn empty() -> Self {
        // Works because `SpriteKind::Empty` has no attributes
        let data = (SpriteKind::Empty as u32).to_be_bytes();
        Self::from_raw(BlockKind::Air, [data[1], data[2], data[3]])
    }

    #[inline]
    pub fn water(sprite: SpriteKind) -> Self { Self::unfilled(BlockKind::Water, sprite) }

    /* Sprite decoding */

    #[inline(always)]
    pub const fn get_sprite(&self) -> Option<SpriteKind> {
        if !self.kind.is_filled() {
            SpriteKind::from_block(*self)
        } else {
            None
        }
    }

    #[inline(always)]
    pub(super) const fn sprite_category_byte(&self) -> u8 { self.data[0] }

    #[inline(always)]
    pub const fn sprite_category(&self) -> Option<sprite::Category> {
        if self.kind.is_filled() {
            None
        } else {
            sprite::Category::from_block(*self)
        }
    }

    /// Build this block with the given sprite attribute set.
    #[inline]
    pub fn with_attr<A: sprite::Attribute>(
        mut self,
        attr: A,
    ) -> Result<Self, sprite::AttributeError<core::convert::Infallible>> {
        match self.sprite_category() {
            Some(category) => category.write_attr(&mut self, attr)?,
            None => return Err(sprite::AttributeError::NotPresent),
        }
        Ok(self)
    }

    /// Set the given attribute of this block's sprite.
    #[inline]
    pub fn set_attr<A: sprite::Attribute>(
        &mut self,
        attr: A,
    ) -> Result<(), sprite::AttributeError<core::convert::Infallible>> {
        match self.sprite_category() {
            Some(category) => category.write_attr(self, attr),
            None => Err(sprite::AttributeError::NotPresent),
        }
    }

    /// Get the given attribute of this block's sprite.
    #[inline]
    pub fn get_attr<A: sprite::Attribute>(&self) -> Result<A, sprite::AttributeError<A::Error>> {
        match self.sprite_category() {
            Some(category) => category.read_attr(*self),
            None => Err(sprite::AttributeError::NotPresent),
        }
    }

    #[inline(always)]
    pub(super) const fn data(&self) -> [u8; 3] { self.data }

    #[inline(always)]
    pub(super) const fn with_data(mut self, data: [u8; 3]) -> Self {
        self.data = data;
        self
    }

    #[inline(always)]
    pub(super) const fn to_be_u32(self) -> u32 {
        u32::from_be_bytes([self.kind as u8, self.data[0], self.data[1], self.data[2]])
    }

    #[inline]
    pub fn get_color(&self) -> Option<Rgb<u8>> {
        if self.has_color() {
            Some(self.data.into())
        } else {
            None
        }
    }

    // TODO: phase out use of this method in favour of `block.get_attr::<Ori>()`
    #[inline]
    pub fn get_ori(&self) -> Option<u8> { self.get_attr::<sprite::Ori>().ok().map(|ori| ori.0) }

    /// Returns the rtsim resource, if any, that this block corresponds to. If
    /// you want the scarcity of a block to change with rtsim's resource
    /// depletion tracking, you can do so by editing this function.
    #[inline]
    pub fn get_rtsim_resource(&self) -> Option<rtsim::ChunkResource> {
        match self.get_sprite()? {
            SpriteKind::Stones => Some(rtsim::ChunkResource::Stone),
            SpriteKind::Twigs
                | SpriteKind::Wood
                | SpriteKind::Bamboo
                | SpriteKind::Hardwood
                | SpriteKind::Ironwood
                | SpriteKind::Frostwood
                | SpriteKind::Eldwood => Some(rtsim::ChunkResource::Wood),
            SpriteKind::Amethyst
                | SpriteKind::Ruby
                | SpriteKind::Sapphire
                | SpriteKind::Emerald
                | SpriteKind::Topaz
                | SpriteKind::Diamond
                | SpriteKind::AmethystSmall
                | SpriteKind::TopazSmall
                | SpriteKind::DiamondSmall
                | SpriteKind::RubySmall
                | SpriteKind::EmeraldSmall
                | SpriteKind::SapphireSmall
                | SpriteKind::CrystalHigh
                | SpriteKind::CrystalLow => Some(rtsim::ChunkResource::Gem),
            SpriteKind::Bloodstone
                | SpriteKind::Coal
                | SpriteKind::Cobalt
                | SpriteKind::Copper
                | SpriteKind::Iron
                | SpriteKind::Tin
                | SpriteKind::Silver
                | SpriteKind::Gold => Some(rtsim::ChunkResource::Ore),

            SpriteKind::LongGrass
                | SpriteKind::MediumGrass
                | SpriteKind::ShortGrass
                | SpriteKind::LargeGrass
                | SpriteKind::GrassSnow
                | SpriteKind::GrassBlue
                | SpriteKind::SavannaGrass
                | SpriteKind::TallSavannaGrass
                | SpriteKind::RedSavannaGrass
                | SpriteKind::JungleRedGrass
                | SpriteKind::Fern => Some(rtsim::ChunkResource::Grass),
            SpriteKind::BlueFlower
                | SpriteKind::PinkFlower
                | SpriteKind::PurpleFlower
                | SpriteKind::RedFlower
                | SpriteKind::WhiteFlower
                | SpriteKind::YellowFlower
                | SpriteKind::Sunflower
                | SpriteKind::Moonbell
                | SpriteKind::Pyrebloom => Some(rtsim::ChunkResource::Flower),
            SpriteKind::Reed
                | SpriteKind::Flax
                | SpriteKind::WildFlax
                | SpriteKind::Cotton
                | SpriteKind::Corn
                | SpriteKind::WheatYellow
                | SpriteKind::WheatGreen => Some(rtsim::ChunkResource::Plant),
            SpriteKind::Apple
                | SpriteKind::Pumpkin
                | SpriteKind::Beehive // TODO: Not a fruit, but kind of acts like one
                | SpriteKind::Coconut => Some(rtsim::ChunkResource::Fruit),
            SpriteKind::Cabbage
                | SpriteKind::Carrot
                | SpriteKind::Tomato
                | SpriteKind::Radish
                | SpriteKind::Turnip => Some(rtsim::ChunkResource::Vegetable),
            SpriteKind::Mushroom
                | SpriteKind::CaveMushroom
                | SpriteKind::CeilingMushroom => Some(rtsim::ChunkResource::Mushroom),

            SpriteKind::Chest
                | SpriteKind::ChestBuried
                | SpriteKind::PotionMinor
                | SpriteKind::DungeonChest0
                | SpriteKind::DungeonChest1
                | SpriteKind::DungeonChest2
                | SpriteKind::DungeonChest3
                | SpriteKind::DungeonChest4
                | SpriteKind::DungeonChest5
                | SpriteKind::CoralChest
                | SpriteKind::HaniwaUrn
                | SpriteKind::TerracottaChest
                | SpriteKind::Crate => Some(rtsim::ChunkResource::Loot),
            _ => None,
        }
    }

    #[inline]
    pub fn get_glow(&self) -> Option<u8> {
        let glow_level = match self.kind() {
            BlockKind::Lava => 24,
            BlockKind::GlowingRock | BlockKind::GlowingWeakRock => 10,
            BlockKind::GlowingMushroom => 20,
            _ => match self.get_sprite()? {
                SpriteKind::StreetLamp | SpriteKind::StreetLampTall => 24,
                SpriteKind::Ember | SpriteKind::FireBlock => 20,
                SpriteKind::WallLamp
                | SpriteKind::WallLampSmall
                | SpriteKind::WallSconce
                | SpriteKind::FireBowlGround
                | SpriteKind::ChristmasOrnament
                | SpriteKind::CliffDecorBlock
                | SpriteKind::Orb
                | SpriteKind::Candle => 16,
                SpriteKind::DiamondLight => 30,
                SpriteKind::Velorite
                | SpriteKind::VeloriteFrag
                | SpriteKind::CavernGrassBlueShort
                | SpriteKind::CavernGrassBlueMedium
                | SpriteKind::CavernGrassBlueLong
                | SpriteKind::CavernLillypadBlue
                | SpriteKind::CavernMycelBlue
                | SpriteKind::CeilingMushroom => 6,
                SpriteKind::CaveMushroom
                | SpriteKind::CookingPot
                | SpriteKind::CrystalHigh
                | SpriteKind::CrystalLow => 10,
                SpriteKind::SewerMushroom => 16,
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
                | SpriteKind::SapphireSmall => 3,
                SpriteKind::Lantern => 24,
                SpriteKind::TerracottaStatue => 8,
                SpriteKind::SeashellLantern | SpriteKind::GlowIceCrystal => 16,
                SpriteKind::SeaDecorEmblem => 12,
                SpriteKind::SeaDecorBlock | SpriteKind::HaniwaKeyDoor => 10,
                SpriteKind::Mine => 2,
                _ => return None,
            },
        };

        if self
            .get_attr::<sprite::LightEnabled>()
            .map_or(true, |l| l.0)
        {
            Some(glow_level)
        } else {
            None
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
            BlockKind::ArtSnow => (6, 2.0),
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

    pub fn valid_collision_dir(
        &self,
        entity_aabb: Aabb<f32>,
        block_aabb: Aabb<f32>,
        move_dir: Vec3<f32>,
    ) -> bool {
        self.get_sprite().map_or(true, |sprite| {
            sprite.valid_collision_dir(entity_aabb, block_aabb, move_dir, self)
        })
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
            BlockKind::Wood => Some(4.5),
            BlockKind::Lava => None,
            _ => self.get_sprite().and_then(|sprite| match sprite {
                sprite if sprite.is_container() => None,
                SpriteKind::Keyhole
                | SpriteKind::KeyDoor
                | SpriteKind::BoneKeyhole
                | SpriteKind::BoneKeyDoor
                | SpriteKind::OneWayWall
                | SpriteKind::KeyholeBars
                | SpriteKind::DoorBars => None,
                SpriteKind::Anvil
                | SpriteKind::Cauldron
                | SpriteKind::CookingPot
                | SpriteKind::CraftingBench
                | SpriteKind::Forge
                | SpriteKind::Loom
                | SpriteKind::SpinningWheel
                | SpriteKind::DismantlingBench
                | SpriteKind::RepairBench
                | SpriteKind::TanningRack
                | SpriteKind::Chest
                | SpriteKind::DungeonChest0
                | SpriteKind::DungeonChest1
                | SpriteKind::DungeonChest2
                | SpriteKind::DungeonChest3
                | SpriteKind::DungeonChest4
                | SpriteKind::DungeonChest5
                | SpriteKind::CoralChest
                | SpriteKind::HaniwaUrn
                | SpriteKind::HaniwaKeyDoor
                | SpriteKind::HaniwaKeyhole
                | SpriteKind::HaniwaTrap
                | SpriteKind::HaniwaTrapTriggered
                | SpriteKind::ChestBuried
                | SpriteKind::TerracottaChest
                | SpriteKind::SeaDecorBlock
                | SpriteKind::SeaDecorChain
                | SpriteKind::SeaDecorWindowHor
                | SpriteKind::SeaDecorWindowVer
                | SpriteKind::Rope
                | SpriteKind::IronSpike
                | SpriteKind::HotSurface
                | SpriteKind::FireBlock
                | SpriteKind::GlassBarrier
                | SpriteKind::GlassKeyhole
                | SpriteKind::TerracottaKeyDoor
                | SpriteKind::TerracottaKeyhole
                | SpriteKind::TerracottaStatue
                | SpriteKind::TerracottaBlock => None,
                SpriteKind::EnsnaringVines
                | SpriteKind::EnsnaringWeb
                | SpriteKind::SeaUrchin
                | SpriteKind::IceSpike
                | SpriteKind::DiamondLight => Some(0.1),
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
    pub fn is_mountable(&self) -> bool { self.mount_offset().is_some() }

    /// Get the position and direction to mount this block if any.
    pub fn mount_offset(&self) -> Option<(Vec3<f32>, Vec3<f32>)> {
        self.get_sprite().and_then(|sprite| sprite.mount_offset())
    }

    pub fn is_controller(&self) -> bool {
        self.get_sprite()
            .map_or(false, |sprite| sprite.is_controller())
    }

    #[inline]
    pub fn is_bonkable(&self) -> bool {
        match self.get_sprite() {
            Some(
                SpriteKind::Apple
                | SpriteKind::Beehive
                | SpriteKind::Coconut
                | SpriteKind::Bomb
                | SpriteKind::Mine,
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
    pub fn is_opaque(&self) -> bool {
        match self.get_sprite() {
            Some(
                SpriteKind::Keyhole
                | SpriteKind::KeyDoor
                | SpriteKind::KeyholeBars
                | SpriteKind::DoorBars,
            ) => true,
            Some(_) => false,
            None => self.kind().is_filled(),
        }
    }

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
            BlockKind::Snow | BlockKind::ArtSnow => 0.8,
            _ => 1.0,
        }
    }

    /// Apply a light toggle to this block, if possible
    pub fn with_toggle_light(self, enable: bool) -> Option<Self> {
        self.with_attr(sprite::LightEnabled(enable)).ok()
    }

    #[inline]
    pub fn kind(&self) -> BlockKind { self.kind }

    /// If this block is a fluid, replace its sprite.
    #[inline]
    #[must_use]
    pub fn with_sprite(mut self, sprite: SpriteKind) -> Self {
        if !self.is_filled() {
            self = Self::unfilled(self.kind, sprite);
        }
        self
    }

    /// If this block can have orientation, give it a new orientation.
    #[inline]
    #[must_use]
    pub fn with_ori(self, ori: u8) -> Option<Self> { self.with_attr(sprite::Ori(ori)).ok() }

    /// Remove the terrain sprite or solid aspects of a block
    #[inline]
    #[must_use]
    pub fn into_vacant(self) -> Self {
        if self.is_fluid() {
            Block::unfilled(self.kind(), SpriteKind::Empty)
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
        let block = Self {
            kind: BlockKind::from_u8(bk)?,
            data: [r, g, b],
        };

        (block.kind.is_filled() || SpriteKind::from_block(block).is_some()).then_some(block)
    }

    #[inline]
    pub fn to_u32(self) -> u32 {
        u32::from_le_bytes([self.kind as u8, self.data[0], self.data[1], self.data[2]])
    }
}

const _: () = assert!(core::mem::size_of::<BlockKind>() == 1);
const _: () = assert!(core::mem::size_of::<Block>() == 4);
