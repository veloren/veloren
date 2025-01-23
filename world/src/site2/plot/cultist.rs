use super::*;
use crate::{
    Land,
    site2::gen::{inscribed_polystar, place_circular},
    util::{DIAGONALS, RandomField, sampler::Sampler},
};
use common::{
    comp::misc::PortalData,
    generation::{EntityInfo, SpecialEntity},
    resources::Secs,
    terrain::SpriteKind,
};
use rand::prelude::*;
use std::sync::Arc;
use vek::*;

pub struct Room {
    room_base: i32,
    room_center: Vec2<i32>,
    clear_center: Vec2<i32>,
    mob_room: bool,
    boss_room: bool,
    portal_to_boss: bool,
}

pub struct Cultist {
    base: i32,
    bounds: Aabr<i32>,
    pub(crate) alt: i32,
    pub(crate) center: Vec2<i32>,
    pub(crate) room_data: Vec<Room>,
    room_size: i32,
    floors: i32,
}
impl Cultist {
    pub fn generate(land: &Land, _rng: &mut impl Rng, site: &Site, tile_aabr: Aabr<i32>) -> Self {
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        let center = bounds.center();
        let base = land.get_alt_approx(center) as i32;
        let room_size = 30;
        let mut room_data = vec![];

        let floors = 3;
        for f in 0..=floors {
            for s in 0..=1 {
                // rooms
                let rooms = [1, 2];
                let boss_portal_floor =
                    (1 + (RandomField::new(0).get(center.with_z(base + 1)) % 2)) as i32;
                let portal_to_boss_index =
                    (RandomField::new(0).get(center.with_z(base)) % 4) as usize;
                if rooms.contains(&f) {
                    for (d, dir) in DIAGONALS.iter().enumerate() {
                        let room_base = base - (f * (2 * (room_size))) - (s * room_size);
                        let room_center = center + (dir * ((room_size * 2) - 5 + (10 * s)));
                        let clear_center = center + (dir * ((room_size * 2) - 6 + (10 * s)));
                        let mob_room = s < 1;
                        let portal_to_boss =
                            mob_room && d == portal_to_boss_index && f == boss_portal_floor;
                        room_data.push(Room {
                            room_base,
                            room_center,
                            clear_center,
                            mob_room,
                            boss_room: false,
                            portal_to_boss,
                        });
                    }
                }
            }
        }
        let boss_room_base = base - (6 * room_size);
        room_data.push(Room {
            room_base: boss_room_base,
            room_center: center,
            clear_center: center,
            mob_room: false,
            boss_room: true,
            portal_to_boss: false,
        });

        Self {
            bounds,
            alt: land.get_alt_approx(site.tile_center_wpos((tile_aabr.max - tile_aabr.min) / 2))
                as i32
                + 2,
            base,
            center,
            room_size,
            room_data,
            floors,
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

impl Structure for Cultist {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_cultist\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_cultist")]
    fn render_inner(&self, _site: &Site, _land: &Land, painter: &Painter) {
        let center = self.center;
        let base = self.base;
        let room_size = self.room_size;
        let floors = self.floors;
        let mut thread_rng = thread_rng();
        let candles_lite = Fill::Sampling(Arc::new(|wpos| {
            Some(match (RandomField::new(0).get(wpos)) % 30 {
                0 => Block::air(SpriteKind::Candle),
                _ => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
            })
        }));

        let mut tower_positions = vec![];
        let mut clear_positions = vec![];
        let room_data = &self.room_data;
        let mut star_positions = vec![];
        let mut sprite_positions = vec![];
        let mut random_npcs = vec![];

        let rock_broken = Fill::Sampling(Arc::new(|center| {
            Some(match (RandomField::new(0).get(center)) % 52 {
                0..=8 => Block::new(BlockKind::Rock, Rgb::new(60, 55, 65)),
                9..=17 => Block::new(BlockKind::Rock, Rgb::new(65, 60, 70)),
                18..=26 => Block::new(BlockKind::Rock, Rgb::new(70, 65, 75)),
                27..=35 => Block::new(BlockKind::Rock, Rgb::new(75, 70, 80)),
                36..=37 => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
                _ => Block::new(BlockKind::Rock, Rgb::new(55, 50, 60)),
            })
        }));
        let rock = Fill::Brick(BlockKind::Rock, Rgb::new(55, 50, 60), 24);
        let water = Fill::Block(Block::new(BlockKind::Water, Rgb::zero()));
        let key_door = Fill::Block(Block::air(SpriteKind::KeyDoor));
        let key_hole = Fill::Block(Block::air(SpriteKind::Keyhole));
        let gold_chain = Fill::Block(Block::air(SpriteKind::SeaDecorChain));

        for room in room_data {
            let (room_base, room_center, mob_room) =
                (room.room_base, room.room_center, room.mob_room);
            // rooms
            // encapsulation
            painter
                .aabb(Aabb {
                    min: (room_center - room_size - 23).with_z(room_base - room_size - 1),
                    max: (room_center + room_size + 23).with_z(room_base - 2),
                })
                .fill(rock.clone());
            if mob_room {
                painter
                    .aabb(Aabb {
                        min: (room_center - room_size - 2).with_z(room_base - room_size - 1),
                        max: (room_center + room_size + 2).with_z(room_base - 2),
                    })
                    .fill(rock_broken.clone());
            }
            // solid floor
            painter
                .aabb(Aabb {
                    min: (room_center - room_size - 10).with_z(room_base - room_size - 2),
                    max: (room_center + room_size + 10).with_z(room_base - room_size - 1),
                })
                .fill(rock.clone());
            // floor candles
            painter
                .cylinder(Aabb {
                    min: (room_center - room_size + 1).with_z(room_base - room_size - 1),
                    max: (room_center + room_size - 1).with_z(room_base - room_size),
                })
                .fill(candles_lite.clone());
        }

        for s in 0..=floors {
            let room_base = base - (s * (2 * room_size));

            // center pit
            for p in 3..=5 {
                let pos = 3 * p;
                let radius = pos * 2;
                let amount = pos;
                let clear_radius = radius - 8;
                let tower_pos = place_circular(center, radius as f32, amount);
                let clear_pos = place_circular(center, clear_radius as f32, amount);
                tower_positions.extend(tower_pos);
                clear_positions.extend(clear_pos);
            }
            for tower_center in &tower_positions {
                let height_var =
                    (RandomField::new(0).get(tower_center.with_z(room_base)) % 15) as i32;
                let height = height_var * 3;
                let size = height_var / 3;

                // towers

                // encapsulation if under ground
                if room_base < base {
                    painter
                        .aabb(Aabb {
                            min: (tower_center - 9 - size).with_z(room_base - 2),
                            max: (tower_center + 9 + size).with_z(room_base + 10 + height),
                        })
                        .fill(rock.clone());
                    painter
                        .aabb(Aabb {
                            min: (tower_center - 9 - (size / 2)).with_z(room_base + 8 + height),
                            max: (tower_center + 9 + (size / 2))
                                .with_z(room_base + 10 + height + 5 + (height / 2)),
                        })
                        .fill(rock.clone());
                }
                painter
                    .aabb(Aabb {
                        min: (tower_center - 8 - size).with_z(room_base),
                        max: (tower_center + 8 + size).with_z(room_base + 10 + height),
                    })
                    .fill(rock_broken.clone());
                painter
                    .aabb(Aabb {
                        min: (tower_center - 8 - (size / 2)).with_z(room_base + 10 + height),
                        max: (tower_center + 8 + (size / 2))
                            .with_z(room_base + 10 + height + 5 + (height / 2)),
                    })
                    .fill(rock_broken.clone());

                // vault carves floor 0
                painter
                    .vault(
                        Aabb {
                            min: Vec2::new(tower_center.x - 8 - size, tower_center.y - 4 - size)
                                .with_z(room_base + size),
                            max: Vec2::new(tower_center.x + 8 + size, tower_center.y + 4 + size)
                                .with_z(room_base + height),
                        },
                        Dir::X,
                    )
                    .clear();

                painter
                    .vault(
                        Aabb {
                            min: Vec2::new(tower_center.x - 4 - size, tower_center.y - 8 - size)
                                .with_z(room_base + size),
                            max: Vec2::new(tower_center.x + 4 + size, tower_center.y + 8 + size)
                                .with_z(room_base + height),
                        },
                        Dir::Y,
                    )
                    .clear();
                // vault carves floor 1
                painter
                    .vault(
                        Aabb {
                            min: Vec2::new(
                                tower_center.x - 8 - (size / 2),
                                tower_center.y - 4 - (size / 2),
                            )
                            .with_z(room_base + 10 + height),
                            max: Vec2::new(
                                tower_center.x + 8 + (size / 2),
                                tower_center.y + 4 + (size / 2),
                            )
                            .with_z(room_base + 10 + height + 5 + (height / 4) + (size / 2)),
                        },
                        Dir::X,
                    )
                    .clear();

                painter
                    .vault(
                        Aabb {
                            min: Vec2::new(
                                tower_center.x - 4 - (size / 2),
                                tower_center.y - 8 - (size / 2),
                            )
                            .with_z(room_base + 10 + height),
                            max: Vec2::new(
                                tower_center.x + 4 + (size / 2),
                                tower_center.y + 8 + (size / 2),
                            )
                            .with_z(room_base + 10 + height + 5 + (height / 4) + (size / 2)),
                        },
                        Dir::Y,
                    )
                    .clear();
            }
            // tower clears
            for (tower_center, clear_center) in tower_positions.iter().zip(&clear_positions) {
                let height_var =
                    (RandomField::new(0).get(tower_center.with_z(room_base)) % 15) as i32;
                let height = height_var * 3;
                let size = height_var / 3;
                // tower clears
                painter
                    .aabb(Aabb {
                        min: (clear_center - 9 - size).with_z(room_base + size),
                        max: (clear_center + 9 + size).with_z(room_base + 8 + height),
                    })
                    .clear();
                painter
                    .aabb(Aabb {
                        min: (clear_center - 8 - (size / 2)).with_z(room_base + 10 + height),
                        max: (clear_center + 8 + (size / 2))
                            .with_z(room_base + 8 + height + 5 + (height / 2)),
                    })
                    .clear();

                // decay
                let decay_size = 8 + size;
                painter
                    .cylinder(Aabb {
                        min: (clear_center - decay_size).with_z(room_base + 8 + height),
                        max: (clear_center + decay_size).with_z(room_base + 10 + height),
                    })
                    .clear();

                painter
                    .cylinder(Aabb {
                        min: (clear_center - decay_size + 5)
                            .with_z(room_base + 8 + height + 5 + (height / 2)),
                        max: (clear_center + decay_size - 5)
                            .with_z(room_base + 10 + height + 5 + (height / 2)),
                    })
                    .clear();
            }

            // center clear
            painter
                .cylinder(Aabb {
                    min: (center - room_size + 10).with_z(room_base),
                    max: (center + room_size - 10).with_z(room_base + (4 * (room_size))),
                })
                .clear();
        }
        // room clears
        for room in room_data {
            let (room_base, room_center, clear_center, mob_room, boss_room, portal_to_boss) = (
                room.room_base,
                room.room_center,
                room.clear_center,
                room.mob_room,
                room.boss_room,
                room.portal_to_boss,
            );
            painter
                .cylinder(Aabb {
                    min: (clear_center - room_size - 1).with_z(room_base - room_size),
                    max: (clear_center + room_size + 1).with_z(room_base - 4),
                })
                .clear();

            // room decor
            let decor_var = RandomField::new(0).get(room_center.with_z(room_base)) % 4;
            if mob_room {
                // room_center platform or water basin
                if decor_var < 3 {
                    painter
                        .aabb(Aabb {
                            min: (room_center - room_size + 10).with_z(room_base - room_size - 1),
                            max: (room_center + room_size - 10).with_z(room_base - 2),
                        })
                        .fill(rock_broken.clone());
                }

                // carves
                let spacing = 12;
                let carve_length = room_size + 8;
                let carve_width = 3;
                for f in 0..3 {
                    for c in 0..5 {
                        // candles & chest & npcs
                        let sprite_pos_1 = Vec2::new(
                            room_center.x - room_size + (spacing / 2) + (spacing * c) - carve_width
                                + 2,
                            room_center.y - carve_length + 2,
                        )
                        .with_z(room_base - room_size - 1 + ((room_size / 3) * f));
                        sprite_positions.push(sprite_pos_1);

                        let sprite_pos_2 = Vec2::new(
                            room_center.x - room_size + (spacing / 2) + (spacing * c) + carve_width
                                - 2,
                            room_center.y + carve_length - 2,
                        )
                        .with_z(room_base - room_size - 1 + ((room_size / 3) * f));
                        sprite_positions.push(sprite_pos_2);

                        let sprite_pos_3 = Vec2::new(
                            room_center.x - carve_length + 2,
                            room_center.y - room_size + (spacing / 2) + (spacing * c) - carve_width
                                + 2,
                        )
                        .with_z(room_base - room_size - 1 + ((room_size / 3) * f));
                        sprite_positions.push(sprite_pos_3);

                        let sprite_pos_4 = Vec2::new(
                            room_center.x + carve_length - 2,
                            room_center.y - room_size + (spacing / 2) + (spacing * c) + carve_width
                                - 2,
                        )
                        .with_z(room_base - room_size - 1 + ((room_size / 3) * f));
                        sprite_positions.push(sprite_pos_4);

                        let candle_limiter = painter.aabb(Aabb {
                            min: (room_center - room_size + 10)
                                .with_z(room_base - room_size - 2 + ((room_size / 3) * f)),
                            max: (room_center + room_size - 10)
                                .with_z(room_base - room_size + ((room_size / 3) * f)),
                        });

                        painter
                            .vault(
                                Aabb {
                                    min: Vec2::new(
                                        room_center.x - room_size + (spacing / 2) + (spacing * c)
                                            - carve_width,
                                        room_center.y - carve_length,
                                    )
                                    .with_z(room_base - room_size - 1 + ((room_size / 3) * f)),
                                    max: Vec2::new(
                                        room_center.x - room_size
                                            + (spacing / 2)
                                            + (spacing * c)
                                            + carve_width,
                                        room_center.y + carve_length,
                                    )
                                    .with_z(
                                        room_base - room_size - 3
                                            + (room_size / 3)
                                            + ((room_size / 3) * f),
                                    ),
                                },
                                Dir::Y,
                            )
                            .clear();

                        painter
                            .aabb(Aabb {
                                min: Vec2::new(
                                    room_center.x - room_size + (spacing / 2) + (spacing * c)
                                        - carve_width,
                                    room_center.y - carve_length,
                                )
                                .with_z(room_base - room_size - 2 + ((room_size / 3) * f)),
                                max: Vec2::new(
                                    room_center.x - room_size
                                        + (spacing / 2)
                                        + (spacing * c)
                                        + carve_width,
                                    room_center.y + carve_length,
                                )
                                .with_z(room_base - room_size - 1 + ((room_size / 3) * f)),
                            })
                            .intersect(candle_limiter)
                            .fill(rock.clone());
                        painter
                            .aabb(Aabb {
                                min: Vec2::new(
                                    room_center.x - room_size + (spacing / 2) + (spacing * c)
                                        - carve_width,
                                    room_center.y - carve_length,
                                )
                                .with_z(room_base - room_size - 1 + ((room_size / 3) * f)),
                                max: Vec2::new(
                                    room_center.x - room_size
                                        + (spacing / 2)
                                        + (spacing * c)
                                        + carve_width,
                                    room_center.y + carve_length,
                                )
                                .with_z(room_base - room_size + ((room_size / 3) * f)),
                            })
                            .intersect(candle_limiter)
                            .fill(candles_lite.clone());

                        painter
                            .vault(
                                Aabb {
                                    min: Vec2::new(
                                        room_center.x - carve_length,
                                        room_center.y - room_size + (spacing / 2) + (spacing * c)
                                            - carve_width,
                                    )
                                    .with_z(room_base - room_size - 1 + ((room_size / 3) * f)),
                                    max: Vec2::new(
                                        room_center.x + carve_length,
                                        room_center.y - room_size
                                            + (spacing / 2)
                                            + (spacing * c)
                                            + carve_width,
                                    )
                                    .with_z(
                                        room_base - room_size - 3
                                            + (room_size / 3)
                                            + ((room_size / 3) * f),
                                    ),
                                },
                                Dir::X,
                            )
                            .clear();

                        painter
                            .aabb(Aabb {
                                min: Vec2::new(
                                    room_center.x - carve_length,
                                    room_center.y - room_size + (spacing / 2) + (spacing * c)
                                        - carve_width,
                                )
                                .with_z(room_base - room_size - 2 + ((room_size / 3) * f)),
                                max: Vec2::new(
                                    room_center.x + carve_length,
                                    room_center.y - room_size
                                        + (spacing / 2)
                                        + (spacing * c)
                                        + carve_width,
                                )
                                .with_z(room_base - room_size - 1 + ((room_size / 3) * f)),
                            })
                            .intersect(candle_limiter)
                            .fill(rock.clone());
                        painter
                            .aabb(Aabb {
                                min: Vec2::new(
                                    room_center.x - carve_length,
                                    room_center.y - room_size + (spacing / 2) + (spacing * c)
                                        - carve_width,
                                )
                                .with_z(room_base - room_size - 1 + ((room_size / 3) * f)),
                                max: Vec2::new(
                                    room_center.x + carve_length,
                                    room_center.y - room_size
                                        + (spacing / 2)
                                        + (spacing * c)
                                        + carve_width,
                                )
                                .with_z(room_base - room_size + ((room_size / 3) * f)),
                            })
                            .intersect(candle_limiter)
                            .fill(candles_lite.clone());
                    }
                    // mob room npcs
                    for dir in CARDINALS {
                        for d in 1..=4 {
                            let npc_pos = (room_center + dir * ((spacing / 2) * d))
                                .with_z(room_base - room_size + ((room_size / 3) * f));
                            let pos_var = RandomField::new(0).get(npc_pos) % 10;
                            if pos_var < 2 {
                                painter.spawn(EntityInfo::at(npc_pos.as_()).with_asset_expect(
                                    "common.entity.dungeon.cultist.cultist",
                                    &mut thread_rng,
                                    None,
                                ))
                            } else if pos_var > 2 && f > 0 {
                                painter.sphere_with_radius(npc_pos, 5_f32).clear();
                            }
                        }
                    }
                    let decor_var = RandomField::new(0).get(room_center.with_z(room_base)) % 4;
                    if decor_var < 3 {
                        // portal platform
                        painter
                            .cylinder(Aabb {
                                min: (room_center - 3).with_z(room_base - (room_size / 4) - 5),
                                max: (room_center + 3).with_z(room_base - (room_size / 4) - 4),
                            })
                            .fill(rock.clone());
                    } else {
                        painter
                            .cylinder(Aabb {
                                min: (room_center - room_size + 10)
                                    .with_z(room_base - room_size - 3),
                                max: (room_center + room_size - 10)
                                    .with_z(room_base - room_size - 2),
                            })
                            .fill(rock.clone());
                        painter
                            .cylinder(Aabb {
                                min: (room_center - room_size + 10)
                                    .with_z(room_base - room_size - 2),
                                max: (room_center + room_size - 10)
                                    .with_z(room_base - room_size - 1),
                            })
                            .fill(water.clone());
                        painter
                            .aabb(Aabb {
                                min: (room_center - room_size + 10)
                                    .with_z(room_base - room_size - 1),
                                max: (room_center + room_size - 10)
                                    .with_z(room_base - (room_size / 4) - 3),
                            })
                            .clear();
                    }
                }
            }
            // room portals
            let mob_portal = room_center.with_z(room_base - (room_size / 4));
            let mob_portal_target = (room_center + 10).with_z(room_base - (room_size * 2));
            let mini_boss_portal = room_center.with_z(room_base - (room_size * 2));
            let exit_position = (center - 10).with_z(base - (6 * room_size));
            let boss_position = (center - 10).with_z(base - (7 * room_size));
            let boss_portal = center.with_z(base - (7 * room_size));
            let mini_boss_portal_target = if portal_to_boss {
                boss_position.as_::<f32>()
            } else {
                exit_position.as_::<f32>()
            };
            if mob_room {
                painter.spawn(EntityInfo::at(mob_portal.as_::<f32>()).into_special(
                    SpecialEntity::Teleporter(PortalData {
                        target: mob_portal_target.as_::<f32>(),
                        requires_no_aggro: true,
                        buildup_time: Secs(5.),
                    }),
                ));
                painter.spawn(EntityInfo::at(mini_boss_portal.as_::<f32>()).into_special(
                    SpecialEntity::Teleporter(PortalData {
                        target: mini_boss_portal_target,
                        requires_no_aggro: true,
                        buildup_time: Secs(5.),
                    }),
                ));
            } else if boss_room {
                painter.spawn(EntityInfo::at(boss_portal.as_::<f32>()).into_special(
                    SpecialEntity::Teleporter(PortalData {
                        target: exit_position.as_::<f32>(),
                        requires_no_aggro: true,
                        buildup_time: Secs(5.),
                    }),
                ));
            }

            if !mob_room {
                if boss_room {
                    let npc_pos = room_center.with_z(room_base - room_size);

                    painter.spawn(EntityInfo::at(npc_pos.as_()).with_asset_expect(
                        "common.entity.dungeon.cultist.mindflayer",
                        &mut thread_rng,
                        None,
                    ));
                } else {
                    let npc_pos = (room_center - 2).with_z(room_base - room_size);
                    painter.spawn(EntityInfo::at(npc_pos.as_()).with_asset_expect(
                        "common.entity.dungeon.cultist.warlock",
                        &mut thread_rng,
                        None,
                    ));

                    painter.spawn(EntityInfo::at(npc_pos.as_()).with_asset_expect(
                        "common.entity.dungeon.cultist.warlord",
                        &mut thread_rng,
                        None,
                    ));
                    painter.spawn(
                        EntityInfo::at(((room_center + 5).with_z(room_base - room_size)).as_())
                            .with_asset_expect(
                                "common.entity.dungeon.cultist.beastmaster",
                                &mut thread_rng,
                                None,
                            ),
                    );
                }
                // gold chains
                let chain_positions = place_circular(room_center, 15.0, 10);
                for pos in chain_positions {
                    painter
                        .aabb(Aabb {
                            min: pos.with_z(room_base - 12),
                            max: (pos + 1).with_z(room_base - 4),
                        })
                        .fill(gold_chain.clone());
                }
            }
            let down = if mob_room && decor_var < 3 {
                0
            } else if mob_room && decor_var > 2 {
                room_size
            } else {
                10
            };
            let magic_circle_bb = painter.cylinder(Aabb {
                min: (room_center - 15).with_z(room_base - 3 - down),
                max: (room_center + 16).with_z(room_base - 2 - down),
            });
            star_positions.push((magic_circle_bb, room_center));
        }
        // candles & chests & npcs
        for sprite_pos in sprite_positions {
            // keep center pit clear
            if sprite_pos.xy().distance_squared(center) > 40_i32.pow(2)
                || sprite_pos.z < (base - (6 * room_size))
            {
                match (RandomField::new(0).get(sprite_pos + 1)) % 16 {
                    0 => {
                        if sprite_pos.z > (base - (6 * room_size)) {
                            random_npcs.push(sprite_pos)
                        }
                    },
                    1 => {
                        // prisoners
                        painter
                            .aabb(Aabb {
                                min: (sprite_pos - 1).with_z(sprite_pos.z),
                                max: (sprite_pos + 2).with_z(sprite_pos.z + 3),
                            })
                            .fill(key_door.clone());
                        painter
                            .aabb(Aabb {
                                min: sprite_pos.with_z(sprite_pos.z + 3),
                                max: (sprite_pos + 1).with_z(sprite_pos.z + 4),
                            })
                            .fill(key_hole.clone());
                        painter
                            .aabb(Aabb {
                                min: (sprite_pos).with_z(sprite_pos.z),
                                max: (sprite_pos + 1).with_z(sprite_pos.z + 2),
                            })
                            .clear();
                        painter.spawn(EntityInfo::at(sprite_pos.as_()).with_asset_expect(
                            match (RandomField::new(0).get(sprite_pos)) % 10 {
                                0 => "common.entity.village.farmer",
                                1 => "common.entity.village.guard",
                                2 => "common.entity.village.hunter",
                                3 => "common.entity.village.skinner",
                                _ => "common.entity.village.villager",
                            },
                            &mut thread_rng,
                            None,
                        ));
                    },
                    _ => {
                        painter.sprite(
                            sprite_pos,
                            match (RandomField::new(0).get(sprite_pos)) % 20 {
                                0 => SpriteKind::DungeonChest5,
                                _ => SpriteKind::Candle,
                            },
                        );
                    },
                }
            }
        }
        // random_npcs around upper entrance and bottom portal
        for s in 0..=1 {
            let radius = 62.0 - (s * 50) as f32;
            let npcs = place_circular(center, radius, 8 - (s * 4));
            for npc_pos in npcs {
                random_npcs.push(npc_pos.with_z(base + 8 - ((6 * room_size) * s) - (s * 8)));
            }
        }
        for pos in random_npcs {
            let entities = [
                "common.entity.dungeon.cultist.cultist",
                "common.entity.dungeon.cultist.turret",
                "common.entity.dungeon.cultist.husk",
                "common.entity.dungeon.cultist.husk_brute",
                "common.entity.dungeon.cultist.hound",
            ];
            let npc = entities[(RandomField::new(0).get(pos) % entities.len() as u32) as usize];
            painter.spawn(EntityInfo::at(pos.as_()).with_asset_expect(npc, &mut thread_rng, None));
        }

        // outside portal
        let top_position = (center - 20).with_z(base + 125);
        let bottom_position = center.with_z(base - (6 * room_size));
        let top_pos = Vec3::new(
            top_position.x as f32,
            top_position.y as f32,
            top_position.z as f32,
        );
        let bottom_pos = Vec3::new(
            bottom_position.x as f32,
            bottom_position.y as f32,
            bottom_position.z as f32,
        );
        painter.spawn(
            EntityInfo::at(bottom_pos).into_special(SpecialEntity::Teleporter(PortalData {
                target: top_pos,
                requires_no_aggro: true,
                buildup_time: Secs(5.),
            })),
        );
        let stone_purple = Block::new(BlockKind::GlowingRock, Rgb::new(96, 0, 128));
        let magic_circle_bb = painter.cylinder(Aabb {
            min: (center - 15).with_z(base - (floors * (2 * room_size)) - 1),
            max: (center + 16).with_z(base - (floors * (2 * room_size))),
        });
        let magic_circle_bb_boss = painter.cylinder(Aabb {
            min: (center - 15).with_z(base - (7 * room_size) - 2),
            max: (center + 16).with_z(base - (7 * room_size) - 1),
        });
        star_positions.push((magic_circle_bb, center));
        star_positions.push((magic_circle_bb_boss, center));
        for (magic_circle_bb, position) in star_positions {
            let magic_circle = painter.prim(Primitive::sampling(
                magic_circle_bb,
                inscribed_polystar(position, 15.0, 7),
            ));
            painter.fill(magic_circle, Fill::Block(stone_purple));
        }
        // base floor
        painter
            .cylinder(Aabb {
                min: (center - room_size - 15).with_z(base - (floors * (2 * room_size)) - 3),
                max: (center + room_size + 15).with_z(base - (floors * (2 * room_size)) - 2),
            })
            .fill(rock.clone());
    }
}
