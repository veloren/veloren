use crate::{
    column::ColumnSample,
    util::{RandomField, Sampler},
};
use common::{
    terrain::{Block, BlockKind},
    vol::{BaseVol, ReadVol, RectSizedVol, Vox, WriteVol},
};
use std::f32;
use vek::*;

pub fn apply_paths_to<'a>(
    wpos2d: Vec2<i32>,
    mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
    vol: &mut (impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
) {
    for y in 0..vol.size_xy().y as i32 {
        for x in 0..vol.size_xy().x as i32 {
            let offs = Vec2::new(x, y);

            let wpos2d = wpos2d + offs;

            // Sample terrain
            let col_sample = if let Some(col_sample) = get_column(offs) {
                col_sample
            } else {
                continue;
            };
            let surface_z = col_sample.riverless_alt.floor() as i32;

            let noisy_color = |col: Rgb<u8>, factor: u32| {
                let nz = RandomField::new(0).get(Vec3::new(wpos2d.x, wpos2d.y, surface_z));
                col.map(|e| {
                    (e as u32 + nz % (factor * 2))
                        .saturating_sub(factor)
                        .min(255) as u8
                })
            };

            if let Some((path_dist, path_nearest)) = col_sample.path.filter(|(dist, _)| *dist < 5.0)
            {
                let inset = 0;

                // Try to use the column at the centre of the path for sampling to make them
                // flatter
                let col_pos = (offs - wpos2d).map(|e| e as f32) + path_nearest;
                let col00 = get_column(col_pos.map(|e| e.floor() as i32) + Vec2::new(0, 0));
                let col10 = get_column(col_pos.map(|e| e.floor() as i32) + Vec2::new(1, 0));
                let col01 = get_column(col_pos.map(|e| e.floor() as i32) + Vec2::new(0, 1));
                let col11 = get_column(col_pos.map(|e| e.floor() as i32) + Vec2::new(1, 1));
                let col_attr = |col: &ColumnSample| {
                    Vec3::new(col.riverless_alt, col.alt, col.water_dist.unwrap_or(1000.0))
                };
                let [riverless_alt, alt, water_dist] = match (col00, col10, col01, col11) {
                    (Some(col00), Some(col10), Some(col01), Some(col11)) => Lerp::lerp(
                        Lerp::lerp(col_attr(col00), col_attr(col10), path_nearest.x.fract()),
                        Lerp::lerp(col_attr(col01), col_attr(col11), path_nearest.x.fract()),
                        path_nearest.y.fract(),
                    ),
                    _ => col_attr(col_sample),
                }
                .into_array();
                let (bridge_offset, depth) = (
                    ((water_dist.max(0.0) * 0.2).min(f32::consts::PI).cos() + 1.0) * 5.0,
                    ((1.0 - ((water_dist + 2.0) * 0.3).min(0.0).cos().abs())
                        * (riverless_alt + 5.0 - alt).max(0.0)
                        * 1.75
                        + 3.0) as i32,
                );
                let surface_z = (riverless_alt + bridge_offset).floor() as i32;

                for z in inset - depth..inset {
                    let _ = vol.set(
                        Vec3::new(offs.x, offs.y, surface_z + z),
                        if bridge_offset >= 2.0 && path_dist >= 3.0 || z < inset - 1 {
                            Block::new(BlockKind::Normal, noisy_color(Rgb::new(80, 80, 100), 8))
                        } else {
                            let path_color = col_sample
                                .sub_surface_color
                                .map(|e| (e * 255.0 * 0.7) as u8);
                            Block::new(BlockKind::Normal, noisy_color(path_color, 8))
                        },
                    );
                }
                let head_space = (8 - (path_dist * 0.25).powf(6.0).round() as i32).max(1);
                for z in inset..inset + head_space {
                    let pos = Vec3::new(offs.x, offs.y, surface_z + z);
                    if vol.get(pos).unwrap().kind() != BlockKind::Water {
                        let _ = vol.set(pos, Block::empty());
                    }
                }
            }
        }
    }
}
