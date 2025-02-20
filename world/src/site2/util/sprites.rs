use crate::site2::Painter;

use super::Dir;
use common::terrain::SpriteKind;
use vek::*;

pub trait PainterSpriteExt {
    fn lanternpost_wood(&self, pos: Vec3<i32>, dir: Dir);
}

impl PainterSpriteExt for Painter {
    fn lanternpost_wood(&self, pos: Vec3<i32>, dir: Dir) {
        let sprite_ori = dir.rotated_cw().sprite_ori();
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
}
