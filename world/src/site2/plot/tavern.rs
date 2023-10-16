use std::{cmp::Ordering, mem::swap, ops::RangeInclusive};

use common::{
    comp::Content,
    lottery::Lottery,
    store::{Id, Store},
    terrain::{Block, BlockKind, SpriteCfg, SpriteKind},
};
use enum_map::EnumMap;
use enumset::EnumSet;
use hashbrown::HashSet;
use rand::{seq::IteratorRandom, Rng};
use strum::{EnumIter, IntoEnumIterator};
use vek::*;

use crate::{
    site::namegen,
    site2::{gen::PrimitiveTransform, Dir, Fill, Site, Structure},
    util::RandomField,
    IndexRef, Land,
};

type Neighbor = Option<Id<Room>>;

struct Wall {
    start: Vec2<i32>,
    end: Vec2<i32>,
    base_alt: i32,
    top_alt: i32,
    from: Neighbor,
    to: Neighbor,
    to_dir: Dir,
    door: Option<i32>,
}

#[derive(Clone, Copy, EnumIter, enum_map::Enum)]
enum RoomKind {
    Garden,
    StageRoom,
    BarRoom,
    EntranceRoom,
}

impl RoomKind {
    /// Returns the (side length size range, area size range)
    fn size_range(&self) -> (RangeInclusive<i32>, RangeInclusive<i32>) {
        match self {
            RoomKind::Garden => (4..=20, 25..=250),
            RoomKind::StageRoom => (10..=20, 130..=400),
            RoomKind::BarRoom => (7..=14, 56..=196),
            RoomKind::EntranceRoom => (3..=10, 9..=50),
        }
    }
}

#[derive(Clone, Copy)]
enum Detail {
    Bar {
        aabr: Aabr<i32>,
    },
    Table {
        pos: Vec2<i32>,
        chairs: EnumSet<Dir>,
    },
    Stage {
        aabr: Aabr<i32>,
    },
}

struct Room {
    /// Inclusive
    bounds: Aabb<i32>,
    kind: RoomKind,
    // stairs: Option<Id<Stairs>>,
    walls: EnumMap<Dir, Vec<Id<Wall>>>,
    // TODO: Remove this, used for debugging
    detail_areas: Vec<Aabr<i32>>,
    details: Vec<Detail>,
}

impl Room {
    fn new(bounds: Aabb<i32>, kind: RoomKind) -> Self {
        Self {
            bounds,
            kind,
            walls: Default::default(),
            detail_areas: Default::default(),
            details: Default::default(),
        }
    }
}

struct Stairs {
    end: Vec2<i32>,
    dir: Dir,
    in_room: Id<Room>,
    to_room: Id<Room>,
}

pub struct Tavern {
    name: String,
    rooms: Store<Room>,
    stairs: Store<Stairs>,
    walls: Store<Wall>,
    /// Tile position of the door tile
    pub door_tile: Vec2<i32>,
    pub(crate) door_wpos: Vec3<i32>,
    /// Axis aligned bounding region for the house
    bounds: Aabr<i32>,
}

impl Tavern {
    pub fn generate(
        land: &Land,
        index: IndexRef,
        rng: &mut impl Rng,
        site: &Site,
        door_tile: Vec2<i32>,
        door_dir: Dir,
        tile_aabr: Aabr<i32>,
    ) -> Self {
        let start = std::time::Instant::now();
        let mut rooms = Store::default();
        let stairs = Store::default();
        let mut walls = Store::default();
        let mut room_counts = EnumMap::<RoomKind, u32>::default();

        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };

        let ibounds = Aabr {
            min: bounds.min + 1,
            max: bounds.max - 2,
        };

        let door_tile_center = site.tile_center_wpos(door_tile);
        let door_wpos = door_dir.select_aabr_with(ibounds, door_tile_center);

        let door_alt = land
            .column_sample(door_wpos, index)
            .map_or_else(|| land.get_alt_approx(door_wpos), |sample| sample.alt);
        let door_wpos = door_wpos.with_z(door_alt.ceil() as i32);

        /// Place room in bounds.
        fn place_room_in(
            room: RoomKind,
            max_bounds: Aabr<i32>,
            in_dir: Dir,
            in_pos: Vec2<i32>,
            rng: &mut impl Rng,
        ) -> Option<Aabr<i32>> {
            let (size_range, area_range) = room.size_range();

            let mut gen_range = |min, max, snap_max| {
                let res = rng.gen_range(min..=max);
                if snap_max <= max && snap_max - res <= 2 {
                    snap_max
                } else {
                    res
                }
            };
            let min = *size_range.start();
            let snap_max = in_dir.select(max_bounds.size());
            let max = snap_max.min(*size_range.end());
            if max < min {
                return None;
            }
            let size_x = gen_range(min, max, snap_max);

            let min = ((*area_range.start() + size_x - 1) / size_x).max(*size_range.start());
            let snap_max = in_dir.orthogonal().select(max_bounds.size());
            let max = snap_max
                .min(*size_range.end())
                .min(*area_range.end() / size_x);

            if max < min {
                return None;
            }
            let size_y = gen_range(min, max, snap_max);

            // calculate a valid aabr
            let half_size_y = size_y / 2 + (size_y % 2) * rng.gen_range(0..=1);
            let min = in_pos + in_dir.to_vec2() + in_dir.rotated_cw().to_vec2() * half_size_y;
            let min = max_bounds.projected_point(min);
            let max = min + in_dir.to_vec2() * size_x + in_dir.rotated_ccw().to_vec2() * size_y;
            let max = max_bounds.projected_point(max);
            let min = max - in_dir.to_vec2() * size_x + in_dir.rotated_cw().to_vec2() * size_y;

            let bounds = Aabr { min, max }.made_valid();
            Some(bounds)
        }
        struct RoomMeta {
            id: Id<Room>,
            walls: EnumSet<Dir>,
        }

        let mut room_metas = Vec::new();

        {
            let entrance_rooms =
                Lottery::from(vec![(1.0, RoomKind::Garden), (2.0, RoomKind::EntranceRoom)]);

            let entrance_room = *entrance_rooms.choose_seeded(rng.gen());
            let entrance_room_hgt = rng.gen_range(3..=4);
            let entrance_room_aabr =
                place_room_in(entrance_room, ibounds, -door_dir, door_wpos.xy(), rng)
                    .expect("Not enough room in plot for a tavern");
            let entrance_room_aabb = Aabb {
                min: entrance_room_aabr.min.with_z(door_wpos.z),
                max: entrance_room_aabr
                    .max
                    .with_z(door_wpos.z + entrance_room_hgt),
            }
            .made_valid();

            let entrance_id = rooms.insert(Room::new(entrance_room_aabb, entrance_room));

            let start = door_dir.select_aabr_with(
                entrance_room_aabr,
                Vec2::broadcast(door_dir.rotated_cw().select_aabr(entrance_room_aabr)),
            ) + door_dir.rotated_cw().to_vec2()
                + door_dir.to_vec2();
            let wall_id = walls.insert(Wall {
                start,
                end: door_dir.select_aabr_with(
                    entrance_room_aabr,
                    Vec2::broadcast(door_dir.rotated_ccw().select_aabr(entrance_room_aabr)),
                ) + door_dir.rotated_ccw().to_vec2()
                    + door_dir.to_vec2(),
                base_alt: entrance_room_aabb.min.z,
                top_alt: entrance_room_aabb.max.z,
                from: None,
                to: Some(entrance_id),
                to_dir: -door_dir,
                door: Some(door_dir.rotated_cw().select(door_wpos.xy() - start).abs()),
            });
            rooms[entrance_id].walls[door_dir].push(wall_id);

            room_metas.push(RoomMeta {
                id: entrance_id,
                walls: Dir::iter()
                .filter(|d| *d != door_dir)
                // .map(|d| {
                //     let a = d.rotated_cw().select_aabr(entrance_room_aabr);
                //     let b = d.rotated_ccw().select_aabr(entrance_room_aabr);
                //     (d, a.min(b)..=a.max(b))
                // })
                .collect(),
            });

            room_counts[entrance_room] += 1;
        }

        let to_aabr = |aabb: Aabb<i32>| Aabr {
            min: aabb.min.xy(),
            max: aabb.max.xy(),
        };
        // Extend a valid aabr
        let extend_aabr = |aabr: Aabr<i32>, amount: i32| Aabr {
            min: aabr.min - amount,
            max: aabr.max + amount,
        };
        'room_gen: while room_metas.len() > 0 {
            let mut room_meta = room_metas.swap_remove(rng.gen_range(0..room_metas.len()));
            if room_meta.walls.is_empty() {
                continue 'room_gen;
            }

            let Some(in_dir) = room_meta.walls.into_iter().choose(rng) else {
                continue 'room_gen;
            };
            room_meta.walls.remove(in_dir);

            let right = in_dir.orthogonal();
            let left = -right;

            let from_id = room_meta.id;
            let from_room = &rooms[from_id];

            if !room_meta.walls.is_empty() {
                room_metas.push(room_meta);
            }

            let from_bounds = to_aabr(from_room.bounds);

            // The maximum bounds, limited by the plot bounds and other rooms.
            let mut max_bounds = Aabr {
                min: in_dir.select_aabr_with(from_bounds, ibounds.min) + in_dir.to_vec2() * 2,
                max: in_dir.select_aabr_with(ibounds, ibounds.max),
            }
            .made_valid();

            // Take other rooms into account when calculating `max_bounds`. We don't care
            // about this room if it's the originating room or at another
            // height.
            for (_, room) in rooms.iter().filter(|(room_id, room)| {
                *room_id != from_id
                    && room.bounds.min.z <= from_room.bounds.max.z
                    && room.bounds.max.z >= from_room.bounds.min.z
            }) {
                let bounds = to_aabr(room.bounds);
                let bounds = extend_aabr(bounds, 2);
                let intersection = bounds.intersection(max_bounds);
                if intersection.is_valid() {
                    let Some(bounds) = Dir::iter()
                        .filter(|dir| {
                            *dir != in_dir
                                && dir.select_aabr(intersection) * dir.signum()
                                    < dir.select_aabr(max_bounds) * dir.signum()
                        })
                        .map(|min_dir| {
                            Aabr {
                                min: min_dir.select_aabr_with(
                                    max_bounds,
                                    Vec2::broadcast(min_dir.rotated_ccw().select_aabr(max_bounds)),
                                ),
                                max: min_dir.select_aabr_with(
                                    intersection,
                                    Vec2::broadcast(min_dir.rotated_cw().select_aabr(max_bounds)),
                                ),
                            }
                            .made_valid()
                        })
                        .filter(|bounds| {
                            left.select_aabr(*bounds) < right.select_aabr(from_bounds)
                                && right.select_aabr(*bounds) > left.select_aabr(from_bounds)
                        })
                        .max_by_key(|bounds| bounds.size().product())
                    else {
                        continue 'room_gen;
                    };

                    max_bounds = bounds;
                }
            }

            // the smallest side on the maximum bounds
            let max_min_size = max_bounds.size().reduce_min();
            // max bounds area
            let max_area = max_bounds.size().product();

            let room_lottery = RoomKind::iter()
                // Filter out rooms that won't fit here.
                .filter(|room_kind| {
                    let (size_range, area_range) = room_kind.size_range();
                    *size_range.start() <= max_min_size && *area_range.start() <= max_area
                })
                // Calculate chance for each room.
                .map(|room_kind| {
                    (
                        match room_kind {
                            RoomKind::Garden => {
                                1.0 / (1.0 + room_counts[RoomKind::Garden] as f32 / 2.0)
                            },
                            RoomKind::StageRoom => {
                                2.0 / (1.0 + room_counts[RoomKind::StageRoom] as f32).powi(2)
                            },
                            RoomKind::BarRoom => {
                                2.0 / (1.0 + room_counts[RoomKind::BarRoom] as f32).powi(2)
                            },
                            RoomKind::EntranceRoom => {
                                0.1 / (1.0 + room_counts[RoomKind::EntranceRoom] as f32)
                            },
                        },
                        room_kind,
                    )
                })
                .collect::<Vec<_>>();
            // We have no rooms to pick from.
            if room_lottery.is_empty() {
                continue 'room_gen;
            }

            // Pick a room.
            let room_lottery = Lottery::from(room_lottery);
            let room = *room_lottery.choose_seeded(rng.gen());

            // Select a door position
            let mut min = left
                .select_aabr(from_bounds)
                .max(left.select_aabr(max_bounds));
            let mut max = right
                .select_aabr(from_bounds)
                .min(right.select_aabr(max_bounds));
            if max < min {
                swap(&mut min, &mut max);
            }
            if min + 2 > max {
                continue 'room_gen;
            }
            let in_pos = rng.gen_range(min + 1..=max - 1);
            let in_pos =
                in_dir.select_aabr_with(from_bounds, Vec2::broadcast(in_pos)) + in_dir.to_vec2();

            let Some(bounds) = place_room_in(room, max_bounds, in_dir, in_pos, rng) else {
                continue 'room_gen;
            };

            let room_hgt = rng.gen_range(3..=5);

            let bounds3 = Aabb {
                min: bounds.min.with_z(from_room.bounds.min.z),
                max: bounds.max.with_z(from_room.bounds.min.z + room_hgt),
            };
            let id = rooms.insert(Room::new(bounds3, room));

            let start = in_dir.select_aabr_with(
                from_bounds,
                Vec2::broadcast(left.select_aabr(from_bounds).max(left.select_aabr(bounds))),
            ) + in_dir.to_vec2()
                + left.to_vec2();

            let end = in_dir.select_aabr_with(
                from_bounds,
                Vec2::broadcast(
                    right
                        .select_aabr(from_bounds)
                        .min(right.select_aabr(bounds)),
                ),
            ) + in_dir.to_vec2()
                + right.to_vec2();

            let wall_id = walls.insert(Wall {
                start,
                end,
                base_alt: bounds3.min.z,
                top_alt: bounds3.max.z,
                from: Some(from_id),
                to: Some(id),
                to_dir: in_dir,
                door: Some(right.select(in_pos - start)),
            });

            rooms[id].walls[-in_dir].push(wall_id);
            rooms[from_id].walls[in_dir].push(wall_id);

            room_metas.push(RoomMeta {
                id,
                walls: Dir::iter().filter(|d| *d != -in_dir).collect(),
            });
            room_counts[room] += 1;
        }

        // Place walls where needed.
        for from_id in rooms.ids() {
            let room_bounds = to_aabr(rooms[from_id].bounds);
            let mut skip = HashSet::new();
            skip.insert(from_id);
            let mut wall_ranges = EnumMap::<Dir, Vec<_>>::default();
            for dir in Dir::iter() {
                let orth = dir.orthogonal();
                let range = (orth.select(room_bounds.min), orth.select(room_bounds.max));
                wall_ranges[dir].push(range);
            }
            let mut split_range = |dir: Dir, min: i32, max: i32| {
                debug_assert!(min <= max);
                let Ok(i) = wall_ranges[dir].binary_search_by(|(r_min, r_max)| {
                    match (min.cmp(r_min), min.cmp(r_max)) {
                        (Ordering::Less, _) => Ordering::Greater,
                        (Ordering::Greater | Ordering::Equal, Ordering::Less | Ordering::Equal) => {
                            Ordering::Equal
                        },
                        (_, Ordering::Greater) => Ordering::Less,
                    }
                }) else {
                    // TODO: Don't panic here.
                    dbg!((min, max));
                    dbg!(&wall_ranges[dir]);
                    panic!("Couldn't find range");
                };

                let range = &mut wall_ranges[dir][i];
                debug_assert!(range.0 <= min);
                debug_assert!(range.1 >= max);

                match (range.0 == min, range.1 == max) {
                    (true, true) => {
                        wall_ranges[dir].remove(i);
                    },
                    (true, false) => *range = (max + 1, range.1),
                    (false, true) => *range = (range.0, min - 1),
                    (false, false) => {
                        let tmp = range.1;
                        *range = (range.0, min - 1);
                        debug_assert!(range.0 <= range.1);
                        let m = (max + 1, tmp);
                        debug_assert!(m.0 <= m.1, "{m:?}");
                        wall_ranges[dir].insert(i + 1, m);
                    },
                }
            };
            for dir in Dir::iter() {
                let connected_walls = &mut rooms[from_id].walls[dir];
                skip.extend(
                    connected_walls
                        .iter()
                        .flat_map(|wall| walls[*wall].from.into_iter().chain(walls[*wall].to)),
                );
                let orth = dir.orthogonal();
                // Divide wall ranges by existing walls.
                for wall in connected_walls.iter() {
                    let wall = &walls[*wall];
                    let mut min = orth.select(wall.start);
                    let mut max = orth.select(wall.end);
                    if min > max {
                        swap(&mut min, &mut max);
                    }
                    min += 1;
                    max -= 1;
                    split_range(dir, min, max);
                }
            }

            // Divide wall ranges by neighbouring rooms
            for to_id in rooms.ids().filter(|id| !skip.contains(id)) {
                let a_min_z = rooms[from_id].bounds.min.z;
                let a_max_z = rooms[from_id].bounds.max.z;
                let b_min_z = rooms[to_id].bounds.min.z;
                let b_max_z = rooms[to_id].bounds.max.z;
                if a_min_z > b_max_z || a_max_z < b_min_z {
                    // We are not at the same altitude.
                    continue;
                }
                let min_z = a_min_z.min(b_min_z);
                let max_z = a_max_z.max(b_max_z);
                let n_room_bounds = to_aabr(rooms[to_id].bounds);

                let p1 = n_room_bounds.projected_point(room_bounds.center());
                let p0 = room_bounds.projected_point(p1);

                let to_dir = Dir::from_vec2(p1 - p0);

                let intersection = to_dir
                    .extend_aabr(room_bounds, 1)
                    .intersection(to_dir.opposite().extend_aabr(n_room_bounds, 1));

                if intersection.is_valid() {
                    let start = intersection.min;
                    let end = intersection.max;

                    let orth = to_dir.orthogonal();

                    let min = orth.select(start);
                    let max = orth.select(end);
                    split_range(to_dir, min, max);
                    let door = if max - min >= 3 && rng.gen_bool(0.8) {
                        Some(rng.gen_range(1..=max - min))
                    } else {
                        None
                    };

                    let id = walls.insert(Wall {
                        start: start - orth.to_vec2(),
                        end: end + orth.to_vec2(),
                        base_alt: min_z,
                        top_alt: max_z,
                        from: Some(from_id),
                        to: Some(to_id),
                        to_dir,
                        door,
                    });

                    rooms[from_id].walls[to_dir].push(id);
                    rooms[to_id].walls[-to_dir].push(id);
                }
            }
            // Place remaining walls.
            for (dir, ranges) in wall_ranges {
                for (min, max) in ranges {
                    let start =
                        dir.select_aabr_with(room_bounds, Vec2::broadcast(min - 1)) + dir.to_vec2();
                    let end =
                        dir.select_aabr_with(room_bounds, Vec2::broadcast(max + 1)) + dir.to_vec2();

                    let wall_id = walls.insert(Wall {
                        start,
                        end,
                        base_alt: rooms[from_id].bounds.min.z,
                        top_alt: rooms[from_id].bounds.max.z,
                        from: Some(from_id),
                        to: None,
                        to_dir: dir,
                        door: None,
                    });

                    rooms[from_id].walls[dir].push(wall_id);
                }
            }
        }

        // Compute detail areas
        for room in rooms.values_mut() {
            let bounds = to_aabr(room.bounds);
            let c = bounds.center();
            let mut b = bounds.split_at_x(c.x);
            b[0].max.x -= 1;
            room.detail_areas.extend(b.into_iter().flat_map(|b| {
                let mut b = b.split_at_y(c.y);
                b[0].max.y -= 1;
                b
            }));
            for (dir, dir_walls) in room.walls.iter() {
                for door_pos in dir_walls.iter().filter_map(|wall_id| {
                    let wall = &walls[*wall_id];

                    wall.door.map(|door| {
                        let wall_dir = Dir::from_vec2(wall.end - wall.start);
                        wall.start + wall_dir.to_vec2() * door
                    })
                }) {
                    let orth = dir.orthogonal();
                    for i in 0..room.detail_areas.len() {
                        let bc = room.detail_areas[i].center();
                        // Check if we are on the doors side of the center of the room.
                        if dir.select(bc - c) * dir.signum() >= 0 {
                            if let Some([a, b]) =
                                orth.try_split_aabr(room.detail_areas[i], orth.select(door_pos))
                            {
                                room.detail_areas[i] = orth.extend_aabr(a, -1);
                                room.detail_areas.push(orth.opposite().extend_aabr(b, -1));
                            }
                        }
                    }
                    room.detail_areas.retain(|area| area.is_valid());
                }
            }
        }

        // Place details in detail areas.

        for room in rooms.values_mut() {
            let room_aabr = to_aabr(room.bounds);
            let table = |pos: Vec2<i32>, aabr: Aabr<i32>| Detail::Table {
                pos,
                chairs: Dir::iter()
                    .filter(|dir| aabr.contains_point(pos + dir.to_vec2()))
                    .collect(),
            };
            match room.kind {
                RoomKind::Garden => room.detail_areas.retain(|&aabr| {
                    if aabr.size().reduce_max() > 1 && rng.gen_bool(0.7) {
                        room.details.push(table(aabr.center(), aabr));
                        false
                    } else {
                        true
                    }
                }),
                RoomKind::StageRoom => {
                    let mut best = None;
                    let mut best_score = 0;
                    for (i, aabr) in room.detail_areas.iter().enumerate() {
                        let edges = Dir::iter()
                            .filter(|dir| dir.select_aabr(*aabr) == dir.select_aabr(room_aabr))
                            .count() as i32;
                        let test_score = edges * aabr.size().product();
                        if best_score < test_score {
                            best_score = test_score;
                            best = Some(i);
                        }
                    }
                    if let Some(aabr) = best.map(|i| room.detail_areas.swap_remove(i)) {
                        room.details.push(Detail::Stage { aabr })
                    }
                    room.detail_areas.retain(|&aabr| {
                        if aabr.size().reduce_max() > 1 && rng.gen_bool(0.8) {
                            room.details.push(table(aabr.center(), aabr));
                            false
                        } else {
                            true
                        }
                    });
                },
                RoomKind::BarRoom => {
                    let mut best = None;
                    let mut best_score = 0;
                    for (i, aabr) in room.detail_areas.iter().enumerate() {
                        let test_score = Dir::iter()
                            .any(|dir| dir.select_aabr(*aabr) == dir.select_aabr(room_aabr))
                            as i32
                            * aabr.size().product();
                        if best_score < test_score {
                            best_score = test_score;
                            best = Some(i);
                        }
                    }
                    if let Some(aabr) = best.map(|i| room.detail_areas.swap_remove(i)) {
                        room.details.push(Detail::Bar { aabr })
                    }
                    room.detail_areas.retain(|&aabr| {
                        if aabr.size().reduce_max() > 1 && rng.gen_bool(0.1) {
                            room.details.push(table(aabr.center(), aabr));
                            false
                        } else {
                            true
                        }
                    });
                },
                RoomKind::EntranceRoom => {},
            }
        }

        let name = namegen::NameGen::location(rng).generate_tavern();

        println!("GENERATION TIME: {}μs", start.elapsed().as_micros());
        Self {
            name,
            rooms,
            stairs,
            walls,
            door_tile,
            door_wpos,
            bounds,
        }
    }
}

fn aabb(mut aabb: Aabb<i32>) -> Aabb<i32> {
    aabb.make_valid();
    aabb.max += 1;
    aabb
}

impl Structure for Tavern {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_tavern\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_tavern")]
    fn render_inner(&self, _site: &Site, _land: &Land, painter: &crate::site2::Painter) {
        let bounds = Aabr {
            min: self.bounds.min,
            max: self.bounds.max - 1,
        };

        let stone = Fill::Brick(BlockKind::Rock, Rgb::new(70, 70, 70), 10);
        let wood = Fill::Block(Block::new(BlockKind::Wood, Rgb::new(106, 73, 64)));
        let dark_wood = Fill::Block(Block::new(BlockKind::Wood, Rgb::new(80, 53, 48)));

        let field = RandomField::new(740384);
        painter
            .aabb(aabb(Aabb {
                min: bounds.min.with_z(self.door_wpos.z - 10),
                max: bounds.max.with_z(self.door_wpos.z - 1),
            }))
            .fill(stone.clone());

        for (_, room) in self.rooms.iter() {
            painter.aabb(aabb(room.bounds)).clear();
            painter
                .aabb(aabb(Aabb {
                    min: room.bounds.min.with_z(room.bounds.min.z - 1),
                    max: room.bounds.max.with_z(room.bounds.min.z - 1),
                }))
                .fill(wood.clone());

            let room_aabr = Aabr {
                min: room.bounds.min.xy(),
                max: room.bounds.max.xy(),
            };
            for (i, aabr) in room.detail_areas.iter().enumerate() {
                let color = fxhash::hash32(&i).to_le_bytes();

                painter
                    .aabb(aabb(Aabb {
                        min: aabr.min.with_z(room.bounds.min.z - 1),
                        max: aabr.max.with_z(room.bounds.min.z - 1),
                    }))
                    .fill(Fill::Block(Block::new(
                        BlockKind::Rock,
                        Rgb::new(color[0], color[1], color[3]),
                    )));
            }
            match room.kind {
                RoomKind::Garden => {
                    let dir = Dir::from_vec2(room_aabr.size().into());

                    painter
                        .aabb(aabb(Aabb {
                            min: dir
                                .select_aabr_with(room_aabr, room_aabr.min - 2)
                                .with_z(room.bounds.max.z + 1),
                            max: dir
                                .select_aabr_with(room_aabr, room_aabr.max + 2)
                                .with_z(room.bounds.max.z + 1),
                        }))
                        .repeat(
                            -dir.to_vec3() * 2,
                            (dir.select(room_aabr.size()) as u32 + 3) / 2,
                        )
                        .fill(dark_wood.clone())
                },
                RoomKind::StageRoom => {
                    for aabr in room.detail_areas.iter().copied() {
                        for dir in Dir::iter().filter(|dir| {
                            dir.select_aabr(aabr) == dir.select_aabr(room_aabr)
                                && dir.rotated_cw().select_aabr(aabr)
                                    == dir.rotated_cw().select_aabr(room_aabr)
                        }) {
                            let pos = dir.select_aabr_with(
                                aabr,
                                Vec2::broadcast(dir.rotated_cw().select_aabr(aabr)),
                            );
                            painter.sprite(pos.with_z(room.bounds.min.z), SpriteKind::StreetLamp);
                        }
                    }
                },
                RoomKind::BarRoom => {
                    for aabr in room.detail_areas.iter().copied() {
                        for dir in Dir::iter()
                            .filter(|dir| dir.select_aabr(aabr) == dir.select_aabr(room_aabr))
                        {
                            let pos = dir
                                .select_aabr_with(aabr, aabr.center())
                                .with_z(room.bounds.center().z);

                            painter.rotated_sprite(
                                pos,
                                SpriteKind::WallLampSmall,
                                dir.opposite().sprite_ori(),
                            );
                        }
                    }
                },
                RoomKind::EntranceRoom => {
                    for aabr in room.detail_areas.iter() {
                        let edges = Dir::iter()
                            .filter(|dir| dir.select_aabr(*aabr) == dir.select_aabr(room_aabr))
                            .count();
                        let hanger_pos = if edges == 2 {
                            let pos = aabr.center().with_z(room.bounds.min.z);
                            painter.sprite(pos, SpriteKind::CoatRack);
                            Some(pos)
                        } else {
                            None
                        };

                        for dir in Dir::iter()
                            .filter(|dir| dir.select_aabr(*aabr) == dir.select_aabr(room_aabr))
                        {
                            let pos = dir
                                .select_aabr_with(*aabr, aabr.center())
                                .with_z(room.bounds.center().z + 1);
                            if hanger_pos.map_or(false, |p| p.xy() != pos.xy()) {
                                painter.rotated_sprite(
                                    pos,
                                    SpriteKind::WallLampSmall,
                                    dir.opposite().sprite_ori(),
                                );
                            }
                        }
                    }
                },
            }
            for detail in room.details.iter() {
                match *detail {
                    Detail::Bar { aabr } => {
                        for dir in Dir::iter() {
                            let edge = dir.select_aabr(aabr);
                            let rot_dir = if field.chance(aabr.center().with_z(0), 0.5) {
                                dir.rotated_cw()
                            } else {
                                dir.rotated_ccw()
                            };
                            let rot_edge = rot_dir.select_aabr(aabr);
                            match (
                                edge == dir.select_aabr(room_aabr),
                                rot_edge == rot_dir.select_aabr(room_aabr),
                            ) {
                                (false, _) => {
                                    let (min, max) = (
                                        dir.select_aabr_with(
                                            aabr,
                                            Vec2::broadcast(rot_dir.select_aabr(aabr)),
                                        ),
                                        dir.select_aabr_with(
                                            aabr,
                                            Vec2::broadcast(rot_dir.opposite().select_aabr(aabr)),
                                        ),
                                    );
                                    painter
                                        .aabb(aabb(Aabb {
                                            min: (min - rot_dir.to_vec2())
                                                .with_z(room.bounds.min.z),
                                            max: max.with_z(room.bounds.min.z),
                                        }))
                                        .fill(dark_wood.clone());
                                    painter
                                        .aabb(aabb(Aabb {
                                            min: min.with_z(room.bounds.min.z + 3),
                                            max: max.with_z(room.bounds.max.z),
                                        }))
                                        .fill(dark_wood.clone());
                                },
                                (true, true) => {
                                    painter.sprite(
                                        dir.vec2(edge, rot_edge).with_z(room.bounds.min.z),
                                        SpriteKind::CookingPot,
                                    );
                                },
                                (true, false) => {},
                            }
                        }
                    },
                    Detail::Stage { aabr } => {
                        painter
                            .aabb(aabb(Aabb {
                                min: aabr.min.with_z(room.bounds.min.z),
                                max: aabr.max.with_z(room.bounds.min.z),
                            }))
                            .fill(stone.clone());
                        painter
                            .aabb(aabb(Aabb {
                                min: (aabr.min + 1).with_z(room.bounds.min.z),
                                max: (aabr.max - 1).with_z(room.bounds.min.z),
                            }))
                            .fill(wood.clone());
                        for dir in Dir::iter().filter(|dir| {
                            dir.select_aabr(aabr) != dir.select_aabr(room_aabr)
                                && dir.rotated_cw().select_aabr(aabr)
                                    != dir.rotated_cw().select_aabr(room_aabr)
                        }) {
                            let pos = dir.select_aabr_with(
                                aabr,
                                Vec2::broadcast(dir.rotated_cw().select_aabr(aabr)),
                            );
                            painter
                                .column(pos, room.bounds.min.z..=room.bounds.max.z)
                                .fill(dark_wood.clone());

                            for dir in Dir::iter() {
                                painter.rotated_sprite(
                                    pos.with_z(room.bounds.center().z + 1) + dir.to_vec2(),
                                    SpriteKind::WallSconce,
                                    dir.sprite_ori(),
                                );
                            }
                        }
                    },
                    Detail::Table { pos, chairs } => {
                        let pos = pos.with_z(room.bounds.min.z);
                        painter.sprite(pos, SpriteKind::TableDining);
                        for dir in chairs.into_iter() {
                            painter.rotated_sprite(
                                pos + dir.to_vec2(),
                                SpriteKind::ChairSingle,
                                dir.opposite().sprite_ori(),
                            );
                        }
                    },
                }
            }
        }

        for (_, wall) in self.walls.iter() {
            let get_kind = |room| self.rooms.get(room).kind;
            let wall_aabb = Aabb {
                min: wall.start.with_z(wall.base_alt),
                max: wall.end.with_z(wall.top_alt),
            };
            let wall_dir = Dir::from_vec2(wall.end - wall.start);
            match (wall.from.map(get_kind), wall.to.map(get_kind)) {
                (Some(RoomKind::Garden), Some(RoomKind::Garden) | None)
                | (None, Some(RoomKind::Garden)) => {
                    let hgt = wall_aabb.min.z..=wall_aabb.max.z;
                    painter
                        .column(wall_aabb.min.xy(), hgt.clone())
                        .fill(dark_wood.clone());
                    painter
                        .column(wall_aabb.max.xy(), hgt)
                        .fill(dark_wood.clone());
                    let z = (wall.base_alt + wall.top_alt) / 2;

                    painter.rotated_sprite(
                        wall_aabb.min.with_z(z) + wall_dir.to_vec2(),
                        SpriteKind::WallSconce,
                        wall_dir.sprite_ori(),
                    );
                    painter.rotated_sprite(
                        wall_aabb.max.with_z(z) - wall_dir.to_vec2(),
                        SpriteKind::WallSconce,
                        wall_dir.opposite().sprite_ori(),
                    );
                    painter
                        .aabb(aabb(Aabb {
                            min: wall_aabb.min,
                            max: wall_aabb.max.with_z(wall_aabb.min.z),
                        }))
                        .fill(dark_wood.clone());
                    painter
                        .aabb(aabb(Aabb {
                            min: wall_aabb.min.with_z(wall_aabb.max.z),
                            max: wall_aabb.max,
                        }))
                        .fill(dark_wood.clone());
                },
                (None, None) => {},
                _ => {
                    painter.aabb(aabb(wall_aabb)).fill(wood.clone());
                    painter
                        .column(wall.start, wall.base_alt..=wall.top_alt)
                        .fill(dark_wood.clone());
                    painter
                        .column(wall.end, wall.base_alt..=wall.top_alt)
                        .fill(dark_wood.clone());
                },
            }
            if let Some(door) = wall.door {
                let door_pos = wall.start + wall_dir.to_vec2() * door;
                let min = match wall.from {
                    None => door_pos - wall.to_dir.to_vec2(),
                    Some(_) => door_pos,
                };
                let max = match wall.to {
                    None => door_pos + wall.to_dir.to_vec2(),
                    Some(_) => door_pos,
                };
                painter
                    .aabb(aabb(Aabb {
                        min: min.with_z(wall.base_alt),
                        max: max.with_z(wall.base_alt + 2),
                    }))
                    .clear();
            }
            if let (Some(room), to @ None) | (None, to @ Some(room)) =
                (wall.from.map(get_kind), wall.to.map(get_kind))
            {
                let dir = if to.is_none() {
                    -wall.to_dir
                } else {
                    wall.to_dir
                };
                let width = dir.orthogonal().select(wall.end - wall.start).abs();
                let wall_center = (wall.start + wall.end) / 2;
                let d = wall_dir.select(wall_center - wall.start) * wall_dir.signum();
                match room {
                    RoomKind::Garden => {
                        if wall.door.map_or(true, |door| (door - d).abs() >= 2) {
                            painter.rotated_sprite(
                                wall_center.with_z(wall.base_alt + 1),
                                SpriteKind::Planter,
                                dir.sprite_ori(),
                            );
                        }
                    },
                    _ => {
                        if width >= 5 && wall.door.map_or(true, |door| (door - d).abs() > 3) {
                            painter
                                .aabb(aabb(Aabb {
                                    min: (wall_center + dir.rotated_ccw().to_vec2())
                                        .with_z(wall.base_alt + 1),
                                    max: (wall_center + dir.rotated_cw().to_vec2())
                                        .with_z(wall.base_alt + 2),
                                }))
                                .fill(Fill::RotatedSprite(SpriteKind::Window1, dir.sprite_ori()));
                        }
                    },
                }
                if let Some(door) = wall.door {
                    let door_pos = wall.start + wall_dir.to_vec2() * door;
                    let diff = door_pos - wall_aabb.center().xy();
                    let orth = if diff == Vec2::zero() {
                        wall_dir
                    } else {
                        Dir::from_vec2(diff)
                    };
                    let pos = door_pos - dir.to_vec2() + orth.to_vec2();
                    let (sprite, z) = match room {
                        RoomKind::Garden => (SpriteKind::Sign, 0),
                        _ => (SpriteKind::HangingSign, 2),
                    };
                    painter.rotated_sprite_with_cfg(
                        pos.with_z(wall.base_alt + z),
                        sprite,
                        dir.opposite().sprite_ori(),
                        SpriteCfg {
                            unlock: None,
                            content: Some(Content::Plain(self.name.clone())),
                        },
                    );
                }
            }
        }

        for (_, stairs) in self.stairs.iter() {
            let down_room = &self.rooms[stairs.in_room];
            let up_room = &self.rooms[stairs.to_room];

            let down = -stairs.dir;
            let right = stairs.dir.rotated_cw();

            let aabr = Aabr {
                min: stairs.end - right.to_vec2()
                    + down.to_vec2() * (up_room.bounds.min.z - 1 - down_room.bounds.min.z),
                max: stairs.end + right.to_vec2(),
            };

            painter
                .aabb(aabb(Aabb {
                    min: aabr.min.with_z(up_room.bounds.min.z - 1),
                    max: aabr.max.with_z(up_room.bounds.min.z - 1),
                }))
                .clear();

            painter
                .ramp(
                    aabb(Aabb {
                        min: aabr.min.with_z(down_room.bounds.min.z),
                        max: aabr.max.with_z(up_room.bounds.min.z - 1),
                    }),
                    stairs.dir,
                )
                .fill(wood.clone());
        }
    }
}
