use super::*;
use crate::{
    site2::{plot::dungeon::spiral_staircase, util::gradient::WrapMode},
    util::{sampler::Sampler, RandomField},
    Land,
};
use common::generation::EntityInfo;
use rand::prelude::*;
use std::{f32::consts::TAU, sync::Arc};
use vek::*;

pub struct Sahagin {
    bounds: Aabr<i32>,
    pub(crate) alt: i32,
    surface_color: Rgb<f32>,
    sub_surface_color: Rgb<f32>,
    pub(crate) center: Vec2<i32>,
    pub(crate) rooms: Vec<Vec2<i32>>,
    pub(crate) room_size: i32,
}
impl Sahagin {
    pub fn generate(
        land: &Land,
        index: IndexRef,
        _rng: &mut impl Rng,
        site: &Site,
        tile_aabr: Aabr<i32>,
    ) -> Self {
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
        let room_size = 30;
        let center = bounds.center();
        let outer_room_radius = (room_size * 2) + (room_size / 3);

        let outer_rooms = place_circular(center, outer_room_radius as f32, 5);
        let mut rooms = vec![center];
        rooms.extend(outer_rooms);

        Self {
            bounds,
            alt: CONFIG.sea_level as i32,
            surface_color,
            sub_surface_color,
            center,
            rooms,
            room_size,
        }
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            waypoints: false,
            trees: wpos.distance_squared(self.bounds.center()) > (75_i32).pow(2),
            ..SpawnRules::default()
        }
    }
}

impl Structure for Sahagin {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_sahagin\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_sahagin")]
    fn render_inner(&self, _site: &Site, _land: &Land, painter: &Painter) {
        let room_size = self.room_size;
        let center = self.center;
        let base = self.alt - room_size + 1;
        let rooms = &self.rooms;
        let mut thread_rng = thread_rng();
        let surface_color = self.surface_color.map(|e| (e * 255.0) as u8);
        let sub_surface_color = self.sub_surface_color.map(|e| (e * 255.0) as u8);
        let gradient_center = Vec3::new(center.x as f32, center.y as f32, (base + 1) as f32);
        let gradient_var_1 = RandomField::new(0).get(center.with_z(base)) as i32 % 8;
        let gradient_var_2 = RandomField::new(0).get(center.with_z(base + 1)) as i32 % 10;
        let mut random_npcs = vec![];
        let brick = Fill::Gradient(
            util::gradient::Gradient::new(
                gradient_center,
                8.0 + gradient_var_1 as f32,
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
        let jellyfish = Fill::Gradient(
            util::gradient::Gradient::new(
                gradient_center,
                8.0 + gradient_var_1 as f32,
                util::gradient::Shape::Point,
                (Rgb::new(180, 181, 227), Rgb::new(120, 160, 255)),
            )
            .with_repeat(if gradient_var_2 > 5 {
                WrapMode::Repeat
            } else {
                WrapMode::PingPong
            }),
            BlockKind::GlowingRock,
        );
        let white = Fill::Sampling(Arc::new(|center| {
            Some(match (RandomField::new(0).get(center)) % 37 {
                0..=8 => Block::new(BlockKind::Rock, Rgb::new(251, 251, 227)),
                9..=17 => Block::new(BlockKind::Rock, Rgb::new(245, 245, 229)),
                18..=26 => Block::new(BlockKind::Rock, Rgb::new(250, 243, 221)),
                27..=35 => Block::new(BlockKind::Rock, Rgb::new(240, 240, 230)),
                _ => Block::new(BlockKind::GlowingRock, Rgb::new(255, 244, 193)),
            })
        }));
        let wood = Fill::Brick(BlockKind::Wood, Rgb::new(71, 33, 11), 12);
        let key_door = Fill::Block(Block::air(SpriteKind::SahaginKeyDoor));
        let key_hole = Fill::Block(Block::air(SpriteKind::SahaginKeyhole));
        let rope = Fill::Block(Block::air(SpriteKind::Rope));
        let room_size = 30;
        let cell_size_raw = room_size / 6;
        let ground_floor = base - (room_size * 2);
        let outer_room_radius = (room_size * 2) + (room_size / 3);
        let tunnel_radius = (room_size * 3) + 6;
        let tunnel_points = place_circular(center, tunnel_radius as f32, 25);
        let scaler = -10;
        let height_handle = -room_size;
        let shell_radius = 3 * (room_size / 2) + scaler;
        let shell_carve_radius = 6 * (room_size / 2) + scaler;
        let shell_base = base + (room_size * 1) + height_handle;
        let high_carve_base = base + (room_size * 7) + height_handle;
        let low_carve_base = base + height_handle;
        let shell_carve_limiter_1 = painter.aabb(Aabb {
            min: (center - shell_radius - 6).with_z(shell_base),
            max: (center + shell_radius + 6).with_z(shell_base + (5 * shell_radius)),
        });

        let shell_carve_limiter_2 = painter.aabb(Aabb {
            min: (center - shell_radius).with_z(base + (room_size + 2) - 2),
            max: (center + shell_radius).with_z(shell_base + (5 * shell_radius)),
        });
        painter
            .cylinder_with_radius(
                center.with_z(shell_base),
                shell_radius as f32,
                5.0 * shell_radius as f32,
            )
            .intersect(shell_carve_limiter_2)
            .fill(white.clone());
        // decor bubbles
        let decor_radius = room_size / 3;
        for b in 3..=7 {
            let shell_decor = place_circular(center, (shell_radius - 2) as f32, 3 * b);

            for pos in shell_decor {
                let decor_var = 3 + RandomField::new(0).get(pos.with_z(base)) as i32 % 3;

                painter
                    .sphere_with_radius(
                        pos.with_z(shell_base + (b * (shell_radius / 2))),
                        (decor_radius - decor_var) as f32,
                    )
                    .fill(white.clone());
            }
        }
        // shell carves
        painter
            .sphere_with_radius(
                (center - room_size).with_z(high_carve_base),
                shell_carve_radius as f32,
            )
            .intersect(shell_carve_limiter_1)
            .clear();

        painter
            .sphere_with_radius(
                (center + (room_size / 2)).with_z(low_carve_base),
                shell_carve_radius as f32,
            )
            .intersect(shell_carve_limiter_2)
            .clear();
        // clear room
        painter
            .cylinder_with_radius(
                center.with_z(shell_base + (3 * (shell_radius / 2))),
                (shell_radius - 8) as f32,
                shell_radius as f32,
            )
            .clear();

        painter
            .sphere_with_radius(
                center.with_z(shell_base + (5 * (shell_radius / 2)) - 5),
                (shell_radius - 8) as f32,
            )
            .clear();
        let boss_pos = center.with_z(shell_base + (3 * (shell_radius / 2)));
        painter.spawn(EntityInfo::at(boss_pos.as_()).with_asset_expect(
            "common.entity.dungeon.sahagin.tidalwarrior",
            &mut thread_rng,
            None,
        ));
        // overground towers
        let var_towers = 32 + RandomField::new(0).get(center.with_z(base)) as i32 % 6;
        let tower_positions = place_circular(center, (5 * (room_size / 2)) as f32, var_towers);

        for tower_center_pos in tower_positions {
            for dir in CARDINALS {
                let tower_center = tower_center_pos + dir * 5;
                let var_height =
                    RandomField::new(0).get(tower_center.with_z(base)) as i32 % (room_size / 2);
                painter
                    .rounded_aabb(Aabb {
                        min: (tower_center - 10).with_z(base - room_size),
                        max: (tower_center + 10).with_z(base + (3 * (room_size / 2)) + var_height),
                    })
                    .fill(brick.clone());
            }
        }
        let bldg_base = base + room_size;
        let bldgs = var_towers / 3;
        let beam_th = 2.5;
        let bldg_positions = place_circular(center, (5 * (room_size / 2)) as f32, bldgs);
        // buildings
        for bldg_center in &bldg_positions {
            let bldg_size = ((room_size / 4) + 1)
                + RandomField::new(0).get(bldg_center.with_z(bldg_base)) as i32 % 3;
            let points = 21;
            let ring_0 = place_circular(*bldg_center, (4 * (bldg_size / 2)) as f32, points);
            let ring_1 = place_circular(*bldg_center, (9 * (bldg_size / 2)) as f32, points);
            let ring_2 = place_circular(*bldg_center, (4 * (bldg_size / 2)) as f32, points);
            let ring_3 = place_circular(*bldg_center, (2 * (bldg_size / 2)) as f32, points);

            let ring_4 = place_circular(*bldg_center, (6 * (bldg_size / 2)) as f32, points);
            let ring_5 = place_circular(*bldg_center, (4 * (bldg_size / 2)) as f32, points);
            let ring_6 = place_circular(*bldg_center, (2 * (bldg_size / 2)) as f32, points);

            for b in 0..=(ring_0.len() - 1) {
                painter
                    .cubic_bezier(
                        ring_0[b].with_z(bldg_base + (3 * (bldg_size / 2))),
                        ring_1[b].with_z(bldg_base + (5 * (bldg_size / 2))),
                        ring_2[b].with_z(bldg_base + (10 * (bldg_size / 2))),
                        ring_3[b].with_z(bldg_base + (14 * (bldg_size / 2))),
                        beam_th,
                    )
                    .fill(jellyfish.clone());
                if b == (ring_0.len() - 2) {
                    painter
                        .cubic_bezier(
                            ring_4[b].with_z(bldg_base + (12 * (bldg_size / 2))),
                            ring_5[b + 1].with_z(bldg_base + (14 * (bldg_size / 2))),
                            ring_6[0].with_z(bldg_base + (16 * (bldg_size / 2))),
                            bldg_center.with_z(bldg_base + (18 * (bldg_size / 2))),
                            beam_th,
                        )
                        .fill(jellyfish.clone());
                } else if b == (ring_0.len() - 1) {
                    painter
                        .cubic_bezier(
                            ring_4[b].with_z(bldg_base + (12 * (bldg_size / 2))),
                            ring_5[0].with_z(bldg_base + (14 * (bldg_size / 2))),
                            ring_6[1].with_z(bldg_base + (16 * (bldg_size / 2))),
                            bldg_center.with_z(bldg_base + (18 * (bldg_size / 2))),
                            beam_th,
                        )
                        .fill(jellyfish.clone());
                } else {
                    painter
                        .cubic_bezier(
                            ring_4[b].with_z(bldg_base + (12 * (bldg_size / 2))),
                            ring_5[b + 1].with_z(bldg_base + (14 * (bldg_size / 2))),
                            ring_6[b + 2].with_z(bldg_base + (16 * (bldg_size / 2))),
                            bldg_center.with_z(bldg_base + (18 * (bldg_size / 2))),
                            beam_th,
                        )
                        .fill(jellyfish.clone());
                }
            }
        }
        let key_chest_index_1 =
            RandomField::new(0).get(center.with_z(base)) as usize % bldgs as usize;
        for (p, bldg_center) in bldg_positions.iter().enumerate() {
            let bldg_size = ((room_size / 4) + 1)
                + RandomField::new(0).get(bldg_center.with_z(bldg_base)) as i32 % 3;

            // passage

            if p == (bldg_positions.len() - 1) {
                painter
                    .line(
                        bldg_positions[p].with_z(bldg_base + (5 * (bldg_size / 2))),
                        bldg_positions[0].with_z(bldg_base + (5 * (bldg_size / 2))),
                        beam_th * 2.0,
                    )
                    .clear();
            } else {
                painter
                    .line(
                        bldg_positions[p].with_z(bldg_base + (5 * (bldg_size / 2))),
                        bldg_positions[p + 1].with_z(bldg_base + (5 * (bldg_size / 2))),
                        beam_th * 2.0,
                    )
                    .clear();
            }
            // floor
            painter
                .cylinder(Aabb {
                    min: (bldg_center - (2 * bldg_size) - 2).with_z(base),
                    max: (bldg_center + (2 * bldg_size) + 2)
                        .with_z(bldg_base + (5 * (bldg_size / 2)) - 4),
                })
                .fill(brick.clone());
            let chest_pos = bldg_center - 4;
            if p == key_chest_index_1 {
                painter.sprite(
                    chest_pos.with_z(bldg_base + (9 * (bldg_size / 2))),
                    SpriteKind::SahaginChest,
                );
            }
            painter
                .cylinder(Aabb {
                    min: (chest_pos - 2).with_z(bldg_base + (9 * (bldg_size / 2)) - 1),
                    max: (chest_pos + 3).with_z(bldg_base + (9 * (bldg_size / 2))),
                })
                .fill(wood.clone());

            random_npcs.push(chest_pos.with_z(bldg_base + (9 * (bldg_size / 2)) + 1));
        }
        for bldg_center in bldg_positions {
            let bldg_size = ((room_size / 4) + 1)
                + RandomField::new(0).get(bldg_center.with_z(bldg_base)) as i32 % 3;

            // center spear
            painter
                .cylinder(Aabb {
                    min: (bldg_center - 3).with_z(bldg_base),
                    max: (bldg_center + 3).with_z(bldg_base + (20 * (bldg_size / 2))),
                })
                .fill(wood.clone());
            painter
                .cone(Aabb {
                    min: (bldg_center - 4).with_z(bldg_base + (20 * (bldg_size / 2))),
                    max: (bldg_center + 4).with_z(bldg_base + (30 * (bldg_size / 2))),
                })
                .fill(wood.clone());
        }

        // underground
        // rooms
        for room_center in rooms {
            painter
                .rounded_aabb(Aabb {
                    min: (room_center - room_size - (room_size / 2)).with_z(ground_floor),
                    max: (room_center + room_size + (room_size / 2)).with_z(base + 5),
                })
                .fill(brick.clone());
        }
        let key_chest_index_2 = RandomField::new(0).get(center.with_z(base)) as usize % rooms.len();
        for (r, room_center) in rooms.iter().enumerate() {
            painter
                .rounded_aabb(Aabb {
                    min: (room_center - room_size).with_z(ground_floor + 1),
                    max: (room_center + room_size).with_z(base - 2),
                })
                .clear();
            let cells = place_circular(*room_center, room_size as f32, room_size / 2);
            let spawns = place_circular(*room_center, (room_size + 2) as f32, room_size / 2);
            let cell_floors = (room_size / 6) - 1;
            for f in 0..cell_floors {
                let cell_floor = ground_floor + (room_size / 2) + ((cell_size_raw * 2) * f);
                for cell_pos in &cells {
                    let cell_var = RandomField::new(0).get(cell_pos.with_z(cell_floor)) as i32 % 2;
                    let cell_size = cell_size_raw + cell_var;
                    painter
                        .rounded_aabb(Aabb {
                            min: (cell_pos - cell_size).with_z(cell_floor - cell_size),
                            max: (cell_pos + cell_size).with_z(cell_floor + cell_size),
                        })
                        .clear();
                }
                for spawn_pos in &spawns {
                    painter
                        .cylinder(Aabb {
                            min: (spawn_pos - 3).with_z(cell_floor - cell_size_raw - 1),
                            max: (spawn_pos + 4).with_z(cell_floor - cell_size_raw),
                        })
                        .fill(brick.clone());
                    painter
                        .cylinder(Aabb {
                            min: (spawn_pos - 2).with_z(cell_floor - cell_size_raw),
                            max: (spawn_pos + 3).with_z(cell_floor - cell_size_raw + 1),
                        })
                        .fill(brick.clone());
                    painter
                        .cylinder(Aabb {
                            min: (spawn_pos - 1).with_z(cell_floor - cell_size_raw + 1),
                            max: (spawn_pos + 2).with_z(cell_floor - cell_size_raw + 2),
                        })
                        .fill(brick.clone());
                    painter.sprite(
                        spawn_pos.with_z(cell_floor - cell_size_raw + 2),
                        match (RandomField::new(0)
                            .get(spawn_pos.with_z(cell_floor - cell_size_raw)))
                            % 75
                        {
                            0 => SpriteKind::DungeonChest2,
                            _ => SpriteKind::FireBowlGround,
                        },
                    );

                    let npc_pos = spawn_pos.with_z(cell_floor - cell_size_raw + 3);
                    if RandomField::new(0).get(npc_pos) as i32 % 5 == 1 {
                        random_npcs.push(npc_pos);
                    }
                }
            }
            // solid floor
            painter
                .aabb(Aabb {
                    min: (room_center - room_size).with_z(ground_floor),
                    max: (room_center + room_size).with_z(ground_floor + (room_size / 3)),
                })
                .fill(brick.clone());

            for m in 0..2 {
                let mini_boss_pos = room_center.with_z(ground_floor + (room_size / 3));
                painter.spawn(
                    EntityInfo::at((mini_boss_pos + (1 * m)).as_()).with_asset_expect(
                        "common.entity.dungeon.sahagin.hakulaq",
                        &mut thread_rng,
                        None,
                    ),
                );
            }
            if r == key_chest_index_2 {
                painter.sprite(
                    (room_center - 1).with_z(ground_floor + (room_size / 3)),
                    SpriteKind::SahaginChest,
                );
            }

            let center_entry = RandomField::new(0).get(center.with_z(base)) % 4;

            if r > 0 {
                // overground - keep center clear
                let rooms_center =
                    place_circular(center, (outer_room_radius - 15) as f32, room_size / 2);
                let room_base = base - (room_size / 2) + (room_size / 2);
                for room_center in &rooms_center {
                    let room_var =
                        RandomField::new(0).get(room_center.with_z(room_base)) as i32 % 10;
                    let room_var_size = room_size - room_var;
                    painter
                        .rounded_aabb(Aabb {
                            min: (room_center - room_var_size).with_z(room_base),
                            max: (room_center + room_var_size).with_z(room_base + room_var_size),
                        })
                        .fill(brick.clone());
                }
                if r == (center_entry + 1) as usize {
                    painter
                        .line(
                            room_center.with_z(ground_floor + room_size),
                            center.with_z(ground_floor + room_size),
                            15.0,
                        )
                        .clear();
                }
            }
        }
        // tunnels
        for p in 0..tunnel_points.len() {
            if p == tunnel_points.len() - 1 {
                painter
                    .line(
                        tunnel_points[p].with_z(ground_floor + (room_size / 2)),
                        tunnel_points[0].with_z(ground_floor + (room_size / 2)),
                        5.0,
                    )
                    .clear();
            } else {
                painter
                    .line(
                        tunnel_points[p].with_z(ground_floor + (room_size / 2)),
                        tunnel_points[p + 1].with_z(ground_floor + (room_size / 2)),
                        5.0,
                    )
                    .clear();
            }
        }
        // boss room
        painter
            .rounded_aabb(Aabb {
                min: (center - room_size - 10).with_z(base - 2),
                max: (center + room_size + 10).with_z(base + room_size),
            })
            .fill(brick.clone());
        let clear_limiter = painter.aabb(Aabb {
            min: (center - room_size - 8).with_z(base + (room_size / 5)),
            max: (center + room_size + 8).with_z(base + room_size - 1),
        });
        painter
            .rounded_aabb(Aabb {
                min: (center - room_size - 8).with_z(base),
                max: (center + room_size + 8).with_z(base + room_size - 1),
            })
            .intersect(clear_limiter)
            .clear();

        // lamps
        let var_lamps = 25 + RandomField::new(0).get(center.with_z(base)) as i32 % 5;
        let lamp_positions = place_circular(center, (room_size + 5) as f32, var_lamps);

        for lamp_pos in lamp_positions {
            painter.sprite(
                lamp_pos.with_z(base + (room_size / 5)),
                SpriteKind::FireBowlGround,
            );
        }

        // top entry and stairs
        let stair_radius = room_size / 3;
        for e in 0..=1 {
            let stairs_pos = center - (room_size / 2) + ((room_size * 2) * e);
            // top entry foundation and door
            if e > 0 {
                painter
                    .rounded_aabb(Aabb {
                        min: (stairs_pos - stair_radius - 5).with_z(base - (room_size / 2)),
                        max: (stairs_pos + stair_radius + 5)
                            .with_z(base + (room_size / 5) + (3 * (room_size / 2))),
                    })
                    .fill(brick.clone());
                // door clear
                painter
                    .aabb(Aabb {
                        min: Vec2::new(stairs_pos.x - stair_radius - 8, stairs_pos.y - 2)
                            .with_z(base + (room_size / 5) + (2 * (room_size / 2))),
                        max: Vec2::new(stairs_pos.x - stair_radius, stairs_pos.y + 2)
                            .with_z(base + (room_size / 5) + (2 * (room_size / 2)) + 7),
                    })
                    .clear();
                painter
                    .aabb(Aabb {
                        min: Vec2::new(stairs_pos.x - stair_radius - 8, stairs_pos.y - 1)
                            .with_z(base + (room_size / 5) + (2 * (room_size / 2)) + 7),
                        max: Vec2::new(stairs_pos.x - stair_radius, stairs_pos.y + 1)
                            .with_z(base + (room_size / 5) + (2 * (room_size / 2)) + 8),
                    })
                    .clear();
                // door
                painter
                    .aabb(Aabb {
                        min: Vec2::new(stairs_pos.x - stair_radius - 1, stairs_pos.y - 2)
                            .with_z(base + (room_size / 5) + (2 * (room_size / 2))),
                        max: Vec2::new(stairs_pos.x - stair_radius, stairs_pos.y + 2)
                            .with_z(base + (room_size / 5) + (2 * (room_size / 2)) + 7),
                    })
                    .fill(key_door.clone());
                painter
                    .aabb(Aabb {
                        min: Vec2::new(stairs_pos.x - stair_radius - 1, stairs_pos.y - 1)
                            .with_z(base + (room_size / 5) + (2 * (room_size / 2)) + 7),
                        max: Vec2::new(stairs_pos.x - stair_radius, stairs_pos.y + 1)
                            .with_z(base + (room_size / 5) + (2 * (room_size / 2)) + 8),
                    })
                    .fill(key_door.clone());
                painter
                    .aabb(Aabb {
                        min: Vec2::new(stairs_pos.x - stair_radius - 1, stairs_pos.y)
                            .with_z(base + (room_size / 5) + (2 * (room_size / 2)) + 2),
                        max: Vec2::new(stairs_pos.x - stair_radius, stairs_pos.y + 1)
                            .with_z(base + (room_size / 5) + (2 * (room_size / 2)) + 3),
                    })
                    .fill(key_hole.clone());
                // steps
                for s in 0..4 {
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(stairs_pos.x - stair_radius - 2 - s, stairs_pos.y - 2)
                                .with_z(base + (room_size / 5) + (2 * (room_size / 2)) - 1 - s),
                            max: Vec2::new(stairs_pos.x - stair_radius - 1 - s, stairs_pos.y + 2)
                                .with_z(base + (room_size / 5) + (2 * (room_size / 2)) + 7),
                        })
                        .clear();
                }
            } else {
                // boss entry 1
                // tube
                painter
                    .cylinder(Aabb {
                        min: (stairs_pos - stair_radius - 4).with_z(base + (room_size / 5)),
                        max: (stairs_pos + stair_radius + 4).with_z(base + room_size - 2),
                    })
                    .fill(brick.clone());
                painter
                    .cylinder(Aabb {
                        min: (stairs_pos - stair_radius).with_z(base + (room_size / 5)),
                        max: (stairs_pos + stair_radius).with_z(base + room_size - 3),
                    })
                    .clear();
                // door clear
                painter
                    .aabb(Aabb {
                        min: Vec2::new(stairs_pos.x - stair_radius - 3, stairs_pos.y - 2)
                            .with_z(base + (room_size / 5) + 2),
                        max: Vec2::new(stairs_pos.x - stair_radius, stairs_pos.y + 2)
                            .with_z(base + (room_size / 5) + 9),
                    })
                    .clear();
                painter
                    .aabb(Aabb {
                        min: Vec2::new(stairs_pos.x - stair_radius - 3, stairs_pos.y - 1)
                            .with_z(base + (room_size / 5) + 9),
                        max: Vec2::new(stairs_pos.x - stair_radius, stairs_pos.y + 1)
                            .with_z(base + (room_size / 5) + 10),
                    })
                    .clear();
                // door
                painter
                    .aabb(Aabb {
                        min: Vec2::new(stairs_pos.x - stair_radius - 4, stairs_pos.y - 2)
                            .with_z(base + (room_size / 5) + 2),
                        max: Vec2::new(stairs_pos.x - stair_radius - 3, stairs_pos.y + 2)
                            .with_z(base + (room_size / 5) + 9),
                    })
                    .fill(key_door.clone());
                painter
                    .aabb(Aabb {
                        min: Vec2::new(stairs_pos.x - stair_radius - 4, stairs_pos.y - 1)
                            .with_z(base + (room_size / 5) + 9),
                        max: Vec2::new(stairs_pos.x - stair_radius - 3, stairs_pos.y + 1)
                            .with_z(base + (room_size / 5) + 10),
                    })
                    .fill(key_door.clone());
                painter
                    .aabb(Aabb {
                        min: Vec2::new(stairs_pos.x - stair_radius - 4, stairs_pos.y)
                            .with_z(base + (room_size / 5) + 3),
                        max: Vec2::new(stairs_pos.x - stair_radius - 3, stairs_pos.y + 1)
                            .with_z(base + (room_size / 5) + 4),
                    })
                    .fill(key_hole.clone());
            }

            let stairs_clear = painter.cylinder(Aabb {
                min: (stairs_pos - stair_radius).with_z(ground_floor + (room_size / 3)),
                max: (stairs_pos + stair_radius)
                    .with_z(base + (room_size / 5) + (((3 * (room_size / 2)) - 6) * e)),
            });
            stairs_clear.clear();
            stairs_clear
                .sample(spiral_staircase(
                    stairs_pos.with_z(ground_floor + (room_size / 3)),
                    (stair_radius + 1) as f32,
                    2.5,
                    (room_size - 5) as f32,
                ))
                .fill(wood.clone());
        }

        // boss entry 2
        let boss_entry_pos = center + (room_size / 3);
        let rope_pos = center + (room_size / 3) - 2;
        let spike_pos = center + (room_size / 3) - 1;

        painter
            .cylinder(Aabb {
                min: (boss_entry_pos - stair_radius).with_z(base + room_size - 5),
                max: (boss_entry_pos + stair_radius).with_z(base + (room_size * 2) - 10),
            })
            .fill(wood.clone());
        painter
            .cylinder(Aabb {
                min: (boss_entry_pos - 3).with_z(base + (room_size * 2) - 10),
                max: (boss_entry_pos + 4).with_z(base + (room_size * 2) - 7),
            })
            .fill(wood.clone());
        painter
            .cylinder(Aabb {
                min: (boss_entry_pos - 2).with_z(base + room_size - 5),
                max: (boss_entry_pos + 3).with_z(base + (room_size * 2) - 7),
            })
            .clear();

        painter
            .aabb(Aabb {
                min: rope_pos.with_z(base + (room_size / 4) + 2),
                max: (rope_pos + 1).with_z(base + room_size - 5),
            })
            .fill(rope.clone());

        painter
            .cylinder(Aabb {
                min: (spike_pos - 3).with_z(base + (room_size * 2) - 7),
                max: (spike_pos + 4).with_z(base + (room_size * 2) - 5),
            })
            .fill(Fill::Block(Block::air(SpriteKind::IronSpike)));
        // top room npcs
        let npc_pos = boss_entry_pos;
        let amount = 4 + RandomField::new(0).get(npc_pos.with_z(base)) as i32 % 4;
        for a in 0..amount {
            random_npcs.push((npc_pos + a).with_z(base + (room_size / 4)))
        }
        for m in 0..2 {
            painter.spawn(
                EntityInfo::at(((npc_pos - m).with_z(base + (room_size / 4))).as_())
                    .with_asset_expect(
                        "common.entity.dungeon.sahagin.hakulaq",
                        &mut thread_rng,
                        None,
                    ),
            );
        }
        // room npcs
        for m in 0..2 {
            let mini_boss_pos = center.with_z(base + room_size + 5);
            painter.spawn(
                EntityInfo::at((mini_boss_pos + (1 * m)).as_()).with_asset_expect(
                    "common.entity.dungeon.sahagin.hakulaq",
                    &mut thread_rng,
                    None,
                ),
            );
        }

        for pos in random_npcs {
            let entities = [
                "common.entity.dungeon.sahagin.sniper",
                "common.entity.dungeon.sahagin.sniper",
                "common.entity.dungeon.sahagin.sniper",
                "common.entity.dungeon.sahagin.sorcerer",
                "common.entity.dungeon.sahagin.spearman",
            ];
            let npc = entities[(RandomField::new(0).get(pos) % entities.len() as u32) as usize];
            painter.spawn(EntityInfo::at(pos.as_()).with_asset_expect(npc, &mut thread_rng, None));
        }
    }
}

fn place_circular(center: Vec2<i32>, radius: f32, amount: i32) -> Vec<Vec2<i32>> {
    let phi = TAU / amount as f32;
    let mut positions = vec![];
    for n in 1..=amount {
        let pos = Vec2::new(
            center.x + (radius * ((n as f32 * phi).cos())) as i32,
            center.y + (radius * ((n as f32 * phi).sin())) as i32,
        );
        positions.push(pos);
    }
    positions
}
