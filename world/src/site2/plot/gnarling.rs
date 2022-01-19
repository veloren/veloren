use super::*;
use crate::{assets::AssetHandle, site2::util::Dir, util::attempt, Land};
use common::{
    generation::{ChunkSupplement, EntityInfo},
    terrain::{Structure as PrefabStructure, StructuresGroup},
};
use inline_tweak::tweak;
use kiddo::{distance::squared_euclidean, KdTree};
use lazy_static::lazy_static;
use rand::prelude::*;
use std::collections::HashMap;
use vek::*;

pub struct GnarlingFortification {
    name: String,
    seed: u32,
    origin: Vec2<i32>,
    radius: i32,
    wall_radius: i32,
    wall_segments: Vec<(Vec2<i32>, Vec2<i32>)>,
    wall_towers: Vec<Vec3<i32>>,
    // Structure indicates the kind of structure it is, vec2 is relative position of a hut compared
    // to origin, ori tells which way structure should face
    structure_locations: Vec<(GnarlingStructure, Vec3<i32>, Ori)>,
    tunnels: Tunnels,
}

struct Tunnels {
    start: Vec3<i32>,
    end: Vec3<i32>,
    branches: Vec<(Vec3<i32>, Vec3<i32>)>,
    terminals: Vec<Vec3<i32>>,
}

enum GnarlingStructure {
    Hut,
    Longhut,
    Totem,
    ChieftainHut,
    WatchTower,
    Banner,
}

impl GnarlingStructure {
    fn required_separation(&self, other: &Self) -> i32 {
        match (self, other) {
            (Self::Hut, Self::Hut) => 15,
            (_, Self::Longhut) | (Self::Longhut, _) => 25,
            (_, Self::Banner) | (Self::Banner, _) => 15,

            (Self::Hut, Self::Totem) | (Self::Totem, Self::Hut) => 20,
            (Self::Totem, Self::Totem) => 50,
            // Chieftain hut and watch tower generated in separate pass without distance check
            (Self::ChieftainHut | Self::WatchTower, _)
            | (_, Self::ChieftainHut | Self::WatchTower) => 0,
        }
    }
}

impl GnarlingFortification {
    pub fn generate(wpos: Vec2<i32>, land: &Land, rng: &mut impl Rng) -> Self {
        let rpos_height = |rpos| land.get_alt_approx(rpos + wpos) as i32;

        let name = String::from("Gnarling Fortification");
        let seed = rng.gen();
        let origin = wpos;

        let wall_radius = {
            let unit_size = rng.gen_range(10..20);
            let num_units = rng.gen_range(5..10);
            let variation = rng.gen_range(0..50);
            unit_size * num_units + variation
        };

        let radius = wall_radius + 50;

        // Tunnels
        let alt = land.get_alt_approx(wpos) as i32;
        let start = wpos.with_z(alt - 8);
        let boss_room_shift = rng.gen_range(70..110);
        let end_xy = match rng.gen_range(0..4) {
            0 => Vec2::new(start.x + boss_room_shift, start.y + boss_room_shift),
            1 => Vec2::new(start.x - boss_room_shift, start.y + boss_room_shift),
            2 => Vec2::new(start.x - boss_room_shift, start.y - boss_room_shift),
            3 => Vec2::new(start.x + boss_room_shift, start.y - boss_room_shift),
            // Unreachable
            _ => unreachable!(),
        };

        let is_underground = |pos: Vec3<i32>| land.get_alt_approx(pos.xy()) as i32 - 9 > pos.z;
        let is_valid_edge = |p1: Vec3<i32>, p2: Vec3<i32>| {
            let diff = p1 - p2;
            // Check that the new point is underground and the slope is mildish
            is_underground(p2) && (diff.z.pow(2) / diff.xy().magnitude_squared().max(1)) < 3
        };
        let mut end = end_xy.with_z(start.z - 20);
        for i in 1..31 {
            let new_z = start.z - (i * 20);
            if is_underground(end_xy.with_z(new_z + 35)) {
                end = end_xy.with_z(new_z);
                break;
            }
        }

        let (branches, terminals) =
            rrt(start, end, is_valid_edge, rng).unwrap_or((Vec::new(), Vec::new()));

        let tunnels = Tunnels {
            start,
            end,
            branches,
            terminals,
        };

        let num_points = (wall_radius / 15).max(5);
        let outer_wall_corners = (0..num_points)
            .into_iter()
            .map(|a| {
                let angle = a as f32 / num_points as f32 * core::f32::consts::TAU;
                Vec2::new(angle.cos(), angle.sin()).map(|a| (a * wall_radius as f32) as i32)
            })
            .map(|point| {
                point.map(|a| {
                    let variation = wall_radius / 5;
                    a + rng.gen_range(-variation..=variation)
                })
            })
            .collect::<Vec<_>>();

        let gate_index = rng.gen_range(0..outer_wall_corners.len());

        let chieftain_indices = {
            // Will not be adjacent to gate and needs two sections, so subtract 4 (3 to get
            // rid of gate and adjacent, 1 to account for taking 2 sections)
            let chosen = rng.gen_range(0..(outer_wall_corners.len() - 4));
            let index = if gate_index < 2 {
                chosen + gate_index + 2
            } else if chosen < (gate_index - 2) {
                chosen
            } else {
                chosen + 4
            };
            [index, (index + 1) % outer_wall_corners.len()]
        };

        // TODO: Figure out how to resolve the allow
        #[allow(clippy::needless_collect)]
        let outer_wall_segments = outer_wall_corners
            .iter()
            .enumerate()
            .filter_map(|(i, point)| {
                if i == gate_index {
                    None
                } else {
                    let next_point = if let Some(point) = outer_wall_corners.get(i + 1) {
                        *point
                    } else {
                        outer_wall_corners[0]
                    };
                    Some((*point, next_point))
                }
            })
            .collect::<Vec<_>>();

        // Structures will not spawn in wall corner triangles corresponding to these
        // indices
        let forbidden_indices = [gate_index, chieftain_indices[0], chieftain_indices[1]];
        // Structures will be weighted to spawn further from these indices when
        // selecting a point in the triangle
        let restricted_indices = [
            (chieftain_indices[0] + outer_wall_corners.len() - 1) % outer_wall_corners.len(),
            (chieftain_indices[1] + 1) % outer_wall_corners.len(),
        ];

        let desired_structures = wall_radius.pow(2) / 100;
        let mut structure_locations = Vec::<(GnarlingStructure, Vec3<i32>, Ori)>::new();
        for _ in 0..desired_structures {
            if let Some((hut_loc, kind)) = attempt(16, || {
                // Choose structure kind
                let structure_kind = match rng.gen_range(0..10) {
                    0 => GnarlingStructure::Totem,
                    1..=3 => GnarlingStructure::Longhut,
                    4..=5 => GnarlingStructure::Banner,
                    _ => GnarlingStructure::Hut,
                };

                // Choose triangle
                let corner_1_index = rng.gen_range(0..outer_wall_corners.len());

                if forbidden_indices.contains(&corner_1_index) {
                    return None;
                }

                let center = Vec2::zero();
                let corner_1 = outer_wall_corners[corner_1_index];
                let (corner_2, corner_2_index) =
                    if let Some(corner) = outer_wall_corners.get(corner_1_index + 1) {
                        (*corner, corner_1_index + 1)
                    } else {
                        (outer_wall_corners[0], 0)
                    };

                let center_weight: f32 = rng.gen_range(0.2..0.6);

                // Forbidden and restricted indices are near walls, so don't spawn structures
                // too close to avoid overlap with wall
                let corner_1_weight_range = if restricted_indices.contains(&corner_1_index) {
                    let limit = 0.75;
                    if chieftain_indices.contains(&corner_2_index) {
                        ((1.0 - center_weight) * (1.0 - limit))..(1.0 - center_weight)
                    } else {
                        0.0..((1.0 - center_weight) * limit)
                    }
                } else {
                    0.0..(1.0 - center_weight)
                };

                let corner_1_weight = rng.gen_range(corner_1_weight_range);
                let corner_2_weight = 1.0 - center_weight - corner_1_weight;

                let structure_center: Vec2<i32> = (center * center_weight
                    + corner_1.as_() * corner_1_weight
                    + corner_2.as_() * corner_2_weight)
                    .as_();

                // Check that structure not too close to another structure
                if structure_locations.iter().any(|(kind, loc, _door_dir)| {
                    structure_center.distance_squared(loc.xy())
                        < structure_kind.required_separation(kind).pow(2)
                }) {
                    None
                } else {
                    Some((
                        structure_center.with_z(rpos_height(structure_center)),
                        structure_kind,
                    ))
                }
            }) {
                let dir_to_center = Ori::from_vec2(hut_loc.xy()).opposite();
                let door_rng: u32 = rng.gen_range(0..9);
                let door_dir = match door_rng {
                    0..=3 => dir_to_center,
                    4..=5 => dir_to_center.cw(),
                    6..=7 => dir_to_center.ccw(),
                    // Should only be 8
                    _ => dir_to_center.opposite(),
                };
                structure_locations.push((kind, hut_loc, door_dir));
            }
        }

        let wall_connections = [
            outer_wall_corners[chieftain_indices[0]],
            outer_wall_corners[(chieftain_indices[1] + 1) % outer_wall_corners.len()],
        ];
        let inner_tower_locs = wall_connections
            .iter()
            .map(|corner_pos| *corner_pos / 3)
            .collect::<Vec<_>>();

        let chieftain_hut_loc = ((inner_tower_locs[0] + inner_tower_locs[1])
            + 2 * outer_wall_corners[chieftain_indices[1]])
            / 4;
        let chieftain_hut_ori = Ori::from_vec2(chieftain_hut_loc).opposite();
        structure_locations.push((
            GnarlingStructure::ChieftainHut,
            chieftain_hut_loc.with_z(rpos_height(chieftain_hut_loc)),
            chieftain_hut_ori,
        ));

        let watchtower_locs = {
            let (corner_1, corner_2) = (
                outer_wall_corners[gate_index],
                outer_wall_corners[(gate_index + 1) % outer_wall_corners.len()],
            );
            [
                corner_1 / 5 + corner_2 * 4 / 5,
                corner_1 * 4 / 5 + corner_2 / 5,
            ]
        };
        watchtower_locs.iter().for_each(|loc| {
            structure_locations.push((
                GnarlingStructure::WatchTower,
                loc.with_z(rpos_height(*loc)),
                Ori::North,
            ));
        });

        let wall_towers = outer_wall_corners
            .into_iter()
            .chain(inner_tower_locs.iter().copied())
            .map(|pos_2d| pos_2d.with_z(rpos_height(pos_2d)))
            .collect::<Vec<_>>();
        let wall_segments = outer_wall_segments
            .into_iter()
            .chain(
                wall_connections
                    .iter()
                    .copied()
                    .zip(inner_tower_locs.into_iter()),
            )
            .collect::<Vec<_>>();

        Self {
            name,
            seed,
            origin,
            radius,
            wall_radius,
            wall_towers,
            wall_segments,
            structure_locations,
            tunnels,
        }
    }

    pub fn name(&self) -> &str { &self.name }

    pub fn radius(&self) -> i32 { self.radius }

    pub fn apply_supplement<'a>(
        &'a self,
        // NOTE: Used only for dynamic elements like chests and entities!
        dynamic_rng: &mut impl Rng,
        wpos2d: Vec2<i32>,
        supplement: &mut ChunkSupplement,
    ) {
        let rpos = wpos2d - self.origin;
        let area = Aabr {
            min: rpos,
            max: rpos + TerrainChunkSize::RECT_SIZE.map(|e| e as i32),
        };

        for terminal in &self.tunnels.terminals {
            if area.contains_point(terminal.xy() - self.origin) {
                let chance = dynamic_rng.gen_range(0..10);
                match chance {
                    0..=4 => supplement
                        .add_entity(mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng)),
                    5 => {
                        supplement
                            .add_entity(mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng));
                        supplement
                            .add_entity(mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng));
                    },
                    6 => {
                        supplement.add_entity(deadwood(*terminal - 5 * Vec3::unit_z(), dynamic_rng))
                    },
                    7 => {
                        supplement
                            .add_entity(mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng));
                        supplement
                            .add_entity(deadwood(*terminal - 5 * Vec3::unit_z(), dynamic_rng));
                    },
                    8 => {
                        supplement
                            .add_entity(mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng));
                        supplement
                            .add_entity(mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng));
                        supplement
                            .add_entity(mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng));
                    },
                    _ => {},
                }
            }
        }
        if area.contains_point(self.tunnels.end.xy() - self.origin) {
            supplement.add_entity(harvester_boss(
                self.tunnels.end + Vec2::new(5, 5) - 12 * Vec3::unit_z(),
                dynamic_rng,
            ));
        }

        for (loc, pos, ori) in &self.structure_locations {
            let wpos = *pos + self.origin;
            if area.contains_point(pos.xy()) {
                match loc {
                    GnarlingStructure::Hut => {
                        supplement.add_entity(random_gnarling(wpos, dynamic_rng));
                    },
                    GnarlingStructure::Longhut => {
                        supplement.add_entity(random_gnarling(wpos, dynamic_rng));
                    },
                    GnarlingStructure::Banner => {
                        supplement.add_entity(random_gnarling(wpos, dynamic_rng));
                    },
                    GnarlingStructure::ChieftainHut => {
                        supplement.add_entity(gnarling_chieftain(wpos, dynamic_rng));
                        let left_inner_guard_pos = wpos + ori.dir() * 8 + ori.cw().dir() * 2;
                        supplement.add_entity(wood_golem(left_inner_guard_pos, dynamic_rng));
                        let right_inner_guard_pos = wpos + ori.dir() * 8 + ori.ccw().dir() * 2;
                        supplement.add_entity(wood_golem(right_inner_guard_pos, dynamic_rng));
                        let left_outer_guard_pos = wpos + ori.dir() * 16 + ori.cw().dir() * 2;
                        supplement.add_entity(random_gnarling(left_outer_guard_pos, dynamic_rng));
                        let right_outer_guard_pos = wpos + ori.dir() * 16 + ori.ccw().dir() * 2;
                        supplement.add_entity(random_gnarling(right_outer_guard_pos, dynamic_rng));
                    },
                    GnarlingStructure::WatchTower => {
                        supplement.add_entity(wood_golem(wpos, dynamic_rng));
                        let spawn_pos = wpos.xy().with_z(wpos.z + 27);
                        for _ in 0..4 {
                            supplement.add_entity(gnarling_stalker(
                                spawn_pos + Vec2::broadcast(4),
                                dynamic_rng,
                            ));
                        }
                    },
                    GnarlingStructure::Totem => {
                        let spawn_pos = wpos + pos.xy().map(|x| x.signum() * -5);
                        supplement.add_entity(wood_golem(spawn_pos, dynamic_rng));
                    },
                }
            }
        }

        for pos in &self.wall_towers {
            let wpos = *pos + self.origin;
            if area.contains_point(pos.xy()) {
                for _ in 0..4 {
                    supplement
                        .add_entity(gnarling_stalker(wpos.xy().with_z(wpos.z + 21), dynamic_rng))
                }
            }
        }
    }
}

impl Structure for GnarlingFortification {
    fn render(&self, _site: &Site, land: &Land, painter: &Painter) {
        // Create outer wall
        for (point, next_point) in self.wall_segments.iter() {
            // This adds additional points for the wall on the line between two points,
            // allowing the wall to better handle slopes
            const SECTIONS_PER_WALL_SEGMENT: usize = 3;

            (0..(SECTIONS_PER_WALL_SEGMENT as i32))
                .into_iter()
                .map(move |a| {
                    let get_point =
                        |a| point + (next_point - point) * a / (SECTIONS_PER_WALL_SEGMENT as i32);
                    (get_point(a), get_point(a + 1))
                })
                .for_each(|(point, next_point)| {
                    // 2d world positions of each point in wall segment
                    let start_wpos = point + self.origin;
                    let end_wpos = next_point + self.origin;

                    // Wall base
                    let wall_depth = 3.0;
                    let start = start_wpos
                        .as_()
                        .with_z(land.get_alt_approx(start_wpos) - wall_depth);
                    let end = end_wpos
                        .as_()
                        .with_z(land.get_alt_approx(end_wpos) - wall_depth);

                    let wall_base_thickness = 3.0;
                    let wall_base_height = 3.0;

                    painter
                        .segment_prism(
                            start,
                            end,
                            wall_base_thickness,
                            wall_base_height + wall_depth as f32,
                        )
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(55, 25, 8),
                        )));

                    // Middle of wall
                    let start = start_wpos.as_().with_z(land.get_alt_approx(start_wpos));
                    let end = end_wpos.as_().with_z(land.get_alt_approx(end_wpos));

                    let wall_mid_thickness = 1.0;
                    let wall_mid_height = 5.0 + wall_base_height;

                    painter
                        .segment_prism(start, end, wall_mid_thickness, wall_mid_height)
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(115, 58, 26),
                        )));

                    // Top of wall
                    let start = start_wpos
                        .as_()
                        .with_z(land.get_alt_approx(start_wpos) + wall_mid_height);
                    let end = end_wpos
                        .as_()
                        .with_z(land.get_alt_approx(end_wpos) + wall_mid_height);

                    let wall_top_thickness = 2.0;
                    let wall_top_height = 1.0;

                    painter
                        .segment_prism(start, end, wall_top_thickness, wall_top_height)
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(55, 25, 8),
                        )));

                    // Wall parapets
                    let parapet_z_offset = 1.0;

                    let start = Vec3::new(
                        point.x as f32 * (self.wall_radius as f32 + 1.0)
                            / (self.wall_radius as f32)
                            + self.origin.x as f32,
                        point.y as f32 * (self.wall_radius as f32 + 1.0)
                            / (self.wall_radius as f32)
                            + self.origin.y as f32,
                        land.get_alt_approx(start_wpos) + wall_mid_height + wall_top_height
                            - parapet_z_offset,
                    );
                    let end = Vec3::new(
                        next_point.x as f32 * (self.wall_radius as f32 + 1.0)
                            / (self.wall_radius as f32)
                            + self.origin.x as f32,
                        next_point.y as f32 * (self.wall_radius as f32 + 1.0)
                            / (self.wall_radius as f32)
                            + self.origin.y as f32,
                        land.get_alt_approx(end_wpos) + wall_mid_height + wall_top_height
                            - parapet_z_offset,
                    );

                    let wall_par_thickness = tweak!(0.8);
                    let wall_par_height = 1.0;

                    painter
                        .segment_prism(
                            start,
                            end,
                            wall_par_thickness,
                            wall_par_height + parapet_z_offset as f32,
                        )
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(55, 25, 8),
                        )));
                })
        }

        // Create towers
        self.wall_towers.iter().for_each(|point| {
            let wpos = point.xy() + self.origin;

            // Tower base
            let tower_depth = 3;
            let tower_base_pos = wpos.with_z(land.get_alt_approx(wpos) as i32 - tower_depth);
            let tower_radius = 5.;
            let tower_height = 20.0;

            painter
                .prim(Primitive::cylinder(
                    tower_base_pos,
                    tower_radius,
                    tower_depth as f32 + tower_height - 5.0,
                ))
                .fill(Fill::Block(Block::new(
                    BlockKind::Wood,
                    Rgb::new(55, 25, 8), //dark brown
                )));
            painter
                .prim(Primitive::cylinder(
                    wpos.with_z(
                        land.get_alt_approx(wpos) as i32 - tower_depth
                            + tower_depth as i32
                            + tower_height as i32
                            - 5,
                    ),
                    tower_radius,
                    5.0,
                ))
                .fill(Fill::Block(Block::new(
                    BlockKind::Wood,
                    Rgb::new(115, 58, 26), //light brown
                )));
            painter
                .prim(Primitive::cylinder(tower_base_pos, tower_radius + 1.0, 5.0))
                .fill(Fill::Block(Block::new(
                    BlockKind::Wood,
                    Rgb::new(22, 36, 20), //green
                )));
            let red_deco1 = painter.aabb(Aabb {
                min: Vec2::new(wpos.x - 5, wpos.y - 1)
                    .with_z(land.get_alt_approx(wpos) as i32 - tower_depth),
                max: Vec2::new(wpos.x + 5, wpos.y + 1)
                    .with_z(land.get_alt_approx(wpos) as i32 - tower_depth + 16),
            });
            let red_deco2 = painter.aabb(Aabb {
                min: Vec2::new(wpos.x - 1, wpos.y - 5)
                    .with_z(land.get_alt_approx(wpos) as i32 - tower_depth),
                max: Vec2::new(wpos.x + 1, wpos.y + 5)
                    .with_z(land.get_alt_approx(wpos) as i32 - tower_depth + 16),
            });
            let red_deco = red_deco1.union(red_deco2);
            red_deco.fill(Fill::Block(Block::new(
                BlockKind::Wood,
                Rgb::new(102, 31, 24), //red
            )));
            // Tower cylinder
            let tower_floor_pos = wpos.with_z(land.get_alt_approx(wpos) as i32);

            painter
                .prim(Primitive::cylinder(
                    tower_floor_pos,
                    tower_radius - 1.0,
                    tower_height,
                ))
                .fill(Fill::Block(Block::empty()));

            // Tower top floor
            let top_floor_z = (land.get_alt_approx(wpos) + tower_height - 2.0) as i32;
            let tower_top_floor_pos = wpos.with_z(top_floor_z);

            painter
                .prim(Primitive::cylinder(tower_top_floor_pos, tower_radius, 1.0))
                .fill(Fill::Block(Block::new(
                    BlockKind::Wood,
                    Rgb::new(55, 25, 8),
                )));

            // Tower roof poles
            let roof_pole_height = 5;
            let relative_pole_positions = [
                Vec2::new(-4, -4),
                Vec2::new(-4, 3),
                Vec2::new(3, -4),
                Vec2::new(3, 3),
            ];
            relative_pole_positions
                .iter()
                .map(|rpos| wpos + rpos)
                .for_each(|pole_pos| {
                    painter
                        .line(
                            pole_pos.with_z(top_floor_z),
                            pole_pos.with_z(top_floor_z + roof_pole_height),
                            1.,
                        )
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(55, 25, 8),
                        )));
                });

            // Tower roof
            let roof_sphere_radius = 10;
            let roof_radius = tower_radius + 1.0;
            let roof_height = 3;

            let roof_cyl = painter.prim(Primitive::cylinder(
                wpos.with_z(top_floor_z + roof_pole_height),
                roof_radius,
                roof_height as f32,
            ));

            painter
                .prim(Primitive::sphere(
                    wpos.with_z(top_floor_z + roof_pole_height + roof_height - roof_sphere_radius),
                    roof_sphere_radius as f32,
                ))
                .intersect(roof_cyl)
                .fill(Fill::Block(Block::new(
                    BlockKind::Leaves,
                    Rgb::new(22, 36, 20),
                )));
        });

        self.structure_locations
            .iter()
            .for_each(|(kind, loc, door_dir)| {
                let wpos = self.origin + loc.xy();
                let alt = land.get_alt_approx(wpos) as i32;

                fn generate_hut(
                    painter: &Painter,
                    wpos: Vec2<i32>,
                    alt: i32,
                    door_dir: Ori,
                    hut_radius: f32,
                    hut_wall_height: f32,
                    door_height: i32,
                    roof_height: f32,
                    roof_overhang: f32,
                ) {
                    // Floor
                    let base = wpos.with_z(alt);
                    painter
                        .prim(Primitive::cylinder(base, hut_radius + 1.0, 2.0))
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(55, 25, 8),
                        )));

                    // Wall
                    let floor_pos = wpos.with_z(alt + 1);
                    //alternating colors for ring pattern on ceiling
                    painter
                        .prim(Primitive::cylinder(
                            floor_pos,
                            hut_radius,
                            hut_wall_height + 3.0,
                        ))
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(71, 33, 11),
                        )));
                    painter
                        .prim(Primitive::cylinder(
                            floor_pos,
                            hut_radius - 1.0,
                            hut_wall_height + 3.0,
                        ))
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(55, 25, 8),
                        )));
                    painter
                        .prim(Primitive::cylinder(
                            floor_pos,
                            hut_radius - 2.0,
                            hut_wall_height + 3.0,
                        ))
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(71, 33, 11),
                        )));
                    painter
                        .prim(Primitive::cylinder(
                            floor_pos,
                            hut_radius - 3.0,
                            hut_wall_height + 3.0,
                        ))
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(55, 25, 8),
                        )));
                    painter
                        .prim(Primitive::cylinder(
                            floor_pos,
                            hut_radius - 1.0,
                            hut_wall_height,
                        ))
                        .fill(Fill::Block(Block::empty()));

                    // Door
                    let aabb_min = |dir| {
                        match dir {
                            Ori::North | Ori::East => wpos - Vec2::one(),
                            Ori::South | Ori::West => wpos + Vec2::one(),
                        }
                        .with_z(alt + 1)
                    };
                    let aabb_max = |dir| {
                        (match dir {
                            Ori::North | Ori::East => wpos + Vec2::one(),
                            Ori::South | Ori::West => wpos - Vec2::one(),
                        } + dir.dir() * hut_radius as i32)
                            .with_z(alt + 1 + door_height)
                    };

                    painter
                        .prim(Primitive::Aabb(
                            Aabb {
                                min: aabb_min(door_dir),
                                max: aabb_max(door_dir),
                            }
                            .made_valid(),
                        ))
                        .fill(Fill::Block(Block::empty()));

                    // Roof
                    let roof_radius = hut_radius + roof_overhang;
                    painter
                        .prim(Primitive::cone(
                            wpos.with_z(alt + 3 + hut_wall_height as i32),
                            roof_radius - 1.0,
                            roof_height - 1.0,
                        ))
                        .fill(Fill::Block(Block::new(
                            BlockKind::Leaves,
                            Rgb::new(22, 36, 20),
                        )));
                    //small bits handing from huts
                    let tendril1 = painter.line(
                        Vec2::new(wpos.x - 3, wpos.y - 5).with_z(alt + 1 + hut_wall_height as i32),
                        Vec2::new(wpos.x - 3, wpos.y - 5).with_z(alt + 3 + hut_wall_height as i32),
                        1.0,
                    );

                    let tendril2 = painter.line(
                        Vec2::new(wpos.x + 4, wpos.y + 2).with_z(alt + 2 + hut_wall_height as i32),
                        Vec2::new(wpos.x + 4, wpos.y + 2).with_z(alt + 3 + hut_wall_height as i32),
                        1.0,
                    );

                    let tendril3 = tendril2.translate(Vec3::new(-7, 2, 0));
                    let tendril4 = tendril1.translate(Vec3::new(7, 4, 0));
                    let tendrils = tendril1.union(tendril2).union(tendril3).union(tendril4);

                    tendrils.fill(Fill::Block(Block::new(
                        BlockKind::Leaves,
                        Rgb::new(22, 36, 20),
                    )));
                    //sphere to delete some hut
                    painter
                        .prim(Primitive::sphere(
                            Vec2::new(wpos.x + 1, wpos.y + 2)
                                .with_z(alt + 11 + hut_wall_height as i32),
                            8.5,
                        ))
                        .fill(Fill::Block(Block::empty()));
                }

                fn generate_longhut(
                    painter: &Painter,
                    wpos: Vec2<i32>,
                    alt: i32,
                    _door_dir: Ori,
                    hut_radius: f32,
                    _hut_wall_height: f32,
                    _door_height: i32,
                    roof_height: f32,
                    _roof_overhang: f32,
                ) {
                    // Floor
                    let raise = 5;
                    let base = wpos.with_z(alt);
                    painter
                        .prim(Primitive::cylinder(base, hut_radius + 1.0, 2.0))
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(55, 25, 8),
                        )));

                    let platform = painter.aabb(Aabb {
                        min: (wpos - 13).with_z(alt + raise),
                        max: (wpos + 13).with_z(alt + raise + 1),
                    });

                    painter.fill(
                        platform,
                        Fill::Block(Block::new(BlockKind::Wood, Rgb::new(55, 25, 8))),
                    );

                    let support_1 = painter.line(
                        (wpos - 12).with_z(alt - 3),
                        (wpos - 12).with_z(alt + raise),
                        2.0,
                    );
                    let support_alt = painter.line(
                        (wpos - 12).with_z(alt - 3),
                        (wpos - 12).with_z(alt + raise),
                        2.0,
                    );
                    let support_2 = support_1.translate(Vec3::new(0, 23, 0));
                    let support_3 = support_1.translate(Vec3::new(23, 0, 0));
                    let support_4 = support_1.translate(Vec3::new(23, 23, 0));
                    let support_5 = support_alt.translate(Vec3::new(0, 12, 0));
                    let support_6 = support_alt.translate(Vec3::new(12, 0, 0));
                    let support_7 = support_alt.translate(Vec3::new(12, 23, 0));
                    let support_8 = support_alt.translate(Vec3::new(23, 12, 0));
                    let supports = support_1
                        .union(support_2)
                        .union(support_3)
                        .union(support_4)
                        .union(support_5)
                        .union(support_6)
                        .union(support_7)
                        .union(support_8);

                    painter.fill(
                        supports,
                        Fill::Block(Block::new(BlockKind::Wood, Rgb::new(55, 25, 8))),
                    );
                    let height_1 = 6.0;
                    let height_2 = 8.0;
                    let rad_1 = 11.0;
                    let rad_2 = 8.0;

                    // Wall
                    let floor_pos = wpos.with_z(alt + 1 + raise);
                    painter
                        .prim(Primitive::cylinder(floor_pos, rad_1, height_1))
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(115, 58, 26),
                        )));
                    painter
                        .prim(Primitive::cylinder(floor_pos, rad_1 - 1.0, height_1))
                        .fill(Fill::Block(Block::empty()));

                    let floor2_pos = wpos.with_z(alt + 1 + raise + height_1 as i32);
                    painter
                        .prim(Primitive::cylinder(floor2_pos, rad_2, height_2))
                        .fill(Fill::Block(Block::new(
                            BlockKind::Wood,
                            Rgb::new(115, 58, 26),
                        )));

                    // Roof
                    let roof_radius = rad_1 + 4.0;
                    let roof1 = painter.prim(Primitive::cone(
                        wpos.with_z(alt + 1 + height_1 as i32 + raise),
                        roof_radius,
                        roof_height,
                    ));
                    roof1.fill(Fill::Block(Block::new(
                        BlockKind::Leaves,
                        Rgb::new(22, 36, 20),
                    )));
                    let roof_radius = rad_2 + 1.0;
                    painter
                        .prim(Primitive::cone(
                            wpos.with_z(alt + 1 + height_1 as i32 + height_2 as i32 + raise),
                            roof_radius,
                            roof_height,
                        ))
                        .fill(Fill::Block(Block::new(
                            BlockKind::Leaves,
                            Rgb::new(22, 36, 20),
                        )));
                    let roof_support_1 = painter.line(
                        (wpos + 1).with_z(alt + raise + height_1 as i32 - 3),
                        (wpos + 10).with_z(alt + raise + height_1 as i32),
                        1.5,
                    );
                    let roof_strut = painter.line(
                        (wpos + 1).with_z(alt + raise + height_1 as i32 + 2),
                        (wpos + 10).with_z(alt + raise + height_1 as i32 + 2),
                        1.0,
                    );
                    let wall2support = painter.line(
                        (wpos + rad_2 as i32 - 3).with_z(alt + raise + height_1 as i32 + 6),
                        (wpos + rad_2 as i32 - 3).with_z(alt + raise + height_1 as i32 + 8),
                        1.0,
                    );
                    let wall2roof = painter.line(
                        (wpos + rad_2 as i32 - 4).with_z(alt + raise + height_1 as i32 + 9),
                        (wpos + rad_2 as i32 - 3).with_z(alt + raise + height_1 as i32 + 9),
                        2.0,
                    );

                    let roof_support_1 = roof_support_1
                        .union(roof_strut)
                        .union(wall2support)
                        .union(wall2roof);

                    let roof_support_2 =
                        roof_support_1.rotate(Mat3::new(1, 0, 0, 0, -1, 0, 0, 0, 1));
                    let roof_support_3 =
                        roof_support_1.rotate(Mat3::new(-1, 0, 0, 0, 1, 0, 0, 0, 1));
                    let roof_support_4 =
                        roof_support_1.rotate(Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1));
                    let roof_support = roof_support_1
                        .union(roof_support_2)
                        .union(roof_support_3)
                        .union(roof_support_4);

                    painter.fill(
                        roof_support,
                        Fill::Block(Block::new(BlockKind::Wood, Rgb::new(55, 25, 8))),
                    );

                    let spot = painter.line(
                        (wpos + 1).with_z(alt + raise + height_1 as i32 + 2),
                        (wpos + 1).with_z(alt + raise + height_1 as i32 + 2),
                        1.0,
                    );
                    let spike_1 = painter.line(
                        (wpos + rad_2 as i32 - 1).with_z(alt + raise + height_1 as i32 + 2),
                        (wpos + rad_2 as i32 + 2).with_z(alt + raise + height_1 as i32 + 6),
                        1.0,
                    );

                    let spike_1 = spot.union(spike_1);

                    let spike_2 = spike_1.rotate(Mat3::new(1, 0, 0, 0, -1, 0, 0, 0, 1));
                    let spike_3 = spike_1.rotate(Mat3::new(-1, 0, 0, 0, 1, 0, 0, 0, 1));
                    let spike_4 = spike_1.rotate(Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1));
                    let spikes = spike_1.union(spike_2).union(spike_3).union(spike_4);

                    painter.fill(
                        spikes,
                        Fill::Block(Block::new(BlockKind::Wood, Rgb::new(184, 177, 134))),
                    );
                    //Open doorways (top floor)
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 2, wpos.y - 15)
                                .with_z(alt + raise + height_1 as i32 + 3),
                            max: Vec2::new(wpos.x + 2, wpos.y + 15)
                                .with_z(alt + raise + height_1 as i32 + 8),
                        })
                        .fill(Fill::Block(Block::empty()));
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 3, wpos.y - 15)
                                .with_z(alt + raise + height_1 as i32 + 4),
                            max: Vec2::new(wpos.x + 3, wpos.y + 15)
                                .with_z(alt + raise + height_1 as i32 + 7),
                        })
                        .fill(Fill::Block(Block::empty()));
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 15, wpos.y - 2)
                                .with_z(alt + raise + height_1 as i32 + 3),
                            max: Vec2::new(wpos.x + 15, wpos.y + 2)
                                .with_z(alt + raise + height_1 as i32 + 8),
                        })
                        .fill(Fill::Block(Block::empty()));
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 15, wpos.y - 3)
                                .with_z(alt + raise + height_1 as i32 + 4),
                            max: Vec2::new(wpos.x + 15, wpos.y + 3)
                                .with_z(alt + raise + height_1 as i32 + 7),
                        })
                        .fill(Fill::Block(Block::empty()));
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 15, wpos.y - 1)
                                .with_z(alt + raise + height_1 as i32 - 3),
                            max: Vec2::new(wpos.x + 15, wpos.y + 1)
                                .with_z(alt + raise + height_1 as i32 + 7),
                        })
                        .intersect(roof1)
                        .translate(Vec3::new(0, 0, -1))
                        .fill(Fill::Block(Block::new(
                            BlockKind::Leaves,
                            Rgb::new(55, 25, 8),
                        )));
                    //Roofing details
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 15, wpos.y - 1)
                                .with_z(alt + raise + height_1 as i32 - 3),
                            max: Vec2::new(wpos.x + 15, wpos.y + 1)
                                .with_z(alt + raise + height_1 as i32 + 7),
                        })
                        .intersect(roof1)
                        .fill(Fill::Block(Block::new(
                            BlockKind::Leaves,
                            Rgb::new(102, 31, 24),
                        )));
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 1, wpos.y - 15)
                                .with_z(alt + raise + height_1 as i32 - 3),
                            max: Vec2::new(wpos.x + 1, wpos.y + 15)
                                .with_z(alt + raise + height_1 as i32 + 7),
                        })
                        .intersect(roof1)
                        .translate(Vec3::new(0, 0, -1))
                        .fill(Fill::Block(Block::new(
                            BlockKind::Leaves,
                            Rgb::new(55, 25, 8),
                        )));
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 1, wpos.y - 15)
                                .with_z(alt + raise + height_1 as i32 - 3),
                            max: Vec2::new(wpos.x + 1, wpos.y + 15)
                                .with_z(alt + raise + height_1 as i32 + 7),
                        })
                        .intersect(roof1)
                        .fill(Fill::Block(Block::new(
                            BlockKind::Leaves,
                            Rgb::new(102, 31, 24),
                        )));
                    painter
                        .prim(Primitive::cylinder(floor2_pos, rad_2 - 1.0, height_2))
                        .fill(Fill::Block(Block::empty()));
                }

                match kind {
                    GnarlingStructure::Hut => {
                        let hut_radius = 5.0;
                        let hut_wall_height = 4.0;
                        let door_height = 3;
                        let roof_height = 3.0;
                        let roof_overhang = 1.0;

                        generate_hut(
                            painter,
                            wpos,
                            alt,
                            *door_dir,
                            hut_radius,
                            hut_wall_height,
                            door_height,
                            roof_height,
                            roof_overhang,
                        );
                    },
                    GnarlingStructure::Longhut => {
                        let hut_radius = 5.0;
                        let hut_wall_height = 15.0;
                        let door_height = 3;
                        let roof_height = 3.0;
                        let roof_overhang = 1.0;

                        generate_longhut(
                            painter,
                            wpos,
                            alt,
                            *door_dir,
                            hut_radius,
                            hut_wall_height,
                            door_height,
                            roof_height,
                            roof_overhang,
                        );
                    },
                    GnarlingStructure::Totem => {
                        let totem_pos = wpos.with_z(alt);

                        lazy_static! {
                            pub static ref TOTEM: AssetHandle<StructuresGroup> =
                                PrefabStructure::load_group("site_structures.gnarling.totem");
                        }

                        let totem = TOTEM.read();
                        let totem = totem[self.seed as usize % totem.len()].clone();

                        painter
                            .prim(Primitive::Prefab(Box::new(totem.clone())))
                            .translate(totem_pos)
                            .fill(Fill::Prefab(Box::new(totem), totem_pos, self.seed));
                    },
                    GnarlingStructure::ChieftainHut => {
                        let hut_radius = 12.0;
                        let hut_wall_height = 6.0;
                        let door_height = 3;
                        let roof_height = 7.0;
                        let roof_overhang = 2.0;

                        generate_hut(
                            painter,
                            wpos,
                            alt,
                            *door_dir,
                            hut_radius,
                            hut_wall_height,
                            door_height,
                            roof_height,
                            roof_overhang,
                        );

                        let hut_radius = 5.0;
                        let hut_wall_height = 14.0;
                        let door_height = 3;
                        let roof_height = 6.0;
                        let roof_overhang = 1.0;

                        generate_hut(
                            painter,
                            wpos,
                            alt,
                            *door_dir,
                            hut_radius,
                            hut_wall_height,
                            door_height,
                            roof_height,
                            roof_overhang,
                        );
                    },

                    GnarlingStructure::Banner => {
                        let flag = painter.aabb(Aabb {
                            min: Vec2::new(wpos.x + 1, wpos.y - 1).with_z(alt + 8),
                            max: Vec2::new(wpos.x + 8, wpos.y).with_z(alt + 38),
                        });
                        flag.fill(Fill::Block(Block::new(
                            BlockKind::Leaves,
                            Rgb::new(102, 31, 24),
                        )));
                        //add green streaks
                        let streak1 = painter
                            .line(
                                Vec2::new(wpos.x - 5, wpos.y - 1).with_z(alt + 20),
                                Vec2::new(wpos.x + 8, wpos.y - 1).with_z(alt + 33),
                                4.0,
                            )
                            .intersect(flag);

                        let streak2 = painter
                            .line(
                                Vec2::new(wpos.x - 5, wpos.y - 1).with_z(alt + 12),
                                Vec2::new(wpos.x + 8, wpos.y - 1).with_z(alt + 25),
                                1.5,
                            )
                            .intersect(flag);
                        let streak3 = painter
                            .line(
                                Vec2::new(wpos.x - 5, wpos.y - 1).with_z(alt + 8),
                                Vec2::new(wpos.x + 8, wpos.y - 1).with_z(alt + 21),
                                1.0,
                            )
                            .intersect(flag);
                        let streaks = streak1.union(streak2).union(streak3);
                        streaks.fill(Fill::Block(Block::new(
                            BlockKind::Leaves,
                            Rgb::new(22, 36, 20),
                        )));
                        //erase from top and bottom of rectangle flag for shape
                        painter
                            .line(
                                Vec2::new(wpos.x - 5, wpos.y - 1).with_z(alt + 31),
                                Vec2::new(wpos.x + 8, wpos.y - 1).with_z(alt + 44),
                                5.0,
                            )
                            .intersect(flag)
                            .fill(Fill::Block(Block::empty()));
                        painter
                            .prim(Primitive::sphere(
                                Vec2::new(wpos.x + 8, wpos.y).with_z(alt + 8),
                                6.0,
                            ))
                            .intersect(flag)
                            .fill(Fill::Block(Block::empty()));

                        //flagpole
                        let column = painter.aabb(Aabb {
                            min: (wpos - 1).with_z(alt - 3),
                            max: (wpos).with_z(alt + 30),
                        });

                        let arm = painter.line(
                            Vec2::new(wpos.x - 5, wpos.y - 1).with_z(alt + 26),
                            Vec2::new(wpos.x + 8, wpos.y - 1).with_z(alt + 39),
                            0.9,
                        );
                        let flagpole = column.union(arm);
                        flagpole.fill(Fill::Block(Block::new(
                            BlockKind::Leaves,
                            Rgb::new(55, 25, 8),
                        )));
                    },
                    GnarlingStructure::WatchTower => {
                        let platform_1_height = 14;
                        let platform_2_height = 20;
                        let platform_3_height = 26;
                        let roof_height = 30;

                        let platform_1 = painter.aabb(Aabb {
                            min: (wpos + 1).with_z(alt + platform_1_height),
                            max: (wpos + 10).with_z(alt + platform_1_height + 1),
                        });

                        let platform_2 = painter.aabb(Aabb {
                            min: (wpos + 1).with_z(alt + platform_2_height),
                            max: (wpos + 10).with_z(alt + platform_2_height + 1),
                        });

                        let platform_3 = painter.aabb(Aabb {
                            min: (wpos + 2).with_z(alt + platform_3_height),
                            max: (wpos + 9).with_z(alt + platform_3_height + 1),
                        });

                        let support_1 = painter.line(
                            wpos.with_z(alt),
                            (wpos + 2).with_z(alt + platform_1_height),
                            1.0,
                        );
                        let support_2 = support_1
                            .rotate(Mat3::new(1, 0, 0, 0, -1, 0, 0, 0, 1))
                            .translate(Vec3::new(0, 13, 0));
                        let support_3 = support_1
                            .rotate(Mat3::new(-1, 0, 0, 0, 1, 0, 0, 0, 1))
                            .translate(Vec3::new(13, 0, 0));
                        let support_4 = support_1
                            .rotate(Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1))
                            .translate(Vec3::new(13, 13, 0));

                        let supports = support_1.union(support_2).union(support_3).union(support_4);

                        let platform_1_supports = painter
                            .plane(
                                Aabr {
                                    min: (wpos + 2),
                                    max: (wpos + 9),
                                },
                                (wpos + 3).with_z(alt + platform_1_height + 1),
                                Vec2::new(0.0, 1.0),
                            )
                            .union(painter.plane(
                                Aabr {
                                    min: (wpos + 2),
                                    max: (wpos + 9),
                                },
                                (wpos + 3).with_z(alt + platform_2_height),
                                Vec2::new(0.0, -1.0),
                            ))
                            .without(
                                painter.aabb(Aabb {
                                    min: Vec2::new(wpos.x + 3, wpos.y + 2)
                                        .with_z(alt + platform_1_height),
                                    max: Vec2::new(wpos.x + 8, wpos.y + 9)
                                        .with_z(alt + platform_2_height),
                                }),
                            )
                            .without(
                                painter.aabb(Aabb {
                                    min: Vec2::new(wpos.x + 2, wpos.y + 2)
                                        .with_z(alt + platform_2_height),
                                    max: Vec2::new(wpos.x + 9, wpos.y + 9)
                                        .with_z(alt + platform_2_height + 2),
                                }),
                            )
                            .union(
                                painter.aabb(Aabb {
                                    min: Vec2::new(wpos.x + 2, wpos.y + 2)
                                        .with_z(alt + platform_1_height),
                                    max: Vec2::new(wpos.x + 9, wpos.y + 9)
                                        .with_z(alt + platform_2_height),
                                }),
                            )
                            .without(
                                painter.aabb(Aabb {
                                    min: Vec2::new(wpos.x + 3, wpos.y + 2)
                                        .with_z(alt + platform_1_height),
                                    max: Vec2::new(wpos.x + 8, wpos.y + 9)
                                        .with_z(alt + platform_2_height),
                                }),
                            )
                            .without(
                                painter.aabb(Aabb {
                                    min: Vec2::new(wpos.x + 2, wpos.y + 3)
                                        .with_z(alt + platform_1_height),
                                    max: Vec2::new(wpos.x + 9, wpos.y + 8)
                                        .with_z(alt + platform_2_height),
                                }),
                            );

                        let platform_2_supports = painter
                            .plane(
                                Aabr {
                                    min: (wpos + 3),
                                    max: (wpos + 8),
                                },
                                (wpos + 3).with_z(alt + platform_2_height + 1),
                                Vec2::new(1.0, 0.0),
                            )
                            .union(painter.plane(
                                Aabr {
                                    min: (wpos + 3),
                                    max: (wpos + 8),
                                },
                                (wpos + 3).with_z(alt + platform_3_height),
                                Vec2::new(-1.0, 0.0),
                            ))
                            .without(
                                painter.aabb(Aabb {
                                    min: Vec2::new(wpos.x + 3, wpos.y + 4)
                                        .with_z(alt + platform_2_height),
                                    max: Vec2::new(wpos.x + 8, wpos.y + 7)
                                        .with_z(alt + platform_3_height),
                                }),
                            )
                            .union(
                                painter.aabb(Aabb {
                                    min: Vec2::new(wpos.x + 3, wpos.y + 3)
                                        .with_z(alt + platform_2_height),
                                    max: Vec2::new(wpos.x + 8, wpos.y + 8)
                                        .with_z(alt + platform_3_height),
                                }),
                            )
                            .without(
                                painter.aabb(Aabb {
                                    min: Vec2::new(wpos.x + 4, wpos.y + 3)
                                        .with_z(alt + platform_2_height),
                                    max: Vec2::new(wpos.x + 7, wpos.y + 8)
                                        .with_z(alt + platform_3_height),
                                }),
                            )
                            .without(
                                painter.aabb(Aabb {
                                    min: Vec2::new(wpos.x + 3, wpos.y + 4)
                                        .with_z(alt + platform_2_height),
                                    max: Vec2::new(wpos.x + 8, wpos.y + 7)
                                        .with_z(alt + platform_3_height),
                                }),
                            );

                        let roof = painter
                            .gable(
                                Aabb {
                                    min: (wpos + 2).with_z(alt + roof_height),
                                    max: (wpos + 9).with_z(alt + roof_height + 4),
                                },
                                1,
                                Dir::Y,
                            )
                            .without(
                                painter.gable(
                                    Aabb {
                                        min: Vec2::new(wpos.x + 3, wpos.y + 2)
                                            .with_z(alt + roof_height),
                                        max: Vec2::new(wpos.x + 8, wpos.y + 9)
                                            .with_z(alt + roof_height + 3),
                                    },
                                    1,
                                    Dir::Y,
                                ),
                            );

                        let roof_pillars = painter
                            .aabb(Aabb {
                                min: Vec2::new(wpos.x + 3, wpos.y + 3)
                                    .with_z(alt + platform_3_height),
                                max: Vec2::new(wpos.x + 8, wpos.y + 8)
                                    .with_z(alt + roof_height + 1),
                            })
                            .without(
                                painter.aabb(Aabb {
                                    min: Vec2::new(wpos.x + 4, wpos.y + 3)
                                        .with_z(alt + platform_3_height),
                                    max: Vec2::new(wpos.x + 7, wpos.y + 8)
                                        .with_z(alt + roof_height),
                                }),
                            )
                            .without(
                                painter.aabb(Aabb {
                                    min: Vec2::new(wpos.x + 3, wpos.y + 4)
                                        .with_z(alt + platform_3_height),
                                    max: Vec2::new(wpos.x + 8, wpos.y + 7)
                                        .with_z(alt + roof_height),
                                }),
                            );
                        //skirt
                        let skirt1 = painter
                            .aabb(Aabb {
                                min: Vec2::new(wpos.x + 1, wpos.y)
                                    .with_z(alt + platform_1_height - 3),
                                max: Vec2::new(wpos.x + 4, wpos.y + 1)
                                    .with_z(alt + platform_1_height + 1),
                            })
                            .without(painter.line(
                                Vec2::new(wpos.x + 1, wpos.y).with_z(alt + platform_1_height - 1),
                                Vec2::new(wpos.x + 1, wpos.y).with_z(alt + platform_1_height - 3),
                                1.0,
                            ))
                            .without(painter.line(
                                Vec2::new(wpos.x + 3, wpos.y).with_z(alt + platform_1_height - 2),
                                Vec2::new(wpos.x + 3, wpos.y).with_z(alt + platform_1_height - 3),
                                1.0,
                            ));
                        let skirt2 = skirt1
                            .translate(Vec3::new(6, 0, 0))
                            .rotate(Mat3::new(-1, 0, 0, 0, 1, 0, 0, 0, 1));
                        let skirt3 = skirt2.translate(Vec3::new(3, 0, 0));

                        let skirtside1 = skirt1.union(skirt2).union(skirt3);
                        let skirtside2 = skirtside1
                            .rotate(Mat3::new(0, -1, 0, 1, 0, 0, 0, 0, 1))
                            .translate(Vec3::new(-1, 2, 0));

                        let skirtcorner1 = skirtside1.union(skirtside2);
                        let skirtcorner2 = skirtcorner1
                            .rotate(Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1))
                            .translate(Vec3::new(13, 11, 0));

                        let skirt1 = skirtcorner1.union(skirtcorner2);
                        let skirt2 = skirt1
                            .rotate(Mat3::new(1, 0, 0, 0, -1, 0, 0, 0, 1))
                            .translate(Vec3::new(0, 11, 6));

                        let skirt = skirt1.union(skirt2).union(roof);
                        painter.fill(
                            skirt,
                            Fill::Block(Block::new(BlockKind::Leaves, Rgb::new(22, 36, 20))),
                        );

                        let towerplatform = platform_1.union(platform_2).union(platform_3);

                        painter.fill(
                            towerplatform,
                            Fill::Block(Block::new(BlockKind::Wood, Rgb::new(115, 58, 26))),
                        );
                        let towervertical = supports
                            .union(platform_1_supports)
                            .union(platform_2_supports)
                            .union(roof_pillars);

                        painter.fill(
                            towervertical,
                            Fill::Block(Block::new(BlockKind::Wood, Rgb::new(55, 25, 8))),
                        );
                    },
                }
            });

        // Create tunnels beneath the fortification
        let wood = Fill::Block(Block::new(BlockKind::Wood, Rgb::new(55, 25, 8)));
        let alt = land.get_alt_approx(self.origin) as i32;
        let entrance = painter.cylinder(Aabb {
            min: Vec3::new(self.tunnels.start.x - 3, self.tunnels.start.y - 3, alt),
            max: Vec3::new(
                self.tunnels.start.x + 3 + 1,
                self.tunnels.start.y + 3 + 1,
                alt + 5,
            ),
        });
        let entrance_hollow = painter.cylinder(Aabb {
            min: Vec3::new(
                self.tunnels.start.x - 2,
                self.tunnels.start.y - 2,
                self.tunnels.start.z,
            ),
            max: Vec3::new(
                self.tunnels.start.x + 2 + 1,
                self.tunnels.start.y + 2 + 1,
                alt + 4,
            ),
        });
        let entrance_door = painter.aabb(Aabb {
            min: Vec3::new(self.tunnels.start.x - 1, self.tunnels.start.y, alt),
            max: Vec3::new(
                self.tunnels.start.x + 1 + 1,
                self.tunnels.start.y + 3 + 1,
                alt + 4,
            ),
        });

        let boss_room = painter
            .sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 5,
                    self.tunnels.end.y + 5,
                    self.tunnels.end.z,
                ) - 15,
                max: Vec3::new(
                    self.tunnels.end.x + 5,
                    self.tunnels.end.y + 5,
                    self.tunnels.end.z,
                ) + 15,
            })
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 10,
                    self.tunnels.end.y + 5,
                    self.tunnels.end.z,
                ) - 15,
                max: Vec3::new(
                    self.tunnels.end.x + 10,
                    self.tunnels.end.y + 5,
                    self.tunnels.end.z,
                ) + 15,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 5,
                    self.tunnels.end.y + 10,
                    self.tunnels.end.z,
                ) - 15,
                max: Vec3::new(
                    self.tunnels.end.x + 5,
                    self.tunnels.end.y + 10,
                    self.tunnels.end.z,
                ) + 15,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 10,
                    self.tunnels.end.y + 10,
                    self.tunnels.end.z,
                ) - 15,
                max: Vec3::new(
                    self.tunnels.end.x + 10,
                    self.tunnels.end.y + 10,
                    self.tunnels.end.z,
                ) + 15,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 15,
                    self.tunnels.end.y + 15,
                    self.tunnels.end.z,
                ) - 15,
                max: Vec3::new(
                    self.tunnels.end.x + 15,
                    self.tunnels.end.y + 15,
                    self.tunnels.end.z,
                ) + 15,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 15,
                    self.tunnels.end.y + 10,
                    self.tunnels.end.z,
                ) - 15,
                max: Vec3::new(
                    self.tunnels.end.x + 15,
                    self.tunnels.end.y + 10,
                    self.tunnels.end.z,
                ) + 15,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 10,
                    self.tunnels.end.y + 15,
                    self.tunnels.end.z,
                ) - 15,
                max: Vec3::new(
                    self.tunnels.end.x + 10,
                    self.tunnels.end.y + 15,
                    self.tunnels.end.z,
                ) + 15,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 5,
                    self.tunnels.end.y + 15,
                    self.tunnels.end.z,
                ) - 15,
                max: Vec3::new(
                    self.tunnels.end.x + 5,
                    self.tunnels.end.y + 15,
                    self.tunnels.end.z,
                ) + 15,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 15,
                    self.tunnels.end.y + 5,
                    self.tunnels.end.z,
                ) - 15,
                max: Vec3::new(
                    self.tunnels.end.x + 15,
                    self.tunnels.end.y + 5,
                    self.tunnels.end.z,
                ) + 15,
            }));

        let boss_room_clear = painter
            .sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 5,
                    self.tunnels.end.y + 5,
                    self.tunnels.end.z,
                ) - 14,
                max: Vec3::new(
                    self.tunnels.end.x + 5,
                    self.tunnels.end.y + 5,
                    self.tunnels.end.z,
                ) + 14,
            })
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 10,
                    self.tunnels.end.y + 5,
                    self.tunnels.end.z,
                ) - 14,
                max: Vec3::new(
                    self.tunnels.end.x + 10,
                    self.tunnels.end.y + 5,
                    self.tunnels.end.z,
                ) + 14,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 5,
                    self.tunnels.end.y + 10,
                    self.tunnels.end.z,
                ) - 14,
                max: Vec3::new(
                    self.tunnels.end.x + 5,
                    self.tunnels.end.y + 10,
                    self.tunnels.end.z,
                ) + 14,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 10,
                    self.tunnels.end.y + 10,
                    self.tunnels.end.z,
                ) - 14,
                max: Vec3::new(
                    self.tunnels.end.x + 10,
                    self.tunnels.end.y + 10,
                    self.tunnels.end.z,
                ) + 14,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 15,
                    self.tunnels.end.y + 15,
                    self.tunnels.end.z,
                ) - 14,
                max: Vec3::new(
                    self.tunnels.end.x + 15,
                    self.tunnels.end.y + 15,
                    self.tunnels.end.z,
                ) + 14,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 15,
                    self.tunnels.end.y + 10,
                    self.tunnels.end.z,
                ) - 14,
                max: Vec3::new(
                    self.tunnels.end.x + 15,
                    self.tunnels.end.y + 10,
                    self.tunnels.end.z,
                ) + 14,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 10,
                    self.tunnels.end.y + 15,
                    self.tunnels.end.z,
                ) - 14,
                max: Vec3::new(
                    self.tunnels.end.x + 10,
                    self.tunnels.end.y + 15,
                    self.tunnels.end.z,
                ) + 14,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 5,
                    self.tunnels.end.y + 15,
                    self.tunnels.end.z,
                ) - 14,
                max: Vec3::new(
                    self.tunnels.end.x + 5,
                    self.tunnels.end.y + 15,
                    self.tunnels.end.z,
                ) + 14,
            }))
            .union(painter.sphere(Aabb {
                min: Vec3::new(
                    self.tunnels.end.x + 15,
                    self.tunnels.end.y + 5,
                    self.tunnels.end.z,
                ) - 14,
                max: Vec3::new(
                    self.tunnels.end.x + 15,
                    self.tunnels.end.y + 5,
                    self.tunnels.end.z,
                ) + 14,
            }));

        let mut tunnels = painter.empty();
        let mut tunnels_clear = painter.empty();
        let mut ferns = painter.empty();
        let mut scatter = painter.empty();
        for branch in self.tunnels.branches.iter() {
            let tunnel_radius_i32 = 5 + branch.0.x % 4;
            let tunnel_radius = tunnel_radius_i32 as f32;
            let tunnel = painter.line(branch.0, branch.1, tunnel_radius);
            let tunnel_clear = painter.line(branch.0, branch.1, tunnel_radius - 1.0);
            let min_z = branch.0.z.min(branch.1.z);
            let max_z = branch.0.z.max(branch.1.z);
            for i in branch.0.x - tunnel_radius_i32..branch.1.x + tunnel_radius_i32 {
                for j in branch.0.y - tunnel_radius_i32..branch.1.y + tunnel_radius_i32 {
                    if i % 2 == 0 && j % 2 == 0 {
                        scatter = scatter.union(painter.aabb(Aabb {
                            min: Vec3::new(i, j, min_z),
                            max: Vec3::new(i + 1, j + 1, max_z),
                        }));
                    }
                }
            }
            let fern_clear = painter
                .line(
                    branch.0 + Vec3::unit_z(),
                    branch.1 + Vec3::unit_z(),
                    tunnel_radius - 1.0,
                )
                .union(painter.sphere(Aabb {
                    min: branch.0 - (tunnel_radius_i32 + 1),
                    max: branch.0 + (tunnel_radius_i32 + 1),
                }))
                .union(painter.sphere(Aabb {
                    min: branch.1 - (tunnel_radius_i32 + 1),
                    max: branch.1 + (tunnel_radius_i32 + 1),
                }));
            let fern = tunnel_clear.without(fern_clear).intersect(scatter);
            tunnels = tunnels.union(tunnel);
            tunnels_clear = tunnels_clear.union(tunnel_clear);
            ferns = ferns.union(fern);
        }

        let mut rooms = painter.empty();
        let mut rooms_clear = painter.empty();
        let mut chests_ori_0 = painter.empty();
        let mut chests_ori_2 = painter.empty();
        let mut chests_ori_4 = painter.empty();
        let mut fire_bowls = painter.empty();
        for terminal in self.tunnels.terminals.iter() {
            let room = painter.sphere(Aabb {
                min: terminal - 8,
                max: terminal + 8 + 1,
            });
            let room_clear = painter.sphere(Aabb {
                min: terminal - 7,
                max: terminal + 7 + 1,
            });
            rooms = rooms.union(room);
            rooms_clear = rooms_clear.union(room_clear);

            // FIRE!!!!!
            let fire_bowl = painter.aabb(Aabb {
                min: terminal.with_z(terminal.z - 7),
                max: terminal.with_z(terminal.z - 7) + 1,
            });
            fire_bowls = fire_bowls.union(fire_bowl);

            // Chest
            let chest_seed = terminal.x % 5;
            if chest_seed < 4 {
                let chest_pos = Vec3::new(terminal.x, terminal.y - 4, terminal.z - 6);
                let chest = painter.aabb(Aabb {
                    min: chest_pos,
                    max: chest_pos + 1,
                });
                chests_ori_4 = chests_ori_4.union(chest);
                if chest_seed < 2 {
                    let chest_pos = Vec3::new(terminal.x, terminal.y + 4, terminal.z - 6);
                    let chest = painter.aabb(Aabb {
                        min: chest_pos,
                        max: chest_pos + 1,
                    });
                    chests_ori_0 = chests_ori_0.union(chest);
                    if chest_seed < 1 {
                        let chest_pos = Vec3::new(terminal.x - 4, terminal.y, terminal.z - 6);
                        let chest = painter.aabb(Aabb {
                            min: chest_pos,
                            max: chest_pos + 1,
                        });
                        chests_ori_2 = chests_ori_2.union(chest);
                    }
                }
            }
        }
        entrance.fill(wood.clone());
        tunnels.fill(wood.clone());
        rooms.fill(wood.clone());
        boss_room.fill(wood);

        // Clear out insides after filling the walls in
        entrance_hollow.clear();
        entrance_door.clear();
        rooms_clear.clear();

        // Place room sprites after hollowing out
        chests_ori_0.fill(Fill::Block(
            Block::air(SpriteKind::DungeonChest0).with_ori(0).unwrap(),
        ));
        chests_ori_2.fill(Fill::Block(
            Block::air(SpriteKind::DungeonChest0).with_ori(2).unwrap(),
        ));
        chests_ori_4.fill(Fill::Block(
            Block::air(SpriteKind::DungeonChest0).with_ori(4).unwrap(),
        ));

        // Clear tunnels out after room sprites to prevent floating chests
        tunnels_clear.clear();

        // Place ferns in tunnels
        ferns.fill(Fill::Block(Block::air(SpriteKind::JungleFern)));

        // Place lights after ferns to ensure there is a light
        fire_bowls.fill(Fill::Block(Block::air(SpriteKind::FireBowlGround)));

        // Finally clear boss room
        boss_room_clear.clear();

        //painter.fill(
        //    scatter,
        //    Fill::Block(Block::new(BlockKind::Wood, Rgb::new(255, 0, 0))),
        //);
    }
}

fn gnarling_mugger<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.gnarling.mugger", rng)
}

fn gnarling_stalker<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.gnarling.stalker", rng)
}

fn gnarling_logger<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.gnarling.logger", rng)
}

fn random_gnarling<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    match rng.gen_range(0..3) {
        0 => gnarling_logger(pos, rng),
        1 => gnarling_mugger(pos, rng),
        _ => gnarling_stalker(pos, rng),
    }
}

fn gnarling_chieftain<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.gnarling.chieftain", rng)
}

fn deadwood<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.gnarling.deadwood", rng)
}

fn mandragora<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.gnarling.mandragora", rng)
}

fn wood_golem<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.gnarling.woodgolem", rng)
}

fn harvester_boss<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.gnarling.harvester", rng)
}

fn rrt<F>(
    start: Vec3<i32>,
    end: Vec3<i32>,
    is_valid_edge: F,
    rng: &mut impl Rng,
) -> Option<(Vec<(Vec3<i32>, Vec3<i32>)>, Vec<Vec3<i32>>)>
where
    F: Fn(Vec3<i32>, Vec3<i32>) -> bool,
{
    let mut nodes = Vec::new();
    let mut node_index: usize = 0;

    // HashMap<ChildNode, ParentNode>
    let mut parents = HashMap::new();

    let mut kdtree = KdTree::new();
    let start = start.map(|a| (a + 1) as f32);
    let end = end.map(|a| (a + 1) as f32);

    let min_x = start.x.min(end.x);
    let max_x = start.x.max(end.x);
    let min_y = start.y.min(end.y);
    let max_y = start.y.max(end.y);
    let min_z = start.z.min(end.z);
    let max_z = start.z.max(end.z);

    kdtree.add(&[start.x, start.y, start.z], node_index).ok()?;
    nodes.push(start);
    node_index += 1;
    let mut connect = false;

    for _i in 0..7000 {
        let radius: f32 = rng.gen_range(12.0..22.0);
        let radius_sqrd = radius.powi(2);
        if connect {
            break;
        }
        let sampled_point = Vec3::new(
            rng.gen_range(min_x - 20.0..max_x + 20.0),
            rng.gen_range(min_y - 20.0..max_y + 20.0),
            rng.gen_range(min_z - 5.0..max_z - 7.0),
        );
        let nearest_index = *kdtree
            .nearest_one(
                &[sampled_point.x, sampled_point.y, sampled_point.z],
                &squared_euclidean,
            )
            .ok()?
            .1 as usize;
        let nearest = nodes[nearest_index];
        let dist_sqrd = sampled_point.distance_squared(nearest);
        let new_point = if dist_sqrd > radius_sqrd {
            nearest + (sampled_point - nearest).normalized().map(|a| a * radius)
        } else {
            sampled_point
        };
        if is_valid_edge(
            nearest.map(|e| e.floor() as i32),
            new_point.map(|e| e.floor() as i32),
        ) {
            kdtree
                .add(&[new_point.x, new_point.y, new_point.z], node_index)
                .ok()?;
            nodes.push(new_point);
            parents.insert(node_index, nearest_index);
            node_index += 1;
        }
        if new_point.distance_squared(end) < radius.powi(2) {
            connect = true;
        }
    }

    let mut path = Vec::new();
    let nearest_index = *kdtree
        .nearest_one(&[end.x, end.y, end.z], &squared_euclidean)
        .ok()?
        .1 as usize;
    kdtree.add(&[end.x, end.y, end.z], node_index).ok()?;
    nodes.push(end);
    parents.insert(node_index, nearest_index);
    path.push(end);
    let mut current_node_index = node_index;
    while current_node_index > 0 {
        current_node_index = *parents.get(&current_node_index).unwrap();
        path.push(nodes[current_node_index]);
    }
    path.reverse();

    let mut terminals = Vec::new();
    let last = nodes.len() - 1;
    for (node_id, node_pos) in nodes.iter().enumerate() {
        if !parents.values().any(|e| e == &node_id) && node_id != 0 && node_id != last {
            terminals.push(node_pos.map(|e| e.floor() as i32));
        }
    }

    let branches = parents
        .iter()
        .map(|(a, b)| {
            (
                nodes[*a].map(|e| e.floor() as i32),
                nodes[*b].map(|e| e.floor() as i32),
            )
        })
        .collect::<Vec<(Vec3<i32>, Vec3<i32>)>>();

    Some((branches, terminals))
}
