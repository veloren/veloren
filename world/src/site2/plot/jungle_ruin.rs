use super::*;
use crate::{
    assets::AssetHandle,
    site2::gen::PrimitiveTransform,
    util::{sampler::Sampler, RandomField},
    Land,
};
use common::{
    generation::EntityInfo,
    terrain::{SpriteKind, Structure as PrefabStructure, StructuresGroup},
};
use lazy_static::lazy_static;
use rand::prelude::*;
use std::{f32::consts::TAU, sync::Arc};
use vek::*;

pub struct JungleRuin {
    bounds: Aabr<i32>,
    pub(crate) alt: i32,
}
impl JungleRuin {
    pub fn generate(land: &Land, _rng: &mut impl Rng, site: &Site, tile_aabr: Aabr<i32>) -> Self {
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        Self {
            bounds,
            alt: land.get_alt_approx(site.tile_center_wpos((tile_aabr.max - tile_aabr.min) / 2))
                as i32
                + 2,
        }
    }
}

impl Structure for JungleRuin {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_jungleruin\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_jungleruin")]
    fn render_inner(&self, _site: &Site, land: &Land, painter: &Painter) {
        let center = self.bounds.center();
        let plot_base = land.get_alt_approx(center) as i32;
        let mut thread_rng = thread_rng();
        let stone = Fill::Sampling(Arc::new(|center| {
            Some(match (RandomField::new(0).get(center)) % 52 {
                0..=8 => Block::new(BlockKind::Rock, Rgb::new(92, 99, 86)),
                9..=17 => Block::new(BlockKind::Rock, Rgb::new(83, 89, 78)),
                18..=26 => Block::new(BlockKind::Rock, Rgb::new(75, 89, 66)),
                27..=35 => Block::new(BlockKind::Rock, Rgb::new(79, 83, 73)),
                36..=44 => Block::new(BlockKind::Rock, Rgb::new(66, 80, 59)),
                _ => Block::new(BlockKind::Rock, Rgb::new(88, 94, 83)),
            })
        }));
        let stone_broken = Fill::Sampling(Arc::new(|center| {
            Some(match (RandomField::new(0).get(center)) % 56 {
                0..=8 => Block::new(BlockKind::Rock, Rgb::new(92, 99, 86)),
                9..=17 => Block::new(BlockKind::Rock, Rgb::new(83, 89, 78)),
                18..=26 => Block::new(BlockKind::Rock, Rgb::new(75, 89, 66)),
                27..=35 => Block::new(BlockKind::Rock, Rgb::new(79, 83, 73)),
                36..=44 => Block::new(BlockKind::Rock, Rgb::new(66, 80, 59)),
                45..=49 => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
                _ => Block::new(BlockKind::Rock, Rgb::new(88, 94, 83)),
            })
        }));
        let grass_fill = Fill::Sampling(Arc::new(|wpos| {
            Some(match (RandomField::new(0).get(wpos)) % 30 {
                1..=2 => Block::air(SpriteKind::ShortGrass),
                3..=7 => Block::air(SpriteKind::LongGrass),
                8 => Block::air(SpriteKind::JungleFern),
                _ => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
            })
        }));
        let mut ruin_positions = vec![];
        let pos_var = RandomField::new(0).get(center.with_z(plot_base)) % 10;
        let radius = 25 + pos_var;
        let ruins = 12.0 + pos_var as f32;
        let phi = TAU / ruins;
        for n in 1..=ruins as i32 {
            let pos = Vec2::new(
                center.x + (radius as f32 * ((n as f32 * phi).cos())) as i32,
                center.y + (radius as f32 * ((n as f32 * phi).sin())) as i32,
            );
            let base = land.get_alt_approx(pos) as i32;
            let ground_sink = RandomField::new(0).get(pos.with_z(base)) as i32 % 4;
            let ruin_pos = pos.with_z(base - 8 - ground_sink);
            ruin_positions.push(ruin_pos);
        }
        let underground_chamber = pos_var < 7;
        // center ruin and underground chest chamber
        let room_size = 10;
        let height_handle = 10;
        if underground_chamber {
            // room
            painter
                .aabb(Aabb {
                    min: (center - room_size - 1).with_z(plot_base - height_handle - room_size - 1),
                    max: (center + room_size + 1).with_z(plot_base - height_handle + room_size + 1),
                })
                .fill(stone.clone());
            painter
                .aabb(Aabb {
                    min: (center - room_size).with_z(plot_base - height_handle - room_size),
                    max: (center + room_size).with_z(plot_base - height_handle + room_size + 2),
                })
                .fill(stone_broken.clone());
            // platform
            painter
                .aabb(Aabb {
                    min: (center - room_size + 1).with_z(plot_base - height_handle + room_size + 1),
                    max: (center + room_size - 1).with_z(plot_base - height_handle + room_size + 2),
                })
                .clear();
            let center_ruin_pos = center.with_z(plot_base - 1);
            ruin_positions.push(center_ruin_pos);

            // room decor
            for d in 0..=5 {
                painter
                    .line(
                        Vec2::new(center.x, center.y - room_size + 1)
                            .with_z(plot_base - height_handle - (room_size / 3) + 3),
                        Vec2::new(center.x, center.y + room_size - 1)
                            .with_z(plot_base - height_handle - (room_size / 3) + 3),
                        (room_size - (2 * d)) as f32,
                    )
                    .fill(stone_broken.clone());
                painter
                    .line(
                        Vec2::new(center.x, center.y - room_size + 1)
                            .with_z(plot_base - height_handle - (room_size / 3) + 3),
                        Vec2::new(center.x, center.y + room_size - 1)
                            .with_z(plot_base - height_handle - (room_size / 3) + 3),
                        (room_size - 1 - (2 * d)) as f32,
                    )
                    .clear();
                painter
                    .line(
                        Vec2::new(center.x - room_size + 1, center.y)
                            .with_z(plot_base - height_handle - (room_size / 3) + 3),
                        Vec2::new(center.x + room_size - 1, center.y)
                            .with_z(plot_base - height_handle - (room_size / 3) + 3),
                        (room_size - (2 * d)) as f32,
                    )
                    .fill(stone_broken.clone());
                painter
                    .line(
                        Vec2::new(center.x - room_size + 1, center.y)
                            .with_z(plot_base - height_handle - (room_size / 3) + 3),
                        Vec2::new(center.x + room_size - 1, center.y)
                            .with_z(plot_base - height_handle - (room_size / 3) + 3),
                        (room_size - 1 - (2 * d)) as f32,
                    )
                    .clear();
            }
            // clear room
            painter
                .aabb(Aabb {
                    min: (center - room_size).with_z(plot_base - height_handle - room_size),
                    max: (center + room_size).with_z(plot_base - height_handle + room_size - 1),
                })
                .clear();
            painter
                .aabb(Aabb {
                    min: (center - room_size).with_z(plot_base - height_handle - room_size),
                    max: (center + room_size).with_z(plot_base - height_handle - room_size + 1),
                })
                .fill(stone);
            painter
                .aabb(Aabb {
                    min: (center - room_size).with_z(plot_base - height_handle - room_size + 1),
                    max: (center + room_size).with_z(plot_base - height_handle - room_size + 2),
                })
                .fill(grass_fill);
        }
        for ruin_pos in ruin_positions {
            // ruin models
            lazy_static! {
                pub static ref RUIN: AssetHandle<StructuresGroup> =
                    PrefabStructure::load_group("site_structures.jungle_ruin.jungle_ruin");
            }
            let rng = RandomField::new(0).get(ruin_pos) % 62;
            let ruin = RUIN.read();
            let ruin = ruin[rng as usize % ruin.len()].clone();
            painter
                .prim(Primitive::Prefab(Box::new(ruin.clone())))
                .translate(ruin_pos)
                .fill(Fill::Prefab(Box::new(ruin), ruin_pos, rng));
        }
        if underground_chamber {
            // entry
            painter
                .aabb(Aabb {
                    min: Vec2::new(center.x - 9, center.y - 3)
                        .with_z(plot_base - height_handle - room_size + 1),
                    max: Vec2::new(center.x - 3, center.y + 3).with_z(plot_base + 30),
                })
                .clear();
            // stairs
            painter
                .ramp(
                    Aabb {
                        min: Vec2::new(center.x - room_size, center.y - 3)
                            .with_z(plot_base - height_handle - room_size + 1),
                        max: Vec2::new(center.x, center.y + 3).with_z(plot_base),
                    },
                    Dir::NegX,
                )
                .fill(stone_broken);
            let chest_pos = Vec2::new(center.x + room_size - 2, center.y - 3)
                .with_z(plot_base - height_handle - room_size + 1);
            painter.sprite(chest_pos, SpriteKind::DungeonChest0);
        } else {
            let chest_radius = radius / 2;
            for n in 1..=(ruins / 4.0) as i32 {
                let chest_pos = Vec2::new(
                    center.x + (chest_radius as f32 * ((n as f32 * phi).cos())) as i32,
                    center.y + (chest_radius as f32 * ((n as f32 * phi).sin())) as i32,
                );
                if RandomField::new(0).get(chest_pos.with_z(plot_base)) % 2 > 0 {
                    painter.sprite(chest_pos.with_z(plot_base - 1), SpriteKind::ChestBuried);
                }
            }
        }

        // npcs
        let npc_radius = radius / 4;
        for n in 1..=(ruins / 4.0) as i32 {
            let npc_pos = Vec2::new(
                center.x + (npc_radius as f32 * ((n as f32 * phi).cos())) as i32,
                center.y + (npc_radius as f32 * ((n as f32 * phi).sin())) as i32,
            );
            match RandomField::new(0).get(center.with_z(plot_base)) % 6 {
                // grave robbers
                0 => painter.spawn(
                    EntityInfo::at(npc_pos.with_z(plot_base).as_()).with_asset_expect(
                        "common.entity.spot.dwarf_grave_robber",
                        &mut thread_rng,
                    ),
                ),
                // sauroks
                1 => painter.spawn(
                    EntityInfo::at(npc_pos.with_z(plot_base).as_())
                        .with_asset_expect("common.entity.spot.saurok", &mut thread_rng),
                ),
                // grim salvager
                2 => painter.spawn(
                    EntityInfo::at(npc_pos.with_z(plot_base).as_())
                        .with_asset_expect("common.entity.spot.grim_salvager", &mut thread_rng),
                ),
                _ => {},
            }
        }
    }
}
