use super::*;
use crate::{
    Land,
    assets::AssetHandle,
    site::gen::{PrimitiveTransform, place_circular, place_circular_as_vec},
    util::{DIAGONALS, LOCALITY, NEIGHBORS, RandomField, sampler::Sampler, within_distance},
};
use common::{
    generation::EntityInfo,
    terrain::{Structure as PrefabStructure, StructuresGroup},
};
use lazy_static::lazy_static;
use rand::prelude::*;
use std::{f32::consts::PI, sync::Arc};
use vek::*;

pub struct Haniwa {
    base: i32,
    diameter: i32,
    tree_pos: Vec3<i32>,
    room_size: i32,
    rotation: f32,
    pub(crate) alt: i32,
    pub(crate) center: Vec2<i32>,
    pub(crate) entrance_pos: Vec3<i32>,
    pub(crate) mob_room_positions: Vec<Vec3<i32>>,
    pub(crate) center_room_positions: Vec<Vec3<i32>>,
    pub(crate) room_positions: Vec<Vec3<i32>>,
    pub(crate) boss_room_position: Vec3<i32>,
    pub(crate) mini_boss_room_positions: Vec<Vec3<i32>>,
}
impl Haniwa {
    pub fn generate(land: &Land, _rng: &mut impl Rng, site: &Site, tile_aabr: Aabr<i32>) -> Self {
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        let center = bounds.center();
        let base = land.get_alt_approx(center) as i32;
        let diameter = (150 + RandomField::new(0).get(center.with_z(base)) % 10) as i32;
        let dir_select = (RandomField::new(0).get(center.with_z(base)) % 4) as usize;
        let rotation = (PI / 2.0) * dir_select as f32;
        let entrance_dir = CARDINALS[dir_select];
        let tree_dir = -entrance_dir;
        let entrance_pos = (center + (entrance_dir * (2 * (diameter / 3)))).with_z(base);
        let tree_distance = diameter / 2;
        let tree_pos = (center + (tree_dir * tree_distance)).with_z(base);
        let room_size = diameter / 4;
        let mut floors = vec![];
        for f in 1..=3 {
            let floor = base - ((diameter / 5) * f) - 2;
            floors.push(floor)
        }
        let mut mob_room_positions = vec![];
        let mut mini_boss_room_positions = vec![];
        let mut center_room_positions = vec![];
        let mut room_positions = vec![];
        for floor in floors.iter().take(floors.len() - 1) {
            let (room_distribution, room_distance) =
                if RandomField::new(0).get(center.with_z(*floor)) % 2 > 0 {
                    (DIAGONALS, (3 * (room_size / 2)) + 2)
                } else {
                    (CARDINALS, 2 * (room_size - 2))
                };
            for dir in room_distribution {
                let room_center = center + dir * room_distance;
                mob_room_positions.push(room_center.with_z(floor - (room_size / 4) + 1));
                room_positions.push(room_center.with_z(floor - (room_size / 4) + 1));
            }
            mini_boss_room_positions.push(center.with_z(floor - (room_size / 4) + 1));
        }
        for floor in &floors {
            center_room_positions.push(center.with_z(floor - (room_size / 4) + 1));
            room_positions.push(center.with_z(floor - (room_size / 4) + 1));
        }
        let boss_room_position = center_room_positions[center_room_positions.len() - 1];
        Self {
            alt: land.get_alt_approx(site.tile_center_wpos(tile_aabr.center())) as i32 + 2,
            center,
            base,
            diameter,
            entrance_pos,
            tree_pos,
            room_size,
            rotation,
            room_positions,
            mob_room_positions,
            center_room_positions,
            boss_room_position,
            mini_boss_room_positions,
        }
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            waypoints: false,
            trees: !within_distance(wpos, self.center, self.diameter),
            ..SpawnRules::default()
        }
    }
}

impl Structure for Haniwa {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_haniwa\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "render_haniwa"))]
    fn render_inner(&self, _site: &Site, _land: &Land, painter: &Painter) {
        let center = self.center;
        let base = self.base;
        let diameter = self.diameter;
        let entrance = self.entrance_pos;
        let mut thread_rng = thread_rng();
        let rock = Fill::Brick(BlockKind::Rock, Rgb::new(96, 123, 131), 24);
        let key_door = Fill::Block(Block::air(SpriteKind::HaniwaKeyDoor));
        let key_hole = Fill::Block(Block::air(SpriteKind::HaniwaKeyhole));
        let trap = Fill::Block(Block::air(SpriteKind::HaniwaTrap));
        let rock_broken = Fill::Sampling(Arc::new(|center| {
            Some(match (RandomField::new(0).get(center)) % 48 {
                0..=8 => Block::new(BlockKind::Rock, Rgb::new(97, 124, 134)),
                9..=17 => Block::new(BlockKind::Rock, Rgb::new(92, 118, 128)),
                18..=26 => Block::new(BlockKind::Rock, Rgb::new(85, 111, 121)),
                27..=35 => Block::new(BlockKind::Rock, Rgb::new(82, 108, 117)),
                36..=40 => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
                _ => Block::new(BlockKind::Rock, Rgb::new(96, 123, 131)),
            })
        }));
        let lanterns = Fill::Sampling(Arc::new(|center| {
            Some(match (RandomField::new(0).get(center)) % 200 {
                0 => Block::air(SpriteKind::FireBowlGround),
                _ => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
            })
        }));
        let iron_spikes = Fill::Block(Block::air(SpriteKind::IronSpike));
        let rock_iron_spikes = Fill::Sampling(Arc::new(|center| {
            Some(match (RandomField::new(0).get(center)) % 100 {
                0..=8 => Block::new(BlockKind::Rock, Rgb::new(97, 124, 134)),
                9..=17 => Block::new(BlockKind::Rock, Rgb::new(92, 118, 128)),
                18..=26 => Block::new(BlockKind::Rock, Rgb::new(85, 111, 121)),
                27..=35 => Block::new(BlockKind::Rock, Rgb::new(82, 108, 117)),
                36..=40 => Block::new(BlockKind::Rock, Rgb::new(96, 123, 131)),
                _ => Block::air(SpriteKind::IronSpike),
            })
        }));
        let grass = Fill::Brick(BlockKind::Rock, Rgb::new(72, 87, 22), 24);
        // room npcs
        let npcs = [
            "common.entity.dungeon.haniwa.guard",
            "common.entity.dungeon.haniwa.soldier",
        ];
        let height_handle = diameter / 8;
        let cone_length = 8;
        let cone_radius = (diameter / 2) as f32;
        let cones = diameter / 4;
        let cone_positions = place_circular(center, cone_radius, cones);
        // tree platform
        let outside_radius = cone_radius as i32;
        let tree_pos = Vec2::new(self.tree_pos.x, self.tree_pos.y);
        let platform_size = 23;
        painter
            .cylinder(Aabb {
                min: (tree_pos - platform_size).with_z(base - 5),
                max: (tree_pos + platform_size).with_z(base - 10 + platform_size - 1),
            })
            .fill(rock_broken.clone());
        painter
            .cylinder(Aabb {
                min: (tree_pos - platform_size).with_z(base - 10 + platform_size - 1),
                max: (tree_pos + platform_size).with_z(base - 10 + platform_size),
            })
            .fill(grass.clone());
        let carve_positions = place_circular(tree_pos, platform_size as f32, 15);
        for carve_pos in carve_positions {
            painter
                .line(
                    carve_pos.with_z(base + 2),
                    carve_pos.with_z(base - 5 + platform_size),
                    3.5,
                )
                .clear();
        }
        // gangway
        painter
            .ramp(
                Aabb {
                    min: Vec2::new(center.x - outside_radius, center.y - 16).with_z(base + 10),
                    max: Vec2::new(center.x + (outside_radius / 2) + 8, center.y + 16)
                        .with_z(base + 28),
                },
                Dir::X,
            )
            .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
            .fill(grass.clone());
        painter
            .ramp(
                Aabb {
                    min: Vec2::new(center.x - outside_radius, center.y - 16).with_z(base + 9),
                    max: Vec2::new(center.x + (outside_radius / 2) + 8, center.y + 16)
                        .with_z(base + 27),
                },
                Dir::X,
            )
            .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
            .fill(rock_broken.clone());

        let clear_dist_x = outside_radius / 4;
        let clear_dist_y = (2 * outside_radius) + (outside_radius / 10);
        let clear_radius = 2 * outside_radius;
        let clear_limiter = painter
            .aabb(Aabb {
                min: Vec2::new(center.x - outside_radius, center.y - 16).with_z(base + 9),
                max: Vec2::new(center.x + (outside_radius / 2) + 8, center.y + 16)
                    .with_z(base + 28),
            })
            .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base));
        for c in 0..=1 {
            let clear_pos = Vec2::new(
                center.x - clear_dist_x,
                center.y - clear_dist_y + (c * (clear_dist_y * 2)),
            );
            painter
                .cylinder(Aabb {
                    min: (clear_pos - clear_radius).with_z(base + 9),
                    max: (clear_pos + clear_radius).with_z(base + 28),
                })
                .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
                .intersect(clear_limiter)
                .clear();
        }

        // decor cones
        for position in cone_positions {
            for dir in LOCALITY {
                let cone_pos = position + (dir * 2);
                let cone_var = 10 + RandomField::new(0).get(cone_pos.with_z(base)) as i32 % 10;
                painter
                    .cone_with_radius(
                        cone_pos.with_z(base),
                        (cone_length / 3) as f32,
                        (cone_length + cone_var) as f32,
                    )
                    .fill(rock_broken.clone());
            }
        }
        // repaint platform grass layer
        painter
            .cylinder(Aabb {
                min: (tree_pos - platform_size + 5).with_z(base - 10 + platform_size - 1),
                max: (tree_pos + platform_size - 5).with_z(base - 10 + platform_size),
            })
            .fill(grass.clone());
        // clear platform upwards
        painter
            .cylinder(Aabb {
                min: (tree_pos - platform_size).with_z(base - 10 + platform_size),
                max: (tree_pos + platform_size).with_z(base + platform_size),
            })
            .clear();
        // main sphere
        let sphere_limiter = painter.aabb(Aabb {
            min: (center - diameter - 1).with_z(base - 2),
            max: (center + diameter + 1).with_z(base + height_handle + 1),
        });
        painter
            .sphere(Aabb {
                min: (center - diameter - 1).with_z(base - (2 * diameter) + height_handle - 1),
                max: (center + diameter + 1).with_z(base + height_handle + 1),
            })
            .intersect(sphere_limiter)
            .fill(grass.clone());
        painter
            .sphere(Aabb {
                min: (center - diameter + 1).with_z(base - (2 * diameter) + height_handle + 1),
                max: (center + diameter - 1).with_z(base + height_handle - 1),
            })
            .intersect(sphere_limiter)
            .fill(rock.clone());

        // decor grass ring
        let ring_radius = cone_radius as i32 + 4;
        painter
            .cylinder(Aabb {
                min: (center - ring_radius).with_z(base),
                max: (center + ring_radius).with_z(base + 4),
            })
            .fill(grass.clone());
        let beams = cones + 8;
        let beam_start_radius = cone_radius + 6_f32;
        let beam_end_radius = cone_radius + 20_f32;
        let beam_start = place_circular_as_vec(center, beam_start_radius, beams);
        let beam_end = place_circular_as_vec(center, beam_end_radius, beams);
        let room_size = self.room_size;

        for b in 0..beams {
            painter
                .line(
                    beam_start[b as usize].with_z(base + 2),
                    beam_end[b as usize].with_z(base + 1),
                    2.5,
                )
                .fill(grass.clone());
        }
        // entrance terrain clear
        let entrance_clear = Vec2::new(entrance.x, entrance.y);
        for c in 0..8 {
            painter
                .aabb(Aabb {
                    min: (entrance_clear - 4 - c).with_z(base + c),
                    max: (entrance_clear + 4 + c).with_z(base + 1 + c),
                })
                .clear();
        }
        // entrance tunnel
        painter
            .vault(
                Aabb {
                    min: Vec2::new(center.x + (diameter / 4), center.y - 12).with_z(base - 5),
                    max: Vec2::new(center.x + (diameter / 2) + 9, center.y + 12).with_z(base + 22),
                },
                Dir::X,
            )
            .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
            .fill(rock_broken.clone());
        for v in 1..=5 {
            painter
                .vault(
                    Aabb {
                        min: Vec2::new(center.x + (diameter / 2) + 8 + v, center.y - 10 + v)
                            .with_z(base - 5),
                        max: Vec2::new(center.x + (diameter / 2) + 9 + v, center.y + 10 - v)
                            .with_z(base + 22 - v),
                    },
                    Dir::X,
                )
                .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
                .fill(rock_broken.clone());
        }
        painter
            .vault(
                Aabb {
                    min: Vec2::new(center.x + (diameter / 4), center.y - 4).with_z(base),
                    max: Vec2::new(center.x + (diameter / 2) + 25, center.y + 4).with_z(base + 16),
                },
                Dir::X,
            )
            .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
            .clear();
        // entrance lanterns
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x + (diameter / 4), center.y - 4).with_z(base),
                max: Vec2::new(center.x + (diameter / 2) + 15, center.y + 4).with_z(base + 1),
            })
            .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
            .fill(lanterns.clone());
        // floor 0
        painter
            .cylinder(Aabb {
                min: (center - beam_end_radius as i32 - 3).with_z(base - (diameter / 4) - 3),
                max: (center + beam_end_radius as i32 + 3).with_z(base),
            })
            .fill(rock.clone());
        // entrance trap
        painter
            .cylinder(Aabb {
                min: Vec2::new(center.x + (diameter / 3) - 3, center.y - 2).with_z(base - 1),
                max: Vec2::new(center.x + (diameter / 3) + 3, center.y + 3).with_z(base),
            })
            .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
            .fill(trap.clone());

        // room hulls
        for rooms in &self.room_positions {
            let room_center = Vec2::new(rooms.x, rooms.y);
            let floor = rooms.z + (room_size / 4) + 1;
            painter
                .superquadric(
                    Aabb {
                        min: (room_center - room_size - 2).with_z(floor - (room_size / 4) - 2),
                        max: (room_center + room_size + 2).with_z(floor + (room_size / 4) + 2),
                    },
                    4.0,
                )
                .fill(rock.clone());
            painter
                .superquadric(
                    Aabb {
                        min: (room_center - room_size - 1).with_z(floor - (room_size / 4) - 1),
                        max: (room_center + room_size + 1).with_z(floor + (room_size / 4) + 1),
                    },
                    4.0,
                )
                .fill(rock_broken.clone());
        }
        for rooms in &self.room_positions {
            // clear rooms
            let room_center = Vec2::new(rooms.x, rooms.y);
            let floor = rooms.z + (room_size / 4) + 1;
            painter
                .superquadric(
                    Aabb {
                        min: (room_center - room_size).with_z(floor - (room_size / 4)),
                        max: (room_center + room_size).with_z(floor + (room_size / 4)),
                    },
                    4.0,
                )
                .clear();
            // room floor
            painter
                .aabb(Aabb {
                    min: (room_center - room_size).with_z(floor - (room_size / 4) - 2),
                    max: (room_center + room_size).with_z(floor - (room_size / 4) + 5),
                })
                .fill(rock.clone());
            // room lanterns
            painter
                .aabb(Aabb {
                    min: (room_center - room_size + 7).with_z(floor - (room_size / 4) + 5),
                    max: (room_center + room_size - 7).with_z(floor - (room_size / 4) + 6),
                })
                .fill(lanterns.clone());
        }
        // mob rooms
        for rooms in &self.mob_room_positions {
            let room_center = Vec2::new(rooms.x, rooms.y);
            let floor = rooms.z + (room_size / 4) + 1;
            // inner room
            painter
                .aabb(Aabb {
                    min: (room_center - (2 * (room_size / 3)) - 1)
                        .with_z(floor - (room_size / 4) + 5),
                    max: (room_center + (2 * (room_size / 3)) + 1).with_z(floor + (room_size / 4)),
                })
                .fill(rock_broken.clone());
            painter
                .aabb(Aabb {
                    min: (room_center - (2 * (room_size / 3)) + 1)
                        .with_z(floor - (room_size / 4) + 5),
                    max: (room_center + (2 * (room_size / 3)) - 1)
                        .with_z(floor + (room_size / 4) - 1),
                })
                .clear();
            // inner room entries
            painter
                .vault(
                    Aabb {
                        min: Vec2::new(
                            room_center.x - (2 * (room_size / 3)) - 1,
                            room_center.y - 4,
                        )
                        .with_z(floor - (room_size / 4) + 5),
                        max: Vec2::new(
                            room_center.x + (2 * (room_size / 3)) + 1,
                            room_center.y + 4,
                        )
                        .with_z(floor + (room_size / 8)),
                    },
                    Dir::X,
                )
                .clear();
            painter
                .vault(
                    Aabb {
                        min: Vec2::new(
                            room_center.x - 4,
                            room_center.y - (2 * (room_size / 3)) - 1,
                        )
                        .with_z(floor - (room_size / 4) + 5),
                        max: Vec2::new(
                            room_center.x + 4,
                            room_center.y + (2 * (room_size / 3)) + 1,
                        )
                        .with_z(floor + (room_size / 8)),
                    },
                    Dir::Y,
                )
                .clear();
            // room lanterns inner
            painter
                .aabb(Aabb {
                    min: (room_center - (2 * (room_size / 3)) + 1)
                        .with_z(floor - (room_size / 4) + 5),
                    max: (room_center + (2 * (room_size / 3)) - 1)
                        .with_z(floor - (room_size / 4) + 6),
                })
                .fill(lanterns.clone());
            for dir in DIAGONALS {
                let pillar_pos = room_center + dir * ((2 * (room_size / 3)) - 6);
                let stair_pos = Vec2::new(
                    room_center.x + dir.x * ((2 * (room_size / 3)) - 10),
                    room_center.y + dir.y * ((2 * (room_size / 3)) - 3),
                );
                if (RandomField::new(0).get((pillar_pos).with_z(base)) % 3) > 0 {
                    // stairs
                    painter
                        .line(
                            Vec2::new(room_center.x, stair_pos.y)
                                .with_z(floor - (room_size / 4) - 1),
                            stair_pos.with_z(floor - (room_size / 4) + 6),
                            4.0,
                        )
                        .fill(rock.clone());
                    painter
                        .cylinder(Aabb {
                            min: (pillar_pos - 6).with_z(floor - (room_size / 4) + 5),
                            max: (pillar_pos + 6).with_z(floor - (room_size / 4) + 6),
                        })
                        .fill(rock.clone());
                    painter
                        .cylinder(Aabb {
                            min: (pillar_pos - 6).with_z(floor - (room_size / 4) + 6),
                            max: (pillar_pos + 6).with_z(floor - (room_size / 4) + 7),
                        })
                        .fill(iron_spikes.clone());
                    painter
                        .cylinder(Aabb {
                            min: (pillar_pos - 5).with_z(floor - (room_size / 4) + 5),
                            max: (pillar_pos + 5).with_z(floor - (room_size / 4) + 9),
                        })
                        .fill(rock_broken.clone());
                    painter
                        .cylinder(Aabb {
                            min: (pillar_pos - 6).with_z(floor - (room_size / 4) + 9),
                            max: (pillar_pos + 7).with_z(floor - (room_size / 4) + 10),
                        })
                        .fill(rock.clone());
                    // chests
                    if (RandomField::new(0).get((pillar_pos).with_z(base)) % 5) == 0 {
                        painter
                            .aabb(Aabb {
                                min: pillar_pos.with_z(floor - (room_size / 4) + 9),
                                max: (pillar_pos + 1).with_z(floor - (room_size / 4) + 10),
                            })
                            .fill(rock.clone());
                        painter
                            .aabb(Aabb {
                                min: pillar_pos.with_z(floor - (room_size / 4) + 10),
                                max: (pillar_pos + 1).with_z(floor - (room_size / 4) + 11),
                            })
                            .fill(Fill::Block(Block::air(SpriteKind::DungeonChest3)));
                    }
                    // room npcs
                    for _ in 0..=thread_rng.gen_range(0..2) {
                        // archers on pillars
                        painter.spawn(
                            EntityInfo::at(pillar_pos.with_z(floor - (room_size / 4) + 11).as_())
                                .with_asset_expect(
                                    "common.entity.dungeon.haniwa.archer",
                                    &mut thread_rng,
                                    None,
                                ),
                        );
                    }
                }
            }
            for n in 0..=thread_rng.gen_range(2..=4) {
                let select =
                    (RandomField::new(0).get((room_center + n).with_z(floor)) % 2) as usize;
                painter.spawn(
                    EntityInfo::at(
                        (room_center + 10 + n)
                            .with_z(floor - (room_size / 4) + 6)
                            .as_(),
                    )
                    .with_asset_expect(npcs[select], &mut thread_rng, None),
                )
            }
            let effigy_pos = (room_center - 8).with_z(floor - (room_size / 4) + 6);
            if (RandomField::new(0).get(effigy_pos) % 2) as usize > 0 {
                painter.spawn(EntityInfo::at(effigy_pos.as_()).with_asset_expect(
                    "common.entity.dungeon.haniwa.ancienteffigy",
                    &mut thread_rng,
                    None,
                ));
            }
            // room chest
            match RandomField::new(0).get(room_center.with_z(base)) as i32 % 3 {
                0 => {
                    for dir in CARDINALS {
                        let sentry_pos = room_center + dir * 10;
                        painter.spawn(
                            EntityInfo::at(sentry_pos.with_z(floor - (room_size / 4) + 6).as_())
                                .with_asset_expect(
                                    "common.entity.dungeon.haniwa.sentry",
                                    &mut thread_rng,
                                    None,
                                ),
                        )
                    }
                    painter
                        .aabb(Aabb {
                            min: (room_center - 1).with_z(floor - (room_size / 4) + 5),
                            max: (room_center + 2).with_z(floor - (room_size / 4) + 6),
                        })
                        .fill(rock.clone());
                    painter
                        .aabb(Aabb {
                            min: room_center.with_z(floor - (room_size / 4) + 6),
                            max: (room_center + 1).with_z(floor - (room_size / 4) + 7),
                        })
                        .fill(Fill::Block(Block::air(SpriteKind::HaniwaUrn)));
                },
                1 => {
                    painter
                        .cylinder(Aabb {
                            min: (room_center - 8).with_z(floor - (room_size / 4) + 5),
                            max: (room_center + 8).with_z(floor - (room_size / 4) + 6),
                        })
                        .fill(rock_iron_spikes.clone());
                    painter
                        .cylinder(Aabb {
                            min: (room_center - 7).with_z(floor - (room_size / 4) + 5),
                            max: (room_center + 7).with_z(floor - (room_size / 4) + 6),
                        })
                        .fill(rock.clone());
                    painter
                        .cylinder(Aabb {
                            min: (room_center - 7).with_z(floor - (room_size / 4) + 6),
                            max: (room_center + 7).with_z(floor - (room_size / 4) + 7),
                        })
                        .fill(rock_iron_spikes.clone());
                    painter
                        .cylinder(Aabb {
                            min: (room_center - 6).with_z(floor - (room_size / 4) + 6),
                            max: (room_center + 6).with_z(floor - (room_size / 4) + 7),
                        })
                        .fill(rock.clone());
                    painter
                        .cylinder(Aabb {
                            min: (room_center - 6).with_z(floor - (room_size / 4) + 7),
                            max: (room_center + 6).with_z(floor - (room_size / 4) + 8),
                        })
                        .fill(rock_iron_spikes.clone());
                    painter
                        .aabb(Aabb {
                            min: (room_center - 1).with_z(floor - (room_size / 4) + 7),
                            max: (room_center + 2).with_z(floor - (room_size / 4) + 8),
                        })
                        .fill(rock.clone());
                    painter
                        .aabb(Aabb {
                            min: room_center.with_z(floor - (room_size / 4) + 8),
                            max: (room_center + 1).with_z(floor - (room_size / 4) + 9),
                        })
                        .fill(Fill::Block(Block::air(SpriteKind::HaniwaUrn)));
                },
                _ => {
                    for c in 0..=7 {
                        painter
                            .cylinder(Aabb {
                                min: (room_center - 10 + c).with_z(floor - (room_size / 4) + 4),
                                max: (room_center + 10 - c).with_z(floor - (room_size / 4) + 5),
                            })
                            .fill(match c {
                                0 | 2 | 4 | 6 => trap.clone(),
                                _ => rock.clone(),
                            });
                    }

                    painter
                        .aabb(Aabb {
                            min: (room_center - 1).with_z(floor - (room_size / 4) + 5),
                            max: (room_center + 2).with_z(floor - (room_size / 4) + 6),
                        })
                        .fill(rock.clone());
                    painter
                        .aabb(Aabb {
                            min: room_center.with_z(floor - (room_size / 4) + 6),
                            max: (room_center + 1).with_z(floor - (room_size / 4) + 7),
                        })
                        .fill(Fill::Block(Block::air(SpriteKind::HaniwaUrn)));
                },
            }
        }
        // center rooms
        for rooms in &self.center_room_positions {
            let floor = rooms.z + (room_size / 4) + 1;
            // room decor
            for dir in NEIGHBORS {
                let position = center + dir * 20;
                for p in 0..4 {
                    painter
                        .aabb(Aabb {
                            min: (position - 1 - p).with_z(floor - (room_size / 4) + 5 + (4 * p)),
                            max: (position + 2 + p).with_z(floor - (room_size / 4) + 9 + (4 * p)),
                        })
                        .fill(rock_broken.clone());
                }
                for t in 0..2 {
                    let trap_pos = center + (dir * (12 + (t * 14)));
                    if RandomField::new(0).get((trap_pos).with_z(floor)) % 3 < 1 {
                        painter
                            .aabb(Aabb {
                                min: (trap_pos - 1).with_z(floor - (room_size / 4) + 4),
                                max: (trap_pos + 1).with_z(floor - (room_size / 4) + 5),
                            })
                            .fill(trap.clone());
                        painter
                            .aabb(Aabb {
                                min: (trap_pos - 1).with_z(floor - (room_size / 4) + 5),
                                max: (trap_pos + 1).with_z(floor - (room_size / 4) + 6),
                            })
                            .clear();
                    }
                }
            }
        }
        // center room stairs
        for f in 0..(self.center_room_positions.len() - 1) {
            let floor = self.center_room_positions[f].z + (room_size / 4) + 1;
            let stairs_floor = floor - 5;
            let stairs_start =
                Vec2::new(center.x - (diameter / 6) + 2, center.y - (diameter / 6) + 7);
            for s in 0..(diameter / 5) {
                painter
                    .vault(
                        Aabb {
                            min: Vec2::new(stairs_start.x + s - 1, stairs_start.y - 4)
                                .with_z(stairs_floor - s - 2),
                            max: Vec2::new(stairs_start.x + s, stairs_start.y + 4)
                                .with_z(stairs_floor + 12 - s + 2),
                        },
                        Dir::X,
                    )
                    .fill(rock_broken.clone());
            }
            for s in 0..(diameter / 5) {
                painter
                    .vault(
                        Aabb {
                            min: Vec2::new(stairs_start.x + s - 1, stairs_start.y - 2)
                                .with_z(stairs_floor - s),
                            max: Vec2::new(stairs_start.x + s, stairs_start.y + 2)
                                .with_z(stairs_floor + 10 - s),
                        },
                        Dir::X,
                    )
                    .clear();
                painter
                    .aabb(Aabb {
                        min: Vec2::new(stairs_start.x + s - 1, stairs_start.y - 2)
                            .with_z(stairs_floor - 1 - s),
                        max: Vec2::new(stairs_start.x + s, stairs_start.y + 2)
                            .with_z(stairs_floor - s),
                    })
                    .fill(rock.clone());
                painter
                    .aabb(Aabb {
                        min: Vec2::new(stairs_start.x + s - 1, stairs_start.y - 2)
                            .with_z(stairs_floor - s),
                        max: Vec2::new(stairs_start.x + s, stairs_start.y + 2)
                            .with_z(stairs_floor + 1 - s),
                    })
                    .fill(lanterns.clone());

                let doors = [0, 8, 16, 24];
                if doors.contains(&s) {
                    painter
                        .vault(
                            Aabb {
                                min: Vec2::new(stairs_start.x + s - 1, stairs_start.y - 2)
                                    .with_z(stairs_floor - s),
                                max: Vec2::new(stairs_start.x + s, stairs_start.y + 2)
                                    .with_z(stairs_floor + 10 - s),
                            },
                            Dir::X,
                        )
                        .fill(key_door.clone());
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(stairs_start.x + s - 1, stairs_start.y)
                                .with_z(stairs_floor + 2 - s),
                            max: Vec2::new(stairs_start.x + s, stairs_start.y + 1)
                                .with_z(stairs_floor + 3 - s),
                        })
                        .fill(key_hole.clone());
                }
            }
        }

        // stair case
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x + 3, center.y - 6).with_z(base - (diameter / 4) - 5),
                max: Vec2::new(center.x + 14, center.y + 6).with_z(base - (diameter / 4) + 15),
            })
            .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
            .fill(rock_broken.clone());
        // tunnel stairs
        for s in 0..((diameter / 4) - 3) {
            painter
                .vault(
                    Aabb {
                        min: Vec2::new(center.x + (diameter / 4) - s - 1, center.y - 5)
                            .with_z(base - s - 1),
                        max: Vec2::new(center.x + (diameter / 4) - s, center.y + 5)
                            .with_z(base + 16 - s + 1),
                    },
                    Dir::X,
                )
                .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
                .fill(rock_broken.clone());

            painter
                .vault(
                    Aabb {
                        min: Vec2::new(center.x + (diameter / 4) - s - 1, center.y - 4)
                            .with_z(base - s),
                        max: Vec2::new(center.x + (diameter / 4) - s, center.y + 4)
                            .with_z(base + 16 - s),
                    },
                    Dir::X,
                )
                .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
                .clear();
            painter
                .aabb(Aabb {
                    min: Vec2::new(center.x + (diameter / 4) - s - 1, center.y - 4)
                        .with_z(base - 1 - s),
                    max: Vec2::new(center.x + (diameter / 4) - s, center.y + 4).with_z(base - s),
                })
                .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
                .fill(rock.clone());
            painter
                .aabb(Aabb {
                    min: Vec2::new(center.x + (diameter / 4) - s - 1, center.y - 4)
                        .with_z(base - s),
                    max: Vec2::new(center.x + (diameter / 4) - s, center.y + 4)
                        .with_z(base + 1 - s),
                })
                .rotate_about(Mat3::rotation_z(self.rotation).as_(), center.with_z(base))
                .fill(lanterns.clone());
        }

        // tree model
        lazy_static! {
            pub static ref MODEL: AssetHandle<StructuresGroup> =
                PrefabStructure::load_group("site_structures.haniwa.bonsai");
        }
        let model_pos = tree_pos.with_z(base - 10 + platform_size);
        let rng = RandomField::new(0).get(model_pos) % 10;
        let model = MODEL.read();
        let model = model[rng as usize % model.len()].clone();
        painter
            .prim(Primitive::Prefab(Box::new(model.clone())))
            .translate(model_pos)
            .fill(Fill::Prefab(Box::new(model), model_pos, rng));

        // mini_bosses
        let golem_pos = Vec3::new(
            self.mini_boss_room_positions[0].x,
            self.mini_boss_room_positions[0].y,
            self.mini_boss_room_positions[0].z + 5,
        );
        painter.spawn(EntityInfo::at(golem_pos.as_()).with_asset_expect(
            "common.entity.dungeon.haniwa.claygolem",
            &mut thread_rng,
            None,
        ));
        // mid_boss
        let mid_boss = [
            "common.entity.dungeon.haniwa.general",
            "common.entity.dungeon.haniwa.claysteed",
        ];
        for npc in mid_boss.iter() {
            painter.spawn(
                EntityInfo::at(
                    Vec3::new(
                        self.mini_boss_room_positions[1].x,
                        self.mini_boss_room_positions[1].y,
                        self.mini_boss_room_positions[1].z + 5,
                    )
                    .as_(),
                )
                .with_asset_expect(npc, &mut thread_rng, None),
            );
        }
        // boss
        painter.spawn(
            EntityInfo::at(
                Vec3::new(
                    self.boss_room_position.x,
                    self.boss_room_position.y,
                    self.boss_room_position.z + 5,
                )
                .as_(),
            )
            .with_asset_expect(
                "common.entity.dungeon.haniwa.gravewarden",
                &mut thread_rng,
                None,
            ),
        );
        let bonerattler_pos = (center + ((entrance - center) / 3)).with_z(base);
        for _ in 0..(1 + RandomField::new(0).get(center.with_z(base)) % 2) as i32 {
            painter.spawn(EntityInfo::at(bonerattler_pos.as_()).with_asset_expect(
                "common.entity.dungeon.haniwa.claysteed",
                &mut thread_rng,
                None,
            ))
        }
    }
}
