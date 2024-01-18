use std::{mem::swap, ops::RangeInclusive};

use common::{
    comp::Content,
    lottery::Lottery,
    store::{Id, Store},
    terrain::{BlockKind, SpriteCfg, SpriteKind},
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

pub struct Wall {
    start: Vec2<i32>,
    end: Vec2<i32>,
    base_alt: i32,
    top_alt: i32,
    from: Neighbor,
    to: Neighbor,
    to_dir: Dir,
    door: Option<(i32, i32)>,
}

impl Wall {
    pub fn door_pos(&self) -> Option<Vec3<f32>> {
        let wall_dir = Dir::from_vec2(self.end - self.start);

        self.door.map(|(door_min, door_max)| {
            (self.start.as_() + wall_dir.to_vec2().as_() * (door_min + door_max) as f32 / 2.0 + 0.5)
                .with_z(self.base_alt as f32)
        })
    }

    pub fn door_bounds(&self) -> Option<Aabr<i32>> {
        let wall_dir = Dir::from_vec2(self.end - self.start);

        self.door.map(|(door_min, door_max)| {
            Aabr {
                min: self.start + wall_dir.to_vec2() * door_min,
                max: self.start + wall_dir.to_vec2() * door_max,
            }
            .made_valid()
        })
    }
}

#[derive(Copy, Clone)]
enum RoofStyle {
    Flat,
    FlatBars { dir: Dir },
    LeanTo { dir: Dir, max_z: i32 },
    Gable { dir: Dir, max_z: i32 },
    Hip { max_z: i32 },
}

struct Roof {
    bounds: Aabr<i32>,
    min_z: i32,
    style: RoofStyle,
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
pub enum Detail {
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

pub struct Room {
    /// Inclusive
    pub bounds: Aabb<i32>,
    kind: RoomKind,
    // stairs: Option<Id<Stairs>>,
    walls: EnumMap<Dir, Vec<Id<Wall>>>,
    roofs: Vec<Id<Roof>>,
    detail_areas: Vec<Aabr<i32>>,
    pub details: Vec<Detail>,
}

impl Room {
    fn new(bounds: Aabb<i32>, kind: RoomKind) -> Self {
        Self {
            bounds,
            kind,
            roofs: Default::default(),
            walls: Default::default(),
            detail_areas: Default::default(),
            details: Default::default(),
        }
    }

    /// Are any of this rooms roofs fully covering it?
    fn is_covered_by_roof(&self, roofs: &Store<Roof>) -> bool {
        let aabr = Aabr {
            min: self.bounds.min.xy(),
            max: self.bounds.max.xy(),
        };
        for roof in self.roofs.iter() {
            if roofs[*roof].bounds.contains_aabr(aabr) {
                return true;
            }
        }
        false
    }
}

pub struct Tavern {
    name: String,
    pub rooms: Store<Room>,
    walls: Store<Wall>,
    roofs: Store<Roof>,
    /// Tile position of the door tile
    pub door_tile: Vec2<i32>,
    pub door_wpos: Vec3<i32>,
    /// Axis aligned bounding region for the house
    pub bounds: Aabr<i32>,
}

impl Tavern {
    pub fn generate(
        land: &Land,
        _index: IndexRef,
        rng: &mut impl Rng,
        site: &Site,
        door_tile: Vec2<i32>,
        door_dir: Dir,
        tile_aabr: Aabr<i32>,
    ) -> Self {
        let name = namegen::NameGen::location(rng).generate_tavern();

        let mut rooms = Store::default();
        let mut walls = Store::default();
        let mut roofs = Store::default();
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

        let door_alt = land.get_alt_approx(door_wpos);
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
            let door_center = door_dir.rotated_cw().select(door_wpos.xy() - start).abs();
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
                door: Some((door_center - 1, door_center + 1)),
            });
            rooms[entrance_id].walls[door_dir].push(wall_id);

            room_metas.push(RoomMeta {
                id: entrance_id,
                walls: Dir::iter().filter(|d| *d != door_dir).collect(),
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
        'room_gen: while !room_metas.is_empty() {
            // Continue extending from a random existing room
            let mut room_meta = room_metas.swap_remove(rng.gen_range(0..room_metas.len()));
            if room_meta.walls.is_empty() {
                continue 'room_gen;
            }

            // Pick a direction to choose from
            let Some(in_dir) = room_meta.walls.into_iter().choose(rng) else {
                continue 'room_gen;
            };
            room_meta.walls.remove(in_dir);

            let right = in_dir.orthogonal();
            let left = -right;

            let from_id = room_meta.id;
            let from_room = &rooms[from_id];

            // If there are more directions to continue from, push this room again.
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
            // Pick a  height of the new room
            let room_hgt = rng.gen_range(3..=5);
            let wanted_alt = land.get_alt_approx(max_bounds.center()) as i32 + 1;
            let max_stair_length = (in_dir.select(if wanted_alt < from_room.bounds.min.z {
                from_bounds.size()
            } else {
                max_bounds.size()
            }) / 2)
                .min(5);
            let alt = wanted_alt.clamp(
                from_room.bounds.min.z - max_stair_length,
                from_room.bounds.min.z + max_stair_length,
            );
            let min_z = from_room.bounds.min.z.min(alt);
            let max_z = from_room.bounds.max.z.max(alt + room_hgt);

            // Take other rooms into account when calculating `max_bounds`. We don't care
            // about this room if it's the originating room or at another
            // height.
            for (_, room) in rooms.iter().filter(|(room_id, room)| {
                *room_id != from_id
                    && room.bounds.min.z - 1 <= max_z
                    && room.bounds.max.z + 1 >= min_z
            }) {
                let bounds = to_aabr(room.bounds);
                let bounds = extend_aabr(bounds, 2);
                let intersection = bounds.intersection(max_bounds);
                if intersection.is_valid() {
                    // Find the direction to shrink in that yields the highest area.
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
                                0.5 / (1.0 + room_counts[RoomKind::Garden] as f32 * 0.8)
                            },
                            RoomKind::StageRoom => {
                                2.0 / (1.0 + room_counts[RoomKind::StageRoom] as f32).powi(2)
                            },
                            RoomKind::BarRoom => {
                                2.0 / (1.0 + room_counts[RoomKind::BarRoom] as f32).powi(2)
                            },
                            RoomKind::EntranceRoom => {
                                0.05 / (1.0 + room_counts[RoomKind::EntranceRoom] as f32)
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
            let room_kind = *room_lottery.choose_seeded(rng.gen());

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

            // Place the room in the given max bounds
            let Some(bounds) = place_room_in(room_kind, max_bounds, in_dir, in_pos, rng) else {
                continue 'room_gen;
            };

            let bounds3 = Aabb {
                min: bounds.min.with_z(alt),
                max: bounds.max.with_z(alt + room_hgt),
            };
            let id = rooms.insert(Room::new(bounds3, room_kind));

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

            let door_center = right.select(in_pos - start);
            let b = rng.gen_bool(0.5);
            let door_min = door_center - b as i32;
            let door_max = door_center - (!b) as i32;
            let wall_id = walls.insert(Wall {
                start,
                end,
                base_alt: min_z,
                top_alt: max_z,
                from: Some(from_id),
                to: Some(id),
                to_dir: in_dir,
                door: Some((door_min, door_max)),
            });

            rooms[id].walls[-in_dir].push(wall_id);
            rooms[from_id].walls[in_dir].push(wall_id);

            room_metas.push(RoomMeta {
                id,
                walls: Dir::iter().filter(|d| *d != -in_dir).collect(),
            });
            room_counts[room_kind] += 1;
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
            // Split the wall into parts.
            let mut split_range = |dir: Dir, min: i32, max: i32| {
                debug_assert!(min <= max);
                let mut new_ranges = Vec::new();
                wall_ranges[dir].retain_mut(|(r_min, r_max)| {
                    if *r_min <= max && *r_max >= min {
                        match (*r_min >= min, *r_max <= max) {
                            (true, true) => false,
                            (true, false) => {
                                *r_min = max + 1;
                                true
                            },
                            (false, true) => {
                                *r_max = min - 1;
                                true
                            },
                            (false, false) => {
                                new_ranges.push((max + 1, *r_max));
                                *r_max = min - 1;
                                true
                            },
                        }
                    } else {
                        true
                    }
                });
                wall_ranges[dir].extend(new_ranges);
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
                if a_min_z >= b_max_z || a_max_z <= b_min_z {
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
                    let door = if max - min > 2 && max_z - min_z > 3 && rng.gen_bool(0.8) {
                        let door_center = rng.gen_range(1..=max - min - 2);
                        Some((door_center, door_center + 1))
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
            let walls = &walls;
            let mut avoid = room
                .walls
                .iter()
                .flat_map(|(dir, dir_walls)| {
                    dir_walls.iter().filter_map(move |wall_id| {
                        let wall = &walls[*wall_id];

                        let door_bounds = wall.door_bounds()?;

                        Some(
                            Aabr {
                                min: dir.select_aabr_with(bounds, door_bounds.min),
                                max: dir.select_with(bounds.center(), door_bounds.max),
                            }
                            .made_valid(),
                        )
                    })
                })
                .collect::<Vec<_>>();

            let mut x = bounds.min.x;
            // Basically greedy meshing, but for aabrs
            while x <= bounds.max.x {
                let mut y = bounds.min.y;
                'y_loop: while y <= bounds.max.y {
                    let min = Vec2::new(x, y);
                    let mut max_y = bounds.max.y;
                    for area in avoid.iter() {
                        let contains_x = area.min.x <= min.x && min.x <= area.max.x;
                        let contains_y = area.min.y <= min.y && min.y <= area.max.y;
                        if contains_x && contains_y {
                            y = area.max.y + 1;
                            continue 'y_loop;
                        }

                        if contains_x && min.y < area.min.y && area.min.y - 1 < max_y {
                            max_y = area.min.y - 1;
                        }
                    }

                    let max_x = avoid
                        .iter()
                        .filter_map(|area| {
                            if area.min.x > x && area.min.y <= max_y && area.max.y >= min.y {
                                Some(area.min.x - 1)
                            } else {
                                None
                            }
                        })
                        .min()
                        .unwrap_or(bounds.max.x);

                    let area = Aabr {
                        min,
                        max: Vec2::new(max_x, max_y),
                    };
                    avoid.push(area);
                    room.detail_areas.push(area);
                    y = max_y + 1;
                }
                x += 1;
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

        for room_id in rooms.ids() {
            let room = &rooms[room_id];
            // If a room is already fully covered by a roof, we skip it.
            if room.is_covered_by_roof(&roofs) {
                continue;
            }
            let roof_min_z = room.bounds.max.z + 1;
            let mut roof_bounds = to_aabr(room.bounds);
            roof_bounds.min -= 2;
            roof_bounds.max += 2;
            let mut dirs = Vec::from(Dir::ALL);

            let mut over_rooms = vec![room_id];
            // Extend roof over adjecent rooms.
            while !dirs.is_empty() {
                let dir = dirs.swap_remove(rng.gen_range(0..dirs.len()));
                let orth = dir.orthogonal();
                // Check for room intersections in this direction.
                for (room_id, room) in rooms.iter() {
                    let room_aabr = to_aabr(room.bounds);
                    if room.bounds.max.z == roof_min_z
                        && dir.select_aabr(roof_bounds) + dir.signum()
                            == (-dir).select_aabr(room_aabr)
                        && orth.select_aabr(roof_bounds) <= orth.select_aabr(room_aabr) + 2
                        && (-orth).select_aabr(roof_bounds) >= (-orth).select_aabr(room_aabr) - 2
                    {
                        // If the room we found is fully covered by a roof already, we don't go in
                        // this direction.
                        if room.is_covered_by_roof(&roofs) {
                            break;
                        }
                        roof_bounds = dir.extend_aabr(roof_bounds, dir.select(room_aabr.size()));
                        dirs.push(dir);
                        over_rooms.push(room_id);
                        break;
                    }
                }
            }

            // Build a lottery of valid roofs to pick from
            let mut valid_styles = vec![(0.5, RoofStyle::Flat)];

            let gardens = over_rooms
                .iter()
                .filter(|id| matches!(rooms[**id].kind, RoomKind::Garden))
                .count();

            // If we just have gardens, we can use FlatBars style.
            if gardens == over_rooms.len() {
                let ratio = Dir::X.select(roof_bounds.size()) as f32
                    / Dir::Y.select(roof_bounds.size()) as f32;
                valid_styles.extend([
                    (5.0 * ratio, RoofStyle::FlatBars { dir: Dir::X }),
                    (5.0 / ratio, RoofStyle::FlatBars { dir: Dir::Y }),
                ]);
            }

            // Find heights of possible adjecent rooms.
            let mut dir_zs = EnumMap::default();
            for dir in Dir::iter() {
                let orth = dir.orthogonal();
                for room in rooms.values() {
                    let room_aabr = to_aabr(room.bounds);
                    if room.bounds.max.z > roof_min_z
                        && dir.select_aabr(roof_bounds) == (-dir).select_aabr(room_aabr)
                        && orth.select_aabr(roof_bounds) <= orth.select_aabr(room_aabr) + 2
                        && (-orth).select_aabr(roof_bounds) >= (-orth).select_aabr(room_aabr) - 2
                    {
                        dir_zs[dir] = Some(room.bounds.max.z);
                        break;
                    }
                }
            }

            for dir in [Dir::X, Dir::Y] {
                if dir_zs[dir.orthogonal()].is_none() && dir_zs[-dir.orthogonal()].is_none() {
                    let max_z =
                        roof_min_z + (dir.orthogonal().select(roof_bounds.size()) / 2 - 1).min(7);
                    let max_z = match (dir_zs[dir], dir_zs[-dir]) {
                        (Some(a), Some(b)) => {
                            if a.min(b) >= roof_min_z + 3 {
                                max_z.min(a.min(b))
                            } else {
                                max_z
                            }
                        },
                        (None, None) => max_z,
                        _ => continue,
                    };

                    for max_z in roof_min_z + 3..=max_z {
                        valid_styles.push((1.0, RoofStyle::Gable { dir, max_z }))
                    }
                }
            }

            for dir in Dir::iter() {
                if let (Some(h), None) = (dir_zs[dir], dir_zs[-dir]) {
                    for max_z in roof_min_z + 2..=h {
                        valid_styles.push((1.0, RoofStyle::LeanTo { dir, max_z }))
                    }
                }
            }

            if Dir::iter().all(|d| dir_zs[d].is_none()) {
                for max_z in roof_min_z + 3..=roof_min_z + 7 {
                    valid_styles.push((0.8, RoofStyle::Hip { max_z }))
                }
            }

            let style_lottery = Lottery::from(valid_styles);

            debug_assert!(
                roof_bounds.is_valid(),
                "Roof bounds aren't valid: {:?}",
                roof_bounds
            );
            let roof_id = roofs.insert(Roof {
                bounds: roof_bounds,
                min_z: roof_min_z,
                style: *style_lottery.choose_seeded(rng.gen()),
            });

            for room_id in over_rooms {
                rooms[room_id].roofs.push(roof_id);
            }
        }

        Self {
            name,
            rooms,
            walls,
            roofs,
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
        let field = RandomField::new(740384);

        const DOWN: i32 = 6;

        let mut offset = 0;
        let mut choose = |slice: &[Rgb<u8>]| -> Rgb<u8> {
            offset += 1;
            *field
                .choose(self.door_wpos + offset, slice)
                .expect("Color slice should not be empty.")
        };

        let detail_fill = Fill::Brick(
            BlockKind::Rock,
            choose(&[
                Rgb::new(55, 65, 64),
                Rgb::new(46, 62, 100),
                Rgb::new(46, 100, 62),
                Rgb::new(100, 100, 105),
            ]),
            15,
        );
        let wall_fill = Fill::Brick(
            BlockKind::Wood,
            choose(&[
                Rgb::new(160, 53, 34),
                Rgb::new(147, 51, 29),
                Rgb::new(147, 101, 69),
                Rgb::new(90, 90, 95),
                Rgb::new(170, 140, 52),
            ]),
            20,
        );
        let wall_detail_fill = Fill::Brick(
            BlockKind::Wood,
            choose(&[Rgb::new(108, 100, 79), Rgb::new(150, 150, 150)]),
            25,
        );
        let floor_fill = Fill::Brick(
            BlockKind::Wood,
            choose(&[Rgb::new(42, 44, 43), Rgb::new(56, 18, 10)]),
            10,
        );
        let roof_fill = Fill::Brick(
            BlockKind::Wood,
            choose(&[
                Rgb::new(21, 43, 48),
                Rgb::new(11, 23, 38),
                Rgb::new(45, 28, 21),
                Rgb::new(10, 55, 40),
                Rgb::new(5, 35, 15),
                Rgb::new(40, 5, 11),
                Rgb::new(55, 45, 11),
            ]),
            20,
        );
        let simple_roof_fill = Fill::Brick(
            BlockKind::Wood,
            choose(&[Rgb::new(106, 73, 64), Rgb::new(85, 52, 43)]),
            20,
        );

        let get_kind = |room| self.rooms.get(room).kind;
        let get_door_stair = |wall: &Wall, door: Aabr<i32>| {
            let filter = |room: &Id<Room>| self.rooms[*room].bounds.min.z > wall.base_alt;
            wall.to
                .filter(filter)
                .zip(Some(wall.to_dir))
                .or(wall.from.filter(filter).zip(Some(-wall.to_dir)))
                .map(|(room, to_dir)| {
                    let room = &self.rooms[room];

                    let max = door.max + to_dir.to_vec2() * (room.bounds.min.z - wall.base_alt + 1);
                    (door.min, max, room, to_dir)
                })
        };

        for roof in self.roofs.values() {
            match roof.style {
                RoofStyle::Flat => {
                    painter
                        .aabb(aabb(Aabb {
                            min: roof.bounds.min.with_z(roof.min_z),
                            max: roof.bounds.max.with_z(roof.min_z),
                        }))
                        .fill(roof_fill.clone());
                },
                RoofStyle::FlatBars { dir } => painter
                    .aabb(aabb(Aabb {
                        min: dir
                            .select_aabr_with(roof.bounds, roof.bounds.min)
                            .with_z(roof.min_z),
                        max: dir
                            .select_aabr_with(roof.bounds, roof.bounds.max)
                            .with_z(roof.min_z),
                    }))
                    .repeat(
                        -dir.to_vec3() * 2,
                        (dir.select(roof.bounds.size()) as u32 + 3) / 2,
                    )
                    .fill(simple_roof_fill.clone()),
                RoofStyle::LeanTo { dir, max_z } => {
                    painter
                        .aabb(aabb(Aabb {
                            min: roof.bounds.min.with_z(roof.min_z),
                            max: roof.bounds.max.with_z(roof.min_z),
                        }))
                        .fill(roof_fill.clone());
                    painter
                        .ramp(
                            aabb(Aabb {
                                min: roof.bounds.min.with_z(roof.min_z),
                                max: roof.bounds.max.with_z(max_z),
                            }),
                            dir,
                        )
                        .fill(roof_fill.clone());
                    for d in [dir.orthogonal(), -dir.orthogonal()] {
                        painter
                            .ramp(
                                aabb(Aabb {
                                    min: (d.select_aabr_with(roof.bounds, roof.bounds.min)
                                        - d.to_vec2())
                                    .with_z(roof.min_z - 1),
                                    max: (d.select_aabr_with(roof.bounds, roof.bounds.max)
                                        - d.to_vec2())
                                    .with_z(max_z - 1),
                                }),
                                dir,
                            )
                            .fill(wall_fill.clone());
                        painter
                            .ramp(
                                aabb(Aabb {
                                    min: d
                                        .select_aabr_with(roof.bounds, roof.bounds.min)
                                        .with_z(roof.min_z - 1),
                                    max: d
                                        .select_aabr_with(roof.bounds, roof.bounds.max)
                                        .with_z(max_z - 1),
                                }),
                                dir,
                            )
                            .clear();
                    }
                },
                RoofStyle::Gable { dir, max_z } => {
                    painter
                        .gable(
                            aabb(Aabb {
                                min: roof.bounds.min.with_z(roof.min_z),
                                max: roof.bounds.max.with_z(max_z),
                            }),
                            max_z - roof.min_z + 1,
                            dir,
                        )
                        .fill(roof_fill.clone());
                    for dir in [dir, -dir] {
                        painter
                            .gable(
                                aabb(Aabb {
                                    min: (dir.select_aabr_with(roof.bounds, roof.bounds.min + 1)
                                        - dir.to_vec2())
                                    .with_z(roof.min_z),
                                    max: (dir.select_aabr_with(roof.bounds, roof.bounds.max - 1)
                                        - dir.to_vec2())
                                    .with_z(max_z - 1),
                                }),
                                max_z - roof.min_z,
                                dir,
                            )
                            .fill(wall_fill.clone());
                        painter
                            .aabb(aabb(Aabb {
                                min: (dir.select_aabr_with(roof.bounds, roof.bounds.min + 1)
                                    - dir.to_vec2())
                                .with_z(roof.min_z),
                                max: (dir.select_aabr_with(roof.bounds, roof.bounds.max - 1)
                                    - dir.to_vec2())
                                .with_z(roof.min_z),
                            }))
                            .fill(wall_detail_fill.clone());
                        let center_bounds = Aabr {
                            min: (dir.select_aabr_with(roof.bounds, roof.bounds.center())
                                - dir.to_vec2()),
                            max: (dir.select_aabr_with(
                                roof.bounds,
                                (roof.bounds.min + roof.bounds.max + 1) / 2,
                            ) - dir.to_vec2()),
                        };
                        painter
                            .aabb(aabb(Aabb {
                                min: center_bounds.min.with_z(roof.min_z),
                                max: center_bounds.max.with_z(max_z - 1),
                            }))
                            .fill(wall_detail_fill.clone());
                        for d in [dir.orthogonal(), -dir.orthogonal()] {
                            let hgt = max_z - roof.min_z;
                            let half_size = d.select(roof.bounds.size() + 1) / 2;
                            let e = half_size - hgt + 1;
                            let e = e - e % 2;
                            let f = half_size - e;
                            let hgt = (hgt - 1).min(e - f % 2) - (d.signum() - 1) / 2;
                            let mut aabr = Aabr {
                                min: d.select_aabr_with(center_bounds, center_bounds.min),
                                max: d.select_aabr_with(center_bounds, center_bounds.max)
                                    + d.to_vec2() * hgt,
                            }
                            .made_valid();
                            aabr.max += 1;
                            painter
                                .plane(
                                    aabr,
                                    aabr.min
                                        .with_z(if d.signum() < 0 {
                                            roof.min_z + hgt
                                        } else {
                                            roof.min_z
                                        })
                                        .as_(),
                                    d.to_vec2().as_(),
                                )
                                .fill(wall_detail_fill.clone());
                        }
                        painter
                            .gable(
                                aabb(Aabb {
                                    min: dir
                                        .select_aabr_with(roof.bounds, roof.bounds.min + 1)
                                        .with_z(roof.min_z),
                                    max: dir
                                        .select_aabr_with(roof.bounds, roof.bounds.max - 1)
                                        .with_z(max_z - 1),
                                }),
                                max_z - roof.min_z,
                                dir,
                            )
                            .clear();
                    }
                },
                RoofStyle::Hip { max_z } => {
                    painter
                        .pyramid(aabb(Aabb {
                            min: roof.bounds.min.with_z(roof.min_z),
                            max: roof.bounds.max.with_z(max_z),
                        }))
                        .fill(roof_fill.clone());
                },
            }
        }

        for room in self.rooms.values() {
            painter
                .aabb(aabb(Aabb {
                    min: room.bounds.min.with_z(room.bounds.min.z - DOWN),
                    max: room.bounds.max.with_z(room.bounds.min.z - 1),
                }))
                .fill(floor_fill.clone());
        }
        for wall in self.walls.values() {
            let wall_aabb = Aabb {
                min: wall.start.with_z(wall.base_alt),
                max: wall.end.with_z(wall.top_alt),
            };
            let wall_dir = Dir::from_vec2(wall.end - wall.start);
            match (wall.from.map(get_kind), wall.to.map(get_kind)) {
                (Some(RoomKind::Garden), None) | (None, Some(RoomKind::Garden)) => {
                    let hgt = wall_aabb.min.z..=wall_aabb.max.z;
                    painter
                        .column(wall_aabb.min.xy(), hgt.clone())
                        .fill(wall_detail_fill.clone());
                    painter
                        .column(wall_aabb.max.xy(), hgt)
                        .fill(wall_detail_fill.clone());
                    let z = (wall.base_alt + wall.top_alt) / 2;

                    painter
                        .aabb(aabb(Aabb {
                            min: (wall_aabb.min + wall_dir.to_vec2()).with_z(wall_aabb.min.z + 1),
                            max: (wall_aabb.max - wall_dir.to_vec2()).with_z(wall_aabb.max.z - 1),
                        }))
                        .clear();

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
                            min: wall_aabb.min.with_z(wall_aabb.min.z - DOWN),
                            max: wall_aabb.max.with_z(wall_aabb.min.z),
                        }))
                        .fill(wall_detail_fill.clone());
                    painter
                        .aabb(aabb(Aabb {
                            min: wall_aabb.min.with_z(wall_aabb.max.z),
                            max: wall_aabb.max,
                        }))
                        .fill(wall_detail_fill.clone());
                },
                (Some(RoomKind::Garden), Some(RoomKind::Garden)) => {
                    painter
                        .aabb(aabb(Aabb {
                            min: wall_aabb.min.with_z(wall_aabb.min.z - DOWN),
                            max: wall_aabb.max.with_z(wall_aabb.min.z - 1),
                        }))
                        .fill(floor_fill.clone());
                    painter.aabb(aabb(wall_aabb)).clear();
                },
                (None, None) => {},
                _ => {
                    painter
                        .aabb(aabb(Aabb {
                            min: wall_aabb.min.with_z(wall_aabb.min.z - DOWN),
                            max: wall_aabb.max,
                        }))
                        .fill(wall_fill.clone());
                    painter
                        .column(wall.start, wall.base_alt - DOWN..=wall.top_alt)
                        .fill(wall_detail_fill.clone());
                    painter
                        .column(wall.end, wall.base_alt - DOWN..=wall.top_alt)
                        .fill(wall_detail_fill.clone());
                },
            }
            if let Some(door) = wall.door_bounds() {
                let orth = wall.to_dir.orthogonal();
                if let Some((min, max, room, to_dir)) = get_door_stair(wall, door) {
                    painter
                        .aabb(aabb(Aabb {
                            min: (min + to_dir.to_vec2() - orth.to_vec2())
                                .with_z(wall.base_alt - 1),
                            max: (max + orth.to_vec2()).with_z(room.bounds.min.z - 1),
                        }))
                        .fill(floor_fill.clone());
                }
            }
        }
        for room in self.rooms.values() {
            painter.aabb(aabb(room.bounds)).clear();

            let room_aabr = Aabr {
                min: room.bounds.min.xy(),
                max: room.bounds.max.xy(),
            };
            match room.kind {
                RoomKind::Garden => {},
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
                                        .fill(wall_detail_fill.clone());
                                    painter
                                        .aabb(aabb(Aabb {
                                            min: min.with_z(room.bounds.min.z + 3),
                                            max: max.with_z(room.bounds.max.z),
                                        }))
                                        .fill(wall_detail_fill.clone());
                                },
                                (true, true) => {
                                    painter.sprite(
                                        dir.abs().vec2(edge, rot_edge).with_z(room.bounds.min.z),
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
                            .fill(detail_fill.clone());
                        painter
                            .aabb(aabb(Aabb {
                                min: (aabr.min + 1).with_z(room.bounds.min.z),
                                max: (aabr.max - 1).with_z(room.bounds.min.z),
                            }))
                            .fill(wall_fill.clone());
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
                                .fill(wall_detail_fill.clone());

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

        for wall in self.walls.values() {
            let kinds = (wall.from.map(get_kind), wall.to.map(get_kind));
            let in_dir_room = if let (Some(room), to @ None) | (None, to @ Some(room)) = kinds {
                let in_dir = if to.is_none() {
                    -wall.to_dir
                } else {
                    wall.to_dir
                };

                Some((in_dir, room))
            } else {
                None
            };
            if let Some((in_dir, room)) = in_dir_room {
                let width = in_dir.orthogonal().select(wall.end - wall.start).abs();
                let wall_center = (wall.start + wall.end) / 2;
                let door_dist = wall.door_bounds().map_or(i32::MAX, |door| {
                    (door.min - wall_center)
                        .map(|x| x.abs())
                        .reduce_max()
                        .max((door.max - wall_center).map(|x| x.abs()).reduce_max())
                });
                match room {
                    RoomKind::Garden => {
                        if door_dist >= 2 {
                            painter.rotated_sprite(
                                wall_center.with_z(wall.base_alt + 1),
                                SpriteKind::Planter,
                                in_dir.sprite_ori(),
                            );
                        }
                    },
                    _ => {
                        if width >= 5 && door_dist > 3 {
                            painter
                                .aabb(aabb(Aabb {
                                    min: (wall_center + in_dir.rotated_ccw().to_vec2())
                                        .with_z(wall.base_alt + 1),
                                    max: (wall_center + in_dir.rotated_cw().to_vec2())
                                        .with_z(wall.base_alt + 2),
                                }))
                                .fill(Fill::RotatedSprite(
                                    SpriteKind::Window1,
                                    in_dir.sprite_ori(),
                                ));
                        }
                    },
                }
            }
            if let Some(door) = wall.door_bounds()
                && !matches!(kinds, (Some(RoomKind::Garden), Some(RoomKind::Garden)))
            {
                let orth = wall.to_dir.orthogonal();
                painter
                    .aabb(aabb(Aabb {
                        min: (door.min - orth.to_vec2()).with_z(wall.base_alt),
                        max: (door.max + orth.to_vec2()).with_z(wall.base_alt + 3),
                    }))
                    .fill(detail_fill.clone());
                painter
                    .aabb(aabb(Aabb {
                        min: (door.min - orth.to_vec2()).with_z(wall.base_alt - 1),
                        max: (door.max + orth.to_vec2()).with_z(wall.base_alt - 1),
                    }))
                    .fill(floor_fill.clone());
                painter
                    .aabb(aabb(Aabb {
                        min: (door.min + wall.to_dir.to_vec2()).with_z(wall.base_alt),
                        max: (door.max - wall.to_dir.to_vec2()).with_z(wall.base_alt + 2),
                    }))
                    .clear();
                if let Some((min, max, room, to_dir)) = get_door_stair(wall, door) {
                    // Place a ramp if the door is lower than the room alt.
                    painter
                        .ramp(
                            aabb(Aabb {
                                min: (min - to_dir.to_vec2() * 3).with_z(wall.base_alt),
                                max: max.with_z(room.bounds.min.z + 2),
                            }),
                            to_dir,
                        )
                        // TOOD: For zoomy worldgen, this a sheared aabb.
                        .without(
                            painter
                                .ramp(
                                    aabb(Aabb {
                                        min: (min + to_dir.to_vec2() * 2).with_z(wall.base_alt),
                                        max: max.with_z(room.bounds.min.z - 1),
                                    }),
                                    to_dir,
                                )
                        )
                        .clear();
                }
                if let Some((in_dir, _room)) = in_dir_room {
                    let sprite = match in_dir.rotated_cw().select(door.size()) {
                        2.. => SpriteKind::DoorWide,
                        _ => SpriteKind::Door,
                    };
                    painter.rotated_sprite(
                        in_dir
                            .rotated_cw()
                            .select_aabr_with(door, door.min)
                            .with_z(wall.base_alt),
                        sprite,
                        in_dir.sprite_ori(),
                    );
                    painter.rotated_sprite(
                        in_dir
                            .rotated_ccw()
                            .select_aabr_with(door, door.min)
                            .with_z(wall.base_alt),
                        sprite,
                        in_dir.opposite().sprite_ori(),
                    );

                    let dir = match field.chance(door.min.with_z(wall.base_alt), 0.5) {
                        true => in_dir.rotated_cw(),
                        false => in_dir.rotated_ccw(),
                    };

                    let pos =
                        dir.select_aabr_with(door, door.min) + dir.to_vec2() - in_dir.to_vec2();

                    painter.rotated_sprite_with_cfg(
                        pos.with_z(wall.base_alt + 2),
                        SpriteKind::HangingSign,
                        in_dir.opposite().sprite_ori(),
                        SpriteCfg {
                            unlock: None,
                            content: Some(Content::Plain(self.name.clone())),
                        },
                    );
                }
            }
        }
    }
}
