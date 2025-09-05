use super::*;
use crate::{
    ColumnSample, Land,
    site::{generation::PrimitiveTransform, util::gradient::WrapMode},
    util::{RandomField, Sampler},
};
use common::terrain::{
    Block, BlockKind, SpriteKind, Structure as PrefabStructure, sprite::RelativeNeighborPosition,
};
use rand::prelude::*;
use vek::*;

pub struct Barn {
    /// Tile position of the door tile
    pub door_tile: Vec2<i32>,
    /// Axis aligned bounding region for the house
    bounds: Aabr<i32>,
    /// Approximate altitude of the door tile
    pub(crate) alt: i32,
    is_desert: bool,
    surface_color: Rgb<f32>,
    sub_surface_color: Rgb<f32>,
}

impl Barn {
    pub fn generate(
        land: &Land,
        index: IndexRef,
        _rng: &mut impl Rng,
        site: &Site,
        door_tile: Vec2<i32>,
        door_dir: Vec2<i32>,
        tile_aabr: Aabr<i32>,
        is_desert: bool,
    ) -> Self {
        let door_tile_pos = site.tile_center_wpos(door_tile);
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        let (surface_color, sub_surface_color) =
            if let Some(sample) = land.column_sample(bounds.center(), index) {
                (sample.surface_color, sample.sub_surface_color)
            } else {
                (Rgb::new(161.0, 116.0, 86.0), Rgb::new(88.0, 64.0, 64.0))
            };

        Self {
            door_tile: door_tile_pos,
            bounds,
            alt: land.get_alt_approx(site.tile_center_wpos(door_tile + door_dir)) as i32 + 2,
            is_desert,
            surface_color,
            sub_surface_color,
        }
    }
}

impl Structure for Barn {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_barn\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "render_barn"))]
    fn render_inner(&self, _site: &Site, _land: &Land, painter: &Painter) {
        let base = self.alt;
        let plot_center = self.bounds.center();

        // blend the tiles below the barn with neighboring tiles for a more
        // natural look, this is roughly similar to the way cliff_tower does it
        let surface_color = self.surface_color.map(|e| (e * 255.0) as u8);
        let sub_surface_color = self.sub_surface_color.map(|e| (e * 255.0) as u8);
        let gradient_center = Vec3::new(
            plot_center.x as f32,
            plot_center.y as f32,
            (base - 1) as f32,
        );
        let gradient_var_1 = RandomField::new(0).get(plot_center.with_z(base - 1)) as i32 % 8;
        let gradient_var_2 = RandomField::new(0).get(plot_center.with_z(base)) as i32 % 10;

        let brick = Fill::Gradient(
            util::gradient::Gradient::new(
                gradient_center,
                12.0 + gradient_var_1 as f32,
                util::gradient::Shape::Point,
                (surface_color, sub_surface_color),
            )
            .with_repeat(if gradient_var_2 > 5 {
                WrapMode::Repeat
            } else {
                WrapMode::PingPong
            }),
            BlockKind::Rock,
        );

        let barn_path = "site_structures.plot_structures.barn";
        let asset_handle = PrefabStructure::load_group(barn_path);
        let barn_prefab_structure = asset_handle.read()[0].clone();
        let barn_prefab_structure_bounds = barn_prefab_structure.get_bounds();
        let barn_length = barn_prefab_structure_bounds.max.x - barn_prefab_structure_bounds.min.x;
        let barn_half_length = barn_length / 2;
        let barn_width = barn_prefab_structure_bounds.max.y - barn_prefab_structure_bounds.min.y;
        let barn_half_width = barn_width / 2;
        let barn_height = barn_prefab_structure_bounds.max.z - barn_prefab_structure_bounds.min.z;

        painter
            .aabb(Aabb {
                min: Vec2::new(
                    plot_center.x - barn_half_length,
                    plot_center.y - barn_half_width,
                )
                .with_z(base - 10),
                max: Vec2::new(
                    plot_center.x + barn_half_length,
                    plot_center.y + barn_half_width,
                )
                .with_z(base - 1),
            })
            .fill(brick.clone());

        // air to clear any hills that appear inside of the the barn
        let air = Fill::Block(Block::new(BlockKind::Air, Rgb::new(255, 255, 255)));

        painter
            .aabb(Aabb {
                min: Vec2::new(
                    plot_center.x - barn_half_length,
                    plot_center.y - barn_half_width,
                )
                .with_z(base),
                max: Vec2::new(
                    plot_center.x + barn_half_length,
                    plot_center.y + barn_half_width,
                )
                .with_z(base + barn_height),
            })
            .fill(air);

        // barn prefab
        let entrance_pos: Vec3<i32> = (plot_center.x, plot_center.y, self.alt).into();
        let barn_site_pos: Vec3<i32> = entrance_pos + Vec3::new(0, 0, -1);

        // Render the prefab
        painter
            .prim(Primitive::Prefab(Box::new(barn_prefab_structure.clone())))
            .translate(barn_site_pos)
            .fill(Fill::Prefab(
                Box::new(barn_prefab_structure),
                barn_site_pos,
                0,
            ));
    }

    fn terrain_surface_at<R: Rng>(
        &self,
        wpos: Vec2<i32>,
        old: Block,
        _rng: &mut R,
        col: &ColumnSample,
        z_off: i32,
        _site: &Site,
    ) -> Option<Block> {
        let hit_min_x_bounds = wpos.x == self.bounds.min.x;
        let hit_min_y_bounds = wpos.y == self.bounds.min.y;
        let hit_max_x_bounds = wpos.x == self.bounds.max.x - 1;
        let hit_max_y_bounds = wpos.y == self.bounds.max.y - 1;

        let is_bounds =
            hit_min_x_bounds || hit_min_y_bounds || hit_max_x_bounds || hit_max_y_bounds;

        let is_corner = (hit_max_y_bounds || hit_min_y_bounds)
            && (hit_max_x_bounds || hit_min_x_bounds)
            && is_bounds;

        if z_off == 0 {
            // soil
            Some(Block::new(
                if self.is_desert {
                    BlockKind::Sand
                } else {
                    BlockKind::Grass
                },
                (Lerp::lerp(
                    col.surface_color,
                    col.sub_surface_color * 0.5,
                    false as i32 as f32,
                ) * 255.0)
                    .as_(),
            ))
        } else if z_off == 1 && is_bounds {
            // fence
            let adjacent_type = if is_corner {
                RelativeNeighborPosition::L
            } else {
                RelativeNeighborPosition::I
            };

            let ori = if !is_corner {
                // for straight - "I"
                // can only go in the vertical or horizontal direction
                if hit_min_x_bounds || hit_max_x_bounds {
                    2
                } else {
                    0
                }
            } else {
                // for corners - "L"
                // can be rotated in 4 different directions
                if hit_min_x_bounds && hit_min_y_bounds {
                    4
                } else if hit_max_x_bounds && hit_min_y_bounds {
                    6
                } else if hit_min_x_bounds && hit_max_y_bounds {
                    2
                } else {
                    0
                }
            };

            Some(
                old.into_vacant()
                    .with_sprite(SpriteKind::FenceWoodWoodland)
                    .with_ori(ori)
                    .unwrap()
                    .with_adjacent_type(adjacent_type)
                    .unwrap(),
            )
        } else {
            None
        }
    }
}
