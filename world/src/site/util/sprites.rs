use crate::site::{Fill, Painter};

use super::Dir;
use common::terrain::{
    Block, SpriteKind,
    sprite::{MirrorX, Ori},
};
use enum_map::EnumMap;
use strum::IntoEnumIterator;
use vek::*;

/// A struct to make it easier to create sprites that tile on a 2d plane. Both
/// the `side` and `corner` sprite have to be mirrorable.
///
/// The bounds are inclusive.
pub struct Tileable2 {
    alt: i32,
    bounds: Aabr<i32>,
    center: Block,
    side: EnumMap<Dir, Block>,
    /// The corner selected is `Dir::diagonal()`.
    corner: EnumMap<Dir, Block>,
    rotation: Dir,
}

impl Tileable2 {
    pub fn empty() -> Self {
        Self {
            alt: 0,
            bounds: Aabr::default(),
            center: Block::empty(),
            side: EnumMap::from_fn(|_| Block::empty()),
            corner: EnumMap::from_fn(|_| Block::empty()),
            rotation: Dir::X,
        }
    }

    pub fn new_sprite(
        bounds: Aabr<i32>,
        alt: i32,
        center: SpriteKind,
        side: SpriteKind,
        corner: SpriteKind,
    ) -> Self {
        Self::empty()
            .with_bounds(bounds)
            .with_alt(alt)
            .with_center_sprite(center)
            .with_side_sprite(side)
            .with_corner_sprite(corner)
    }

    pub fn two_by(len: i32, pos: Vec3<i32>, dir: Dir) -> Self {
        Self::empty()
            .with_rotation(dir)
            .with_center_size(pos, Vec2::new(len, 2))
    }

    pub fn with_center_size(self, center: Vec3<i32>, size: Vec2<i32>) -> Self {
        let extent_min = (size - 1) / 2;
        let extent_max = size / 2;
        let rot = self.rotation;
        let bounds = Aabr {
            min: center.xy() - rot.vec2(extent_min.x, extent_min.y),
            max: center.xy() + rot.vec2(extent_max.x, extent_max.y),
        }
        .made_valid();
        self.with_alt(center.z).with_bounds(bounds)
    }

    /// Bounds are inclusive.
    pub fn with_bounds(mut self, bounds: Aabr<i32>) -> Self {
        self.bounds = bounds;
        self
    }

    pub fn with_alt(mut self, alt: i32) -> Self {
        self.alt = alt;
        self
    }

    pub fn with_center(mut self, block: Block) -> Self {
        self.center = block;
        self
    }

    pub fn with_center_sprite(mut self, sprite: SpriteKind) -> Self {
        self.center = self.center.with_sprite(sprite);
        self
    }

    pub fn with_side_sprite(mut self, sprite: SpriteKind) -> Self {
        for (_, block) in self.side.iter_mut() {
            *block = block.with_sprite(sprite);
        }
        self
    }

    pub fn with_side(mut self, new_block: Block) -> Self {
        for (_, block) in self.side.iter_mut() {
            *block = new_block;
        }
        self
    }

    pub fn with_side_dir(mut self, dir: Dir, sprite: SpriteKind) -> Self {
        self.side[dir] = self.side[dir].with_sprite(sprite);
        self
    }

    pub fn with_side_axis(self, axis: Dir, sprite: SpriteKind) -> Self {
        self.with_side_dir(axis, sprite)
            .with_side_dir(-axis, sprite)
    }

    pub fn with_corner_sprite(mut self, sprite: SpriteKind) -> Self {
        for (_, block) in self.corner.iter_mut() {
            *block = block.with_sprite(sprite);
        }
        self
    }

    /// The corner selected is `Dir::diagonal()`.
    pub fn with_corner_dir(mut self, dir: Dir, block: Block) -> Self {
        self.corner[dir] = block;
        self
    }

    /// The corner selected is `Dir::diagonal()`.
    pub fn with_corner_sprite_dir(mut self, dir: Dir, sprite: SpriteKind) -> Self {
        self.corner[dir] = self.corner[dir].with_sprite(sprite);
        self
    }

    pub fn with_corner_side(self, axis: Dir, sprite: Block) -> Self {
        self.with_corner_dir(axis, sprite)
            .with_corner_dir(axis.rotated_ccw(), sprite)
    }

    pub fn with_corner_sprite_side(self, axis: Dir, sprite: SpriteKind) -> Self {
        self.with_corner_sprite_dir(axis, sprite)
            .with_corner_sprite_dir(axis.rotated_ccw(), sprite)
    }

    pub fn with_rotation(mut self, dir: Dir) -> Self {
        self.rotation = dir;
        self
    }

    pub fn bounds(&self) -> Aabr<i32> { self.bounds }

    pub fn size(&self) -> Extent2<i32> { self.bounds.size() + 1 }

    pub fn center(&self) -> Block { self.center }

    pub fn side(&self, dir: Dir) -> Block { self.side[dir.relative_to(self.rotation)] }

    pub fn corner(&self, dir: Dir) -> Block { self.corner[dir.relative_to(self.rotation)] }
}

fn single_block(painter: &Painter, pos: Vec3<i32>, block: Block) {
    painter
        .aabb(Aabb {
            min: pos,
            max: pos + 1,
        })
        .fill(Fill::Sprite(block))
}

/// Only applies changes if the block can have the attributes `Ori` and
/// `MirrorX`.
fn ori_mirror(mut block: Block, dir: Dir, x: bool, y: bool) -> Block {
    let dir_res = block.get_attr::<Ori>().map(|old_ori| {
        let (old_dir, offset) =
            Dir::from_sprite_ori(old_ori.0).expect("We got this from the Ori attr");
        let new_dir = dir.relative_to(old_dir);
        Ori(new_dir.sprite_ori() + offset)
    });
    let mirror_x_res = block
        .get_attr::<MirrorX>()
        .map(|old_x| MirrorX(old_x.0 ^ x));

    if let (Ok(o), Ok(x)) = (dir_res, mirror_x_res) {
        if y {
            // flip
            block
                .set_attr(Ori((o.0 + 4) % 8))
                .expect("We read the attr");
            block.set_attr(MirrorX(!x.0)).expect("We read the attr");
        } else {
            block.set_attr(o).expect("We read the attr");
            block.set_attr(x).expect("We read the attr");
        }
    }

    block
}

pub trait PainterSpriteExt {
    fn lanternpost_wood(&self, pos: Vec3<i32>, dir: Dir);

    fn bed(
        &self,
        pos: Vec3<i32>,
        dir: Dir,
        head: SpriteKind,
        middle: SpriteKind,
        tail: SpriteKind,
    ) -> Aabr<i32> {
        let bed = Tileable2::two_by(3, pos, dir)
            .with_corner_sprite_side(Dir::Y, head)
            .with_corner_sprite_side(Dir::NegY, tail)
            .with_side_sprite(middle);
        self.tileable2(&bed);

        bed.bounds()
    }

    fn bed_wood_woodland(&self, pos: Vec3<i32>, dir: Dir) -> Aabr<i32> {
        self.bed(
            pos,
            dir,
            SpriteKind::BedWoodWoodlandHead,
            SpriteKind::BedWoodWoodlandMiddle,
            SpriteKind::BedWoodWoodlandTail,
        )
    }

    fn bed_desert(&self, pos: Vec3<i32>, dir: Dir) -> Aabr<i32> {
        self.bed(
            pos,
            dir,
            SpriteKind::BedDesertHead,
            SpriteKind::BedDesertMiddle,
            SpriteKind::BedDesertTail,
        )
    }

    fn bed_cliff(&self, pos: Vec3<i32>, dir: Dir) -> Aabr<i32> {
        self.bed(
            pos,
            dir,
            SpriteKind::BedCliffHead,
            SpriteKind::BedCliffMiddle,
            SpriteKind::BedCliffTail,
        )
    }

    fn bed_savannah(&self, pos: Vec3<i32>, dir: Dir) -> Aabr<i32> {
        self.bed(
            pos,
            dir,
            SpriteKind::BedSavannahHead,
            SpriteKind::BedSavannahMiddle,
            SpriteKind::BedSavannahTail,
        )
    }

    fn bed_coastal(&self, pos: Vec3<i32>, dir: Dir) -> Aabr<i32> {
        self.bed(
            pos,
            dir,
            SpriteKind::BedCoastalHead,
            SpriteKind::BedCoastalMiddle,
            SpriteKind::BedCoastalTail,
        )
    }

    fn table_wood_fancy_woodland(&self, pos: Vec3<i32>, axis: Dir) -> Aabr<i32> {
        let table = Tileable2::two_by(3, pos, axis)
            .with_side_sprite(SpriteKind::TableWoodFancyWoodlandBody)
            .with_corner_sprite(SpriteKind::TableWoodFancyWoodlandCorner);

        self.tileable2(&table);

        table.bounds()
    }

    /// Bounds are inclusive
    fn chairs_around(&self, chair: SpriteKind, spacing: usize, bounds: Aabr<i32>, alt: i32);

    /// Tileable in 1 dimension.
    ///
    /// Does nothing if size is less than 2.
    fn tileable1(
        &self,
        pos: Vec3<i32>,
        dir: Dir,
        size: i32,
        middle_sprite: SpriteKind,
        side_sprite: SpriteKind,
    );

    /// This will be placed with the "right side" looking forward at `pos`.
    fn mirrored2(&self, pos: Vec3<i32>, dir: Dir, sprite: SpriteKind) {
        self.tileable1(pos, dir, 2, SpriteKind::Empty, sprite);
    }

    /// Tileable in 2 dimensions.
    ///
    /// Does nothing if the size is less than 2 in any axis.
    fn tileable2(&self, tileable: &Tileable2);
}

impl PainterSpriteExt for Painter {
    fn lanternpost_wood(&self, pos: Vec3<i32>, dir: Dir) {
        let sprite_ori = dir.sprite_ori();
        self.rotated_sprite(pos, SpriteKind::LanternpostWoodBase, sprite_ori);
        self.column(pos.xy(), pos.z + 1..pos.z + 4).clear();
        self.rotated_sprite(
            pos + Vec3::unit_z() * 3,
            SpriteKind::LanternpostWoodUpper,
            sprite_ori,
        );
        self.rotated_sprite(
            pos + dir.to_vec3() + Vec3::unit_z() * 3,
            SpriteKind::LanternpostWoodLantern,
            sprite_ori,
        );
    }

    fn chairs_around(&self, chair: SpriteKind, spacing: usize, bounds: Aabr<i32>, alt: i32) {
        for dir in Dir::iter() {
            let s = dir.orthogonal().select(bounds.size());
            // We skip small sides
            if s <= 2 && dir.select(bounds.size()) > s {
                continue;
            }

            let min = dir.orthogonal().select(bounds.min);
            let max = dir.orthogonal().select(bounds.max);
            let center = dir.orthogonal().select(bounds.center());
            for i in (min..=center)
                .step_by(spacing + 1)
                .chain((center..=max).rev().step_by(spacing + 1))
            {
                let p = dir.select_aabr_with(bounds, i) + dir.to_vec2();
                single_block(
                    self,
                    p.with_z(alt),
                    Block::air(chair)
                        .with_attr(Ori(dir.opposite().sprite_ori()))
                        .expect("Chairs should have the Ori attribute"),
                );
            }
        }
    }

    fn tileable1(
        &self,
        pos: Vec3<i32>,
        dir: Dir,
        size: i32,
        middle_sprite: SpriteKind,
        side_sprite: SpriteKind,
    ) {
        if size < 2 {
            return;
        }
        let orth = dir.rotated_ccw();

        let extent_min = (size - 1) / 2;
        let extent_max = size / 2;

        let aabr = Aabr {
            min: pos.xy() - orth.to_vec2() * extent_min,
            max: pos.xy() + orth.to_vec2() * extent_max,
        }
        .made_valid();

        if size > 2 {
            self.aabb(Aabb {
                min: (aabr.min + orth.abs().to_vec2()).with_z(pos.z),
                max: (aabr.max - orth.abs().to_vec2()).with_z(pos.z) + 1,
            })
            .fill(Fill::Sprite(ori_mirror(
                Block::air(middle_sprite),
                dir,
                false,
                false,
            )));
        }

        single_block(
            self,
            aabr.min.with_z(pos.z),
            ori_mirror(Block::air(side_sprite), dir, false, orth.is_negative()),
        );

        single_block(
            self,
            aabr.max.with_z(pos.z),
            ori_mirror(Block::air(side_sprite), dir, false, orth.is_positive()),
        );
    }

    fn tileable2(&self, tileable: &Tileable2) {
        let alt = tileable.alt;
        let bounds = tileable.bounds;
        let size = tileable.size();
        if size.reduce_min() < 2 {
            // Need at least 2 in each axis to be able to tile.
            return;
        }

        if size.w > 2 && size.h > 2 {
            self.aabb(Aabb {
                min: (bounds.min + 1).with_z(alt),
                max: (bounds.max - 1).with_z(alt) + 1,
            })
            .fill(Fill::Sprite(ori_mirror(
                tileable.center(),
                tileable.rotation,
                false,
                false,
            )));
        }

        if size.h > 2 {
            let rot = Dir::NegY;
            self.aabb(Aabb {
                min: Vec3::new(bounds.min.x, bounds.min.y + 1, alt),
                max: Vec3::new(bounds.min.x, bounds.max.y - 1, alt) + 1,
            })
            .fill(Fill::Sprite(ori_mirror(
                tileable.side(Dir::NegX),
                rot,
                false,
                false,
            )));

            self.aabb(Aabb {
                min: Vec3::new(bounds.max.x, bounds.min.y + 1, alt),
                max: Vec3::new(bounds.max.x, bounds.max.y - 1, alt) + 1,
            })
            .fill(Fill::Sprite(ori_mirror(
                tileable.side(Dir::X),
                rot,
                false,
                // Mirror is applied before rotation so we mirror Y
                true,
            )));
        }

        if size.w > 2 {
            let rot = Dir::X;
            self.aabb(Aabb {
                min: Vec3::new(bounds.min.x + 1, bounds.min.y, alt),
                max: Vec3::new(bounds.max.x - 1, bounds.min.y, alt) + 1,
            })
            .fill(Fill::Sprite(ori_mirror(
                tileable.side(Dir::NegY),
                rot,
                false,
                false,
            )));

            self.aabb(Aabb {
                min: Vec3::new(bounds.min.x + 1, bounds.max.y, alt),
                max: Vec3::new(bounds.max.x - 1, bounds.max.y, alt) + 1,
            })
            .fill(Fill::Sprite(ori_mirror(
                tileable.side(Dir::Y),
                rot,
                false,
                true,
            )));
        }

        let rot = tileable.rotation;
        let orth = rot.rotated_ccw();
        single_block(
            self,
            bounds.min.with_z(alt),
            ori_mirror(
                tileable.corner(Dir::NegX),
                rot,
                rot.is_negative(),
                orth.is_negative(),
            ),
        );
        single_block(
            self,
            Vec3::new(bounds.max.x, bounds.min.y, alt),
            ori_mirror(
                tileable.corner(Dir::NegY),
                rot,
                orth.is_positive(),
                rot.is_negative(),
            ),
        );
        single_block(
            self,
            Vec3::new(bounds.min.x, bounds.max.y, alt),
            ori_mirror(
                tileable.corner(Dir::Y),
                rot,
                orth.is_negative(),
                rot.is_positive(),
            ),
        );
        single_block(
            self,
            bounds.max.with_z(alt),
            ori_mirror(
                tileable.corner(Dir::X),
                rot,
                rot.is_positive(),
                orth.is_positive(),
            ),
        );
    }
}
