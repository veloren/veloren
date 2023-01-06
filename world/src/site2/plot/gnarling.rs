use super::*;
use crate::{
    assets::AssetHandle,
    site2::{gen::PrimitiveTransform, util::Dir},
    util::{attempt, sampler::Sampler, RandomField},
    Land,
};
use common::{
    generation::{ChunkSupplement, EntityInfo},
    terrain::{Structure as PrefabStructure, StructuresGroup},
};
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
    structure_locations: Vec<(GnarlingStructure, Vec3<i32>, Dir)>,
    tunnels: Tunnels,
}

enum GnarlingStructure {
    Hut,
    VeloriteHut,
    Totem,
    ChieftainHut,
    WatchTower,
    Banner,
}

impl GnarlingStructure {
    fn required_separation(&self, other: &Self) -> i32 {
        let radius = |structure: &Self| match structure {
            Self::Hut => 7,
            Self::VeloriteHut => 15,
            Self::Banner => 6,
            Self::Totem => 6,
            // Generated in different pass that doesn't use distance check
            Self::WatchTower => 0,
            Self::ChieftainHut => 0,
        };

        let additional_padding = match (self, other) {
            (Self::Banner, Self::Banner) => 50,
            (Self::Totem, Self::Totem) => 50,
            (Self::VeloriteHut, Self::VeloriteHut) => 50,
            _ => 0,
        };

        radius(self) + radius(other) + additional_padding
    }
}

impl GnarlingFortification {
    pub fn generate(wpos: Vec2<i32>, land: &Land, rng: &mut impl Rng) -> Self {
        let rpos_height = |rpos| land.get_alt_approx(rpos + wpos) as i32;

        let name = NameGen::location(rng).generate_gnarling();
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
        let start = wpos.with_z(alt);
        let boss_room_shift = rng.gen_range(60..110);
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

        let tunnel_length_range = (12.0, 27.0);
        let tunnels =
            Tunnels::new(start, end, is_valid_edge, tunnel_length_range, rng).unwrap_or_default();

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
        let mut structure_locations = Vec::<(GnarlingStructure, Vec3<i32>, Dir)>::new();
        for _ in 0..desired_structures {
            if let Some((hut_loc, kind)) = attempt(50, || {
                // Choose structure kind
                let structure_kind = match rng.gen_range(0..10) {
                    0 => GnarlingStructure::Totem,
                    1..=3 => GnarlingStructure::VeloriteHut,
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

                let center_weight: f32 = rng.gen_range(0.15..0.7);

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

                // Check that structure not in the water or too close to another structure
                if land
                    .get_chunk_wpos(structure_center + origin)
                    .map_or(false, |c| c.is_underwater())
                    || structure_locations.iter().any(|(kind, loc, _door_dir)| {
                        structure_center.distance_squared(loc.xy())
                            < structure_kind.required_separation(kind).pow(2)
                    })
                {
                    None
                } else {
                    Some((
                        structure_center.with_z(rpos_height(structure_center)),
                        structure_kind,
                    ))
                }
            }) {
                let dir_to_center = Dir::from_vector(hut_loc.xy()).opposite();
                let door_rng: u32 = rng.gen_range(0..9);
                let door_dir = match door_rng {
                    0..=3 => dir_to_center,
                    4..=5 => dir_to_center.rotated_cw(),
                    6..=7 => dir_to_center.rotated_ccw(),
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
        let chieftain_hut_ori = Dir::from_vector(chieftain_hut_loc).opposite();
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
                Dir::Y,
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

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            trees: wpos.distance_squared(self.origin) > self.wall_radius.pow(2),
            waypoints: false,
            ..SpawnRules::default()
        }
    }

    // TODO: Find a better way of spawning entities in site2
    pub fn apply_supplement(
        &self,
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
                let entities = match chance {
                    0..=4 => vec![mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng)],
                    5 => vec![
                        mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng),
                        mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng),
                    ],
                    6 => vec![deadwood(*terminal - 5 * Vec3::unit_z(), dynamic_rng)],
                    7 => vec![
                        mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng),
                        deadwood(*terminal - 5 * Vec3::unit_z(), dynamic_rng),
                    ],
                    8 => vec![
                        mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng),
                        mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng),
                        mandragora(*terminal - 5 * Vec3::unit_z(), dynamic_rng),
                    ],
                    _ => Vec::new(),
                };
                for entity in entities {
                    supplement.add_entity(entity)
                }
            }
        }
        if area.contains_point(self.tunnels.end.xy() - self.origin) {
            let boss_room_offset = (self.tunnels.end.xy() - self.tunnels.start.xy())
                .map(|e| if e < 0 { -20 } else { 20 });
            supplement.add_entity(harvester_boss(
                self.tunnels.end + boss_room_offset - 2 * Vec3::unit_z(),
                dynamic_rng,
            ));
        }

        for (loc, pos, _ori) in &self.structure_locations {
            let wpos = *pos + self.origin;
            if area.contains_point(pos.xy()) {
                match loc {
                    GnarlingStructure::Hut => {
                        let num = dynamic_rng.gen_range(1..=3);
                        for _ in 0..num {
                            supplement.add_entity(random_gnarling(wpos, dynamic_rng));
                        }
                    },
                    GnarlingStructure::VeloriteHut => {
                        let num = dynamic_rng.gen_range(1..=4);
                        for _ in 0..num {
                            supplement.add_entity(random_gnarling(
                                wpos.xy().with_z(wpos.z + 12),
                                dynamic_rng,
                            ));
                        }
                    },
                    GnarlingStructure::Banner => {},
                    GnarlingStructure::ChieftainHut => {
                        let pos = wpos.xy().with_z(wpos.z + 8);
                        supplement.add_entity(gnarling_chieftain(pos, dynamic_rng));
                        for _ in 0..2 {
                            supplement.add_entity(wood_golem(pos, dynamic_rng));
                        }
                        for _ in 0..6 {
                            supplement.add_entity(random_gnarling(pos, dynamic_rng));
                        }
                    },
                    GnarlingStructure::WatchTower => {
                        supplement.add_entity(wood_golem(wpos, dynamic_rng));
                        let spawn_pos = wpos.xy().with_z(wpos.z + 27);
                        let num = dynamic_rng.gen_range(2..=4);
                        for _ in 0..num {
                            supplement.add_entity(gnarling_stalker(
                                spawn_pos + Vec2::broadcast(4),
                                dynamic_rng,
                            ));
                        }
                    },
                    GnarlingStructure::Totem => {},
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
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_gnarlingfortification\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_gnarlingfortification")]
    fn render_inner(&self, _site: &Site, land: &Land, painter: &Painter) {
        // Create outer wall
        for (point, next_point) in self.wall_segments.iter() {
            // This adds additional points for the wall on the line between two points,
            // allowing the wall to better handle slopes
            const SECTIONS_PER_WALL_SEGMENT: usize = 8;

            (0..(SECTIONS_PER_WALL_SEGMENT as i32))
                .into_iter()
                .map(move |a| {
                    let get_point =
                        |a| point + (next_point - point) * a / (SECTIONS_PER_WALL_SEGMENT as i32);
                    (get_point(a), get_point(a + 1))
                })
                .for_each(|(point, next_point)| {
                    // 2d world positions of each point in wall segment
                    let point = point;
                    let start_wpos = point + self.origin;
                    let end_wpos = next_point + self.origin;

                    let darkwood = Fill::Brick(BlockKind::Wood, Rgb::new(55, 25, 8), 12);
                    let lightwood = Fill::Brick(BlockKind::Wood, Rgb::new(71, 33, 11), 12);
                    let moss = Fill::Brick(BlockKind::Wood, Rgb::new(22, 36, 20), 24);

                    let start = (start_wpos + 2)
                        .as_()
                        .with_z(land.get_alt_approx(start_wpos) + 0.0);
                    let end = (end_wpos + 2)
                        .as_()
                        .with_z(land.get_alt_approx(end_wpos) + 0.0);
                    let randstart = start % 10.0 - 5.;
                    let randend = end % 10.0 - 5.0;
                    let mid = (start + end) / 2.0;
                    let startshift = Vec3::new(
                        randstart.x * 5.0,
                        randstart.y * 5.0,
                        randstart.z * 1.0 + 5.0,
                    );
                    let endshift =
                        Vec3::new(randend.x * 5.0, randend.y * 5.0, randend.z * 1.0 + 5.0);

                    let mossroot =
                        painter.cubic_bezier(start, mid + startshift, mid + endshift, end, 3.0);

                    let start = start_wpos
                        .as_()
                        .with_z(land.get_alt_approx(start_wpos) - 2.0);
                    let end = end_wpos.as_().with_z(land.get_alt_approx(end_wpos) - 2.0);
                    let randstart = start % 10.0 - 5.;
                    let randend = end % 10.0 - 5.0;
                    let mid = (start + end) / 2.0;
                    let startshift =
                        Vec3::new(randstart.x * 2.0, randstart.y * 2.0, randstart.z * 0.5);
                    let endshift = Vec3::new(randend.x * 2.0, randend.y * 2.0, randend.z * 0.5);

                    let root1 =
                        painter.cubic_bezier(start, mid + startshift, mid + endshift, end, 5.0);

                    let mosstop1 = root1.translate(Vec3::new(0, 0, 1));

                    root1.fill(darkwood.clone());

                    let start = (start_wpos + 3)
                        .as_()
                        .with_z(land.get_alt_approx(start_wpos) + 0.0);
                    let end = (end_wpos + 3)
                        .as_()
                        .with_z(land.get_alt_approx(end_wpos) + 0.0);
                    let randstart = start % 10.0 - 5.;
                    let randend = end % 10.0 - 5.0;
                    let mid = (start + end) / 2.0;
                    let startshift = Vec3::new(
                        randstart.x * 3.0,
                        randstart.y * 3.0,
                        randstart.z * 2.0 + 5.0,
                    );
                    let endshift =
                        Vec3::new(randend.x * 3.0, randend.y * 3.0, randend.z * 2.0 + 5.0);
                    let root2 =
                        painter.cubic_bezier(start, mid + startshift, mid + endshift, end, 2.0);

                    let mosstop2 = root2.translate(Vec3::new(0, 0, 1));
                    let start = start_wpos.as_().with_z(land.get_alt_approx(start_wpos));
                    let end = end_wpos.as_().with_z(land.get_alt_approx(end_wpos));

                    let wall_base_height = 3.0;
                    let wall_mid_thickness = 1.0;
                    let wall_mid_height = 7.0 + wall_base_height;

                    painter
                        .segment_prism(start, end, wall_mid_thickness, wall_mid_height)
                        .fill(darkwood);

                    let start = start_wpos
                        .as_()
                        .with_z(land.get_alt_approx(start_wpos) + wall_mid_height);
                    let end = end_wpos
                        .as_()
                        .with_z(land.get_alt_approx(end_wpos) + wall_mid_height);

                    let wall_top_thickness = 2.0;
                    let wall_top_height = 1.0;

                    let topwall =
                        painter.segment_prism(start, end, wall_top_thickness, wall_top_height);
                    let mosstopwall = topwall.translate(Vec3::new(0, 0, 1));

                    topwall.fill(lightwood.clone());

                    root2.fill(lightwood);

                    mosstopwall
                        .intersect(mossroot.translate(Vec3::new(0, 0, 8)))
                        .fill(moss.clone());

                    mosstop1.intersect(mossroot).fill(moss.clone());
                    mosstop2.intersect(mossroot).fill(moss);
                })
        }

        // Create towers
        self.wall_towers.iter().for_each(|point| {
            let wpos = point.xy() + self.origin;

            // Tower base
            let tower_depth = 3;
            let tower_base_pos = wpos.with_z(land.get_alt_approx(wpos) as i32 - tower_depth);
            let tower_radius = 5.0;
            let tower_height = 30.0;

            let randx = wpos.x.abs() % 10;
            let randy = wpos.y.abs() % 10;
            let randz = (land.get_alt_approx(wpos) as i32).abs() % 10;
            //layers of rings, starting at exterior
            let darkwood = Fill::Brick(BlockKind::Wood, Rgb::new(55, 25, 8), 12);
            let lightwood = Fill::Brick(BlockKind::Wood, Rgb::new(71, 33, 11), 12);
            let moss = Fill::Brick(BlockKind::Wood, Rgb::new(22, 36, 20), 24);
            let red = Fill::Brick(BlockKind::Wood, Rgb::new(102, 31, 2), 12);

            let twist = painter.cubic_bezier(
                tower_base_pos + Vec3::new(4, 4, 8),
                tower_base_pos + Vec3::new(-9, 9, 14),
                tower_base_pos + Vec3::new(-11, -11, 16),
                tower_base_pos + Vec3::new(4, -4, 21),
                1.5,
            );
            let mosstwist = twist.translate(Vec3::new(0, 0, 1));
            let mossarea = twist.translate(Vec3::new(1, 0, 1));

            mossarea
                .intersect(mosstwist)
                .without(twist)
                .fill(moss.clone());
            twist.fill(darkwood.clone());

            let outside = painter
                .cylinder_with_radius(
                    wpos.with_z(land.get_alt_approx(wpos) as i32),
                    tower_radius,
                    tower_height,
                )
                .without(painter.cylinder_with_radius(
                    wpos.with_z(land.get_alt_approx(wpos) as i32),
                    tower_radius - 1.0,
                    tower_height,
                ));
            outside.fill(lightwood.clone());
            painter
                .cylinder_with_radius(
                    wpos.with_z(land.get_alt_approx(wpos) as i32),
                    tower_radius - 1.0,
                    tower_height,
                )
                .fill(darkwood.clone());
            painter
                .cylinder_with_radius(
                    wpos.with_z(land.get_alt_approx(wpos) as i32),
                    tower_radius - 2.0,
                    tower_height,
                )
                .fill(lightwood.clone());
            painter
                .cylinder_with_radius(
                    wpos.with_z(land.get_alt_approx(wpos) as i32),
                    tower_radius - 3.0,
                    tower_height,
                )
                .fill(darkwood);
            painter
                .cylinder_with_radius(
                    wpos.with_z(land.get_alt_approx(wpos) as i32),
                    tower_radius - 4.0,
                    tower_height,
                )
                .fill(lightwood);
            //top layer, green above the tower
            painter
                .cylinder_with_radius(
                    wpos.with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32),
                    tower_radius,
                    2.0,
                )
                .fill(moss);
            //standing area one block deeper
            painter
                .cylinder_with_radius(
                    wpos.with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 9),
                    tower_radius - 1.0,
                    1.0,
                )
                .clear();
            //remove top sphere from trunk
            painter
                .sphere_with_radius(
                    Vec2::new(wpos.x - (randy - 5) / 2, wpos.y - (randz - 5) / 2).with_z(
                        land.get_alt_approx(wpos) as i32 + tower_height as i32 + 6 - randx / 3,
                    ),
                    5.5,
                )
                .clear();
            //remove bark from exterior layer
            painter
                .sphere_with_radius(
                    Vec2::new(wpos.x - (randy - 5) * 2, wpos.y - (randz - 5) * 2)
                        .with_z(land.get_alt_approx(wpos) as i32 + randx * 2),
                    7.5,
                )
                .intersect(outside)
                .clear();

            painter
                .sphere_with_radius(
                    Vec2::new(wpos.x - (randx - 5) * 2, wpos.y - (randy - 5) * 2)
                        .with_z(land.get_alt_approx(wpos) as i32 + randz * 2),
                    5.5,
                )
                .intersect(outside)
                .clear();

            //cut out standing room
            painter
                .aabb(Aabb {
                    min: Vec2::new(wpos.x - 3, wpos.y - 10)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 8),
                    max: Vec2::new(wpos.x + 3, wpos.y + 10)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 3),
                })
                .clear();
            painter
                .aabb(Aabb {
                    min: Vec2::new(wpos.x - 10, wpos.y - 3)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 8),
                    max: Vec2::new(wpos.x + 10, wpos.y + 3)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 3),
                })
                .clear();
            painter
                .aabb(Aabb {
                    min: Vec2::new(wpos.x - 2, wpos.y - 10)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 8),
                    max: Vec2::new(wpos.x + 2, wpos.y + 10)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 2),
                })
                .clear();
            painter
                .aabb(Aabb {
                    min: Vec2::new(wpos.x - 10, wpos.y - 2)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 8),
                    max: Vec2::new(wpos.x + 10, wpos.y + 2)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 2),
                })
                .clear();
            //flags
            painter
                .aabb(Aabb {
                    min: Vec2::new(wpos.x - 2, wpos.y - tower_radius as i32 - 1)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 16),
                    max: Vec2::new(wpos.x + 2, wpos.y + tower_radius as i32 + 1)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 10),
                })
                .intersect(outside)
                .fill(red.clone());
            painter
                .aabb(Aabb {
                    min: Vec2::new(wpos.x - tower_radius as i32 - 1, wpos.y - 2)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 16),
                    max: Vec2::new(wpos.x + tower_radius as i32 + 1, wpos.y + 2)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 10),
                })
                .intersect(outside)
                .fill(red.clone());
            painter
                .aabb(Aabb {
                    min: Vec2::new(wpos.x - 1, wpos.y - tower_radius as i32 - 1)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 17),
                    max: Vec2::new(wpos.x + 1, wpos.y + tower_radius as i32 + 1)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 16),
                })
                .intersect(outside)
                .fill(red.clone());
            painter
                .aabb(Aabb {
                    min: Vec2::new(wpos.x - tower_radius as i32 - 1, wpos.y - 1)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 17),
                    max: Vec2::new(wpos.x + tower_radius as i32 + 1, wpos.y + 1)
                        .with_z(land.get_alt_approx(wpos) as i32 + tower_height as i32 - 16),
                })
                .intersect(outside)
                .fill(red);
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
                    door_dir: Dir,
                    hut_radius: f32,
                    hut_wall_height: f32,
                    door_height: i32,
                    roof_height: f32,
                    roof_overhang: f32,
                ) {
                    let randx = wpos.x.abs() % 10;
                    let randy = wpos.y.abs() % 10;
                    let randz = alt.abs() % 10;
                    let hut_wall_height = hut_wall_height + randy as f32 * 1.5;

                    let darkwood = Fill::Brick(BlockKind::Wood, Rgb::new(55, 25, 8), 12);
                    let lightwood = Fill::Brick(BlockKind::Wood, Rgb::new(71, 33, 11), 12);
                    let moss = Fill::Brick(BlockKind::Leaves, Rgb::new(22, 36, 20), 24);

                    // Floor
                    let base = wpos.with_z(alt - 4);
                    painter
                        .cylinder_with_radius(base, hut_radius + 1.0, 6.0)
                        .fill(darkwood.clone());

                    // Wall
                    let floor_pos = wpos.with_z(alt + 1);
                    //alternating colors for ring pattern on ceiling
                    painter
                        .cylinder_with_radius(floor_pos, hut_radius, hut_wall_height + 3.0)
                        .fill(lightwood.clone());
                    painter
                        .cylinder_with_radius(floor_pos, hut_radius - 1.0, hut_wall_height + 3.0)
                        .fill(darkwood.clone());
                    painter
                        .cylinder_with_radius(floor_pos, hut_radius - 2.0, hut_wall_height + 3.0)
                        .fill(lightwood);
                    painter
                        .cylinder_with_radius(floor_pos, hut_radius - 3.0, hut_wall_height + 3.0)
                        .fill(darkwood);
                    painter
                        .cylinder_with_radius(floor_pos, hut_radius - 1.0, hut_wall_height)
                        .clear();

                    // Door
                    let aabb_min = |dir| {
                        match dir {
                            Dir::X | Dir::Y => wpos - Vec2::one(),
                            Dir::NegX | Dir::NegY => wpos + randx / 5 + 1,
                        }
                        .with_z(alt + 1)
                    };
                    let aabb_max = |dir| {
                        (match dir {
                            Dir::X | Dir::Y => wpos + randx / 5 + 1,
                            Dir::NegX | Dir::NegY => wpos - Vec2::one(),
                        } + dir.to_vec2() * hut_radius as i32)
                            .with_z(alt + 1 + door_height)
                    };

                    painter
                        .aabb(Aabb {
                            min: aabb_min(door_dir),
                            max: aabb_max(door_dir),
                        })
                        .clear();

                    // Roof
                    let roof_radius = hut_radius + roof_overhang;
                    painter
                        .cone_with_radius(
                            wpos.with_z(alt + 3 + hut_wall_height as i32),
                            roof_radius - 1.0,
                            roof_height - 1.0,
                        )
                        .fill(moss.clone());

                    //small bits hanging from huts
                    let tendril1 = painter.line(
                        Vec2::new(wpos.x - 3, wpos.y - 5)
                            .with_z(alt + (hut_wall_height * 0.75) as i32),
                        Vec2::new(wpos.x - 3, wpos.y - 5).with_z(alt + 3 + hut_wall_height as i32),
                        1.0,
                    );

                    let tendril2 = painter.line(
                        Vec2::new(wpos.x + 4, wpos.y + 2)
                            .with_z(alt + 1 + (hut_wall_height * 0.75) as i32),
                        Vec2::new(wpos.x + 4, wpos.y + 2).with_z(alt + 3 + hut_wall_height as i32),
                        1.0,
                    );

                    let tendril3 = tendril2.translate(Vec3::new(-7, 2, 0));
                    let tendril4 = tendril1.translate(Vec3::new(7, 4, 0));
                    let tendrils = tendril1.union(tendril2).union(tendril3).union(tendril4);

                    tendrils.fill(moss);

                    //sphere to delete some hut
                    painter
                        .sphere_with_radius(
                            Vec2::new(wpos.x - (randy - 5) / 2, wpos.y - (randz - 5) / 2)
                                .with_z(alt + 12 + hut_wall_height as i32 - randx / 3),
                            8.5,
                        )
                        .clear();
                }

                fn generate_chieftainhut(
                    painter: &Painter,
                    wpos: Vec2<i32>,
                    alt: i32,
                    roof_height: f32,
                ) {
                    let darkwood = Fill::Brick(BlockKind::Wood, Rgb::new(55, 25, 8), 12);
                    let lightwood = Fill::Brick(BlockKind::Wood, Rgb::new(71, 33, 11), 12);
                    let moss = Fill::Brick(BlockKind::Wood, Rgb::new(22, 36, 20), 24);
                    let red = Fill::Brick(BlockKind::Wood, Rgb::new(102, 31, 2), 12);

                    // Floor
                    let raise = 5;

                    let platform = painter.aabb(Aabb {
                        min: (wpos - 20).with_z(alt + raise),
                        max: (wpos + 20).with_z(alt + raise + 1),
                    });

                    painter.fill(platform, darkwood.clone());

                    let supports = painter
                        .line(
                            (wpos - 19).with_z(alt - 3),
                            (wpos - 19).with_z(alt + raise),
                            2.0,
                        )
                        .repeat(Vec3::new(37, 0, 0), 2)
                        .repeat(Vec3::new(0, 37, 0), 2);

                    let supports_inner = painter
                        .aabb(Aabb {
                            min: (wpos - 19).with_z(alt - 10) + Vec3::unit_y() * 17,
                            max: (wpos - 15).with_z(alt + raise) + Vec3::unit_y() * 17,
                        })
                        .repeat(Vec3::new(17, 17, 0), 2)
                        .repeat(Vec3::new(17, -17, 0), 2);
                    // let support_inner_2 = support_inner_1.translate(Vec3::new(34, 0, 0));
                    // let support_inner_3 = support_inner_1.translate(Vec3::new(17, 17, 0));
                    // let support_inner_4 = support_inner_1.translate(Vec3::new(17, -17, 0));
                    let supports = supports.union(supports_inner);

                    painter.fill(supports, darkwood.clone());
                    let height_1 = 10.0;
                    let height_2 = 12.0;
                    let rad_1 = 18.0;
                    let rad_2 = 15.0;

                    let floor_pos = wpos.with_z(alt + 1 + raise);
                    painter
                        .cylinder_with_radius(floor_pos, rad_1, height_1)
                        .fill(lightwood.clone());
                    painter
                        .cylinder_with_radius(floor_pos, rad_1 - 1.0, height_1)
                        .clear();

                    let floor2_pos = wpos.with_z(alt + 1 + raise + height_1 as i32);
                    painter
                        .cylinder_with_radius(floor2_pos, rad_2, height_2)
                        .fill(lightwood);

                    // Roof
                    let roof_radius = rad_1 + 5.0;
                    let roof1 = painter.cone_with_radius(
                        wpos.with_z(alt + 1 + height_1 as i32 + raise),
                        roof_radius,
                        roof_height,
                    );
                    roof1.fill(moss.clone());
                    let roof_radius = rad_2 + 1.0;
                    painter
                        .cone_with_radius(
                            wpos.with_z(alt + 1 + height_1 as i32 + height_2 as i32 + raise),
                            roof_radius,
                            roof_height,
                        )
                        .fill(moss);
                    let centerspot = painter.line(
                        (wpos + 1).with_z(alt + raise + height_1 as i32 + 2),
                        (wpos + 1).with_z(alt + raise + height_1 as i32 + 2),
                        1.0,
                    );
                    let roof_support_1 = painter.line(
                        (wpos + rad_1 as i32 - 7).with_z(alt + raise + height_1 as i32 - 2),
                        (wpos + rad_1 as i32 - 2).with_z(alt + raise + height_1 as i32),
                        1.5,
                    );
                    let roof_strut = painter.line(
                        (wpos + rad_1 as i32 - 7).with_z(alt + raise + height_1 as i32 + 2),
                        (wpos + rad_1 as i32 - 2).with_z(alt + raise + height_1 as i32 + 2),
                        1.0,
                    );
                    let wall2support = painter.line(
                        (wpos + rad_2 as i32 - 5).with_z(alt + raise + height_1 as i32 + 7),
                        (wpos + rad_2 as i32 - 5).with_z(alt + raise + height_1 as i32 + 12),
                        1.5,
                    );
                    let wall2roof = painter.line(
                        (wpos + rad_2 as i32 - 7).with_z(alt + raise + height_2 as i32 + 12),
                        (wpos + rad_2 as i32 - 4).with_z(alt + raise + height_2 as i32 + 10),
                        2.0,
                    );

                    let roof_support_1 = centerspot
                        .union(roof_support_1)
                        .union(roof_strut)
                        .union(wall2support)
                        .union(wall2roof);

                    let roof_support_2 =
                        roof_support_1.rotate_about_min(Mat3::new(1, 0, 0, 0, -1, 0, 0, 0, 1));
                    let roof_support_3 =
                        roof_support_1.rotate_about_min(Mat3::new(-1, 0, 0, 0, 1, 0, 0, 0, 1));
                    let roof_support_4 =
                        roof_support_1.rotate_about_min(Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1));
                    let roof_support = roof_support_1
                        .union(roof_support_2)
                        .union(roof_support_3)
                        .union(roof_support_4);

                    painter.fill(roof_support, red.clone());

                    let spike_high = painter.cubic_bezier(
                        (wpos + rad_2 as i32 - 5).with_z(alt + raise + height_1 as i32 + 2),
                        (wpos + rad_2 as i32 - 2).with_z(alt + raise + height_1 as i32 + 2),
                        (wpos + rad_2 as i32 - 1).with_z(alt + raise + height_1 as i32 + 5),
                        (wpos + rad_2 as i32).with_z(alt + raise + height_1 as i32 + 8),
                        1.3,
                    );
                    let spike_low =
                        spike_high.rotate_about_min(Mat3::new(1, 0, 0, 0, 1, 0, 0, 0, -1));
                    let spike_1 = centerspot.union(spike_low).union(spike_high);

                    let spike_2 = spike_1.rotate_about_min(Mat3::new(1, 0, 0, 0, -1, 0, 0, 0, 1));
                    let spike_3 = spike_1.rotate_about_min(Mat3::new(-1, 0, 0, 0, 1, 0, 0, 0, 1));
                    let spike_4 = spike_1.rotate_about_min(Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1));
                    let spikes = spike_1.union(spike_2).union(spike_3).union(spike_4);

                    painter.fill(
                        spikes,
                        Fill::Brick(BlockKind::Wood, Rgb::new(112, 110, 99), 24),
                    );
                    //Open doorways (top floor)
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 2, wpos.y - 15)
                                .with_z(alt + raise + height_1 as i32 + 3),
                            max: Vec2::new(wpos.x + 2, wpos.y + 15)
                                .with_z(alt + raise + height_1 as i32 + height_2 as i32),
                        })
                        .clear();
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 3, wpos.y - 15)
                                .with_z(alt + raise + height_1 as i32 + 4),
                            max: Vec2::new(wpos.x + 3, wpos.y + 15)
                                .with_z(alt + raise + height_1 as i32 + 10),
                        })
                        .clear();
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 15, wpos.y - 2)
                                .with_z(alt + raise + height_1 as i32 + 3),
                            max: Vec2::new(wpos.x + 15, wpos.y + 2)
                                .with_z(alt + raise + height_1 as i32 + height_2 as i32),
                        })
                        .clear();
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 15, wpos.y - 3)
                                .with_z(alt + raise + height_1 as i32 + 4),
                            max: Vec2::new(wpos.x + 15, wpos.y + 3)
                                .with_z(alt + raise + height_1 as i32 + 10),
                        })
                        .clear();

                    //
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 18, wpos.y - 2)
                                .with_z(alt + raise + height_1 as i32 - 9),
                            max: Vec2::new(wpos.x + 18, wpos.y + 2)
                                .with_z(alt + raise + height_1 as i32 - 1),
                        })
                        .clear();
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 2, wpos.y - 18)
                                .with_z(alt + raise + height_1 as i32 - 9),
                            max: Vec2::new(wpos.x + 2, wpos.y + 18)
                                .with_z(alt + raise + height_1 as i32 - 1),
                        })
                        .clear();
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 18, wpos.y - 3)
                                .with_z(alt + raise + height_1 as i32 - 9),
                            max: Vec2::new(wpos.x + 18, wpos.y + 3)
                                .with_z(alt + raise + height_1 as i32 - 3),
                        })
                        .clear();
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 3, wpos.y - 18)
                                .with_z(alt + raise + height_1 as i32 - 9),
                            max: Vec2::new(wpos.x + 3, wpos.y + 18)
                                .with_z(alt + raise + height_1 as i32 - 3),
                        })
                        .clear();
                    //Roofing details

                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 23, wpos.y - 2)
                                .with_z(alt + raise + height_1 as i32 - 3),
                            max: Vec2::new(wpos.x + 23, wpos.y + 2)
                                .with_z(alt + raise + height_1 as i32 + 7),
                        })
                        .intersect(roof1)
                        .translate(Vec3::new(0, 0, -1))
                        .fill(darkwood.clone());
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 23, wpos.y - 2)
                                .with_z(alt + raise + height_1 as i32 - 3),
                            max: Vec2::new(wpos.x + 23, wpos.y + 2)
                                .with_z(alt + raise + height_1 as i32 + 7),
                        })
                        .intersect(roof1)
                        .fill(red.clone());
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 2, wpos.y - 23)
                                .with_z(alt + raise + height_1 as i32 - 3),
                            max: Vec2::new(wpos.x + 2, wpos.y + 23)
                                .with_z(alt + raise + height_1 as i32 + 7),
                        })
                        .intersect(roof1)
                        .translate(Vec3::new(0, 0, -1))
                        .fill(darkwood);
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 2, wpos.y - 23)
                                .with_z(alt + raise + height_1 as i32 - 3),
                            max: Vec2::new(wpos.x + 2, wpos.y + 23)
                                .with_z(alt + raise + height_1 as i32 + 7),
                        })
                        .intersect(roof1)
                        .fill(red);
                    painter
                        .cylinder_with_radius(floor2_pos, rad_2 - 1.0, height_2)
                        .clear();
                }

                match kind {
                    GnarlingStructure::Hut => {
                        let hut_radius = 5.0;
                        let hut_wall_height = 4.0;
                        let door_height = 4;
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
                    GnarlingStructure::VeloriteHut => {
                        let rand = Vec3::new(
                            wpos.x.abs() % 10,
                            wpos.y.abs() % 10,
                            (land.get_alt_approx(wpos) as i32).abs() % 10,
                        );

                        let length = 14;
                        let width = 6;
                        let height = alt + 12;
                        let darkwood = Fill::Brick(BlockKind::Wood, Rgb::new(55, 25, 8), 12);
                        let lightwood = Fill::Brick(BlockKind::Wood, Rgb::new(71, 33, 11), 12);
                        let moss = Fill::Brick(BlockKind::Wood, Rgb::new(22, 36, 20), 24);
                        let red = Fill::Brick(BlockKind::Wood, Rgb::new(102, 31, 2), 12);

                        let low1 = painter.aabb(Aabb {
                            min: Vec2::new(wpos.x - width, wpos.y - length).with_z(height + 1),
                            max: Vec2::new(wpos.x + width, wpos.y + length).with_z(height + 2),
                        });

                        let low2 = painter.aabb(Aabb {
                            min: Vec2::new(wpos.x - length, wpos.y - width).with_z(height + 1),
                            max: Vec2::new(wpos.x + length, wpos.y + width).with_z(height + 2),
                        });
                        let top1 = painter.aabb(Aabb {
                            min: Vec2::new(wpos.x - width + 1, wpos.y - length + 1)
                                .with_z(height + 2),
                            max: Vec2::new(wpos.x + width - 1, wpos.y + length - 1)
                                .with_z(height + 2 + 1),
                        });

                        let top2 = painter.aabb(Aabb {
                            min: Vec2::new(wpos.x - length + 1, wpos.y - width + 1)
                                .with_z(height + 2),
                            max: Vec2::new(wpos.x + length - 1, wpos.y + width - 1)
                                .with_z(height + 2 + 1),
                        });
                        let low = low1.union(low2);
                        let top = top1.union(top2);

                        let roof = low1.union(low2).union(top1).union(top2);
                        let roofmoss = roof.translate(Vec3::new(0, 0, 1)).without(top).without(low);
                        top.fill(darkwood.clone());
                        low.fill(lightwood);
                        roofmoss.fill(moss);
                        painter
                            .sphere_with_radius(
                                Vec2::new(wpos.x + rand.y - 5, wpos.y + rand.z - 5)
                                    .with_z(height + 4),
                                7.0,
                            )
                            .intersect(roofmoss)
                            .clear();
                        painter
                            .sphere_with_radius(
                                Vec2::new(wpos.x + rand.x - 5, wpos.y + rand.y - 5)
                                    .with_z(height + 4),
                                4.0,
                            )
                            .intersect(roofmoss)
                            .clear();
                        painter
                            .sphere_with_radius(
                                Vec2::new(wpos.x + 2 * (rand.z - 5), wpos.y + 2 * (rand.x - 5))
                                    .with_z(height + 4),
                                4.0,
                            )
                            .intersect(roofmoss)
                            .clear();

                        //inside leg
                        let leg1 = painter.aabb(Aabb {
                            min: Vec2::new(wpos.x - width, wpos.y - width).with_z(height - 12),
                            max: Vec2::new(wpos.x - width + 2, wpos.y - width + 2)
                                .with_z(height + 2),
                        });
                        let legsupport1 = painter.line(
                            Vec2::new(wpos.x - width, wpos.y - width).with_z(height - 6),
                            Vec2::new(wpos.x - width + 3, wpos.y - width + 3).with_z(height),
                            1.0,
                        );

                        let leg2 = leg1.translate(Vec3::new(0, width * 2 - 2, 0));
                        let leg3 = leg1.translate(Vec3::new(width * 2 - 2, 0, 0));
                        let leg4 = leg1.translate(Vec3::new(width * 2 - 2, width * 2 - 2, 0));
                        let legsupport2 = legsupport1
                            .rotate_about_min(Mat3::new(0, 1, 0, -1, 0, 0, 0, 0, 1))
                            .translate(Vec3::new(1, width * 2 + 1, 0));
                        let legsupport3 = legsupport1
                            .rotate_about_min(Mat3::new(0, -1, 0, 1, 0, 0, 0, 0, 1))
                            .translate(Vec3::new(width * 2 + 1, 1, 0));
                        let legsupport4 = legsupport1
                            .rotate_about_min(Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1))
                            .translate(Vec3::new(width * 2 + 2, width * 2 + 2, 0));

                        let legsupports = legsupport1
                            .union(legsupport2)
                            .union(legsupport3)
                            .union(legsupport4);

                        let legs = leg1.union(leg2).union(leg3).union(leg4);
                        legs.fill(darkwood);
                        legsupports.without(legs).fill(red);

                        let spike1 = painter.line(
                            Vec2::new(wpos.x - 12, wpos.y + 2).with_z(height + 3),
                            Vec2::new(wpos.x - 7, wpos.y + 2).with_z(height + 5),
                            1.0,
                        );
                        let spike2 = painter.line(
                            Vec2::new(wpos.x - 12, wpos.y - 3).with_z(height + 3),
                            Vec2::new(wpos.x - 7, wpos.y - 3).with_z(height + 5),
                            1.0,
                        );
                        let spikes = spike1.union(spike2);
                        let spikesalt = spikes
                            .rotate_about_min(Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1))
                            .translate(Vec3::new(26, 8, 0));
                        let spikeshalf = spikes.union(spikesalt);
                        let spikesotherhalf = spikeshalf
                            .rotate_about_min(Mat3::new(0, -1, 0, 1, 0, 0, 0, 0, 1))
                            .translate(Vec3::new(16, -9, 0));
                        let spikesall = spikeshalf.union(spikesotherhalf);

                        spikesall.fill(Fill::Brick(BlockKind::Wood, Rgb::new(112, 110, 99), 24));
                        let crystal1 = painter.aabb(Aabb {
                            min: Vec2::new(wpos.x - 2, wpos.y - 3).with_z(alt),
                            max: Vec2::new(wpos.x + 2, wpos.y + 1).with_z(alt + 7),
                        });
                        let crystal2 = painter.aabb(Aabb {
                            min: Vec2::new(wpos.x - 3, wpos.y - 3).with_z(alt),
                            max: Vec2::new(wpos.x + 3, wpos.y + 1).with_z(alt + 6),
                        });
                        let crystal3 = painter.aabb(Aabb {
                            min: Vec2::new(wpos.x - 2, wpos.y - 2).with_z(alt),
                            max: Vec2::new(wpos.x + 4, wpos.y + 3).with_z(alt + 4),
                        });
                        let crystal4 = painter.aabb(Aabb {
                            min: Vec2::new(wpos.x - 1, wpos.y - 4).with_z(alt),
                            max: Vec2::new(wpos.x + 2, wpos.y + 2).with_z(alt + 8),
                        });
                        let crystalp1 = crystal1.union(crystal3);
                        let crystalp2 = crystal2.union(crystal4);

                        crystalp1.fill(Fill::Brick(
                            BlockKind::GlowingRock,
                            Rgb::new(50, 225, 210),
                            24,
                        ));
                        crystalp2.fill(Fill::Brick(
                            BlockKind::GlowingRock,
                            Rgb::new(36, 187, 151),
                            24,
                        ));
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
                        let roof_height = 3.0;

                        generate_chieftainhut(painter, wpos, alt, roof_height);
                    },

                    GnarlingStructure::Banner => {
                        let rand = Vec3::new(
                            wpos.x.abs() % 10,
                            wpos.y.abs() % 10,
                            (land.get_alt_approx(wpos) as i32).abs() % 10,
                        );

                        let darkwood = Fill::Brick(BlockKind::Wood, Rgb::new(55, 25, 8), 12);
                        let moss = Fill::Brick(BlockKind::Leaves, Rgb::new(22, 36, 20), 24);
                        let red = Fill::Brick(BlockKind::Wood, Rgb::new(102, 31, 2), 12);
                        let flag = painter.aabb(Aabb {
                            min: Vec2::new(wpos.x + 1, wpos.y - 1).with_z(alt + 8),
                            max: Vec2::new(wpos.x + 8, wpos.y).with_z(alt + 38),
                        });
                        flag.fill(red);
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
                        streaks.fill(moss);
                        //erase from top and bottom of rectangle flag for shape
                        painter
                            .line(
                                Vec2::new(wpos.x - 5, wpos.y - 1).with_z(alt + 31),
                                Vec2::new(wpos.x + 8, wpos.y - 1).with_z(alt + 44),
                                5.0,
                            )
                            .intersect(flag)
                            .clear();
                        painter
                            .sphere_with_radius(Vec2::new(wpos.x + 8, wpos.y).with_z(alt + 8), 6.0)
                            .intersect(flag)
                            .clear();
                        //tatters
                        painter
                            .line(
                                Vec2::new(wpos.x + 3 + rand.x / 5, wpos.y - 1)
                                    .with_z(alt + 15 + rand.y),
                                Vec2::new(wpos.x + 3 + rand.y / 5, wpos.y - 1).with_z(alt + 5),
                                0.9 * rand.z as f32 / 4.0,
                            )
                            .clear();
                        painter
                            .line(
                                Vec2::new(wpos.x + 4 + rand.z / 2, wpos.y - 1)
                                    .with_z(alt + 20 + rand.x),
                                Vec2::new(wpos.x + 4 + rand.z / 2, wpos.y - 1)
                                    .with_z(alt + 17 + rand.y),
                                0.9 * rand.z as f32 / 6.0,
                            )
                            .clear();

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
                        flagpole.fill(darkwood);
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
                            .rotate_about_min(Mat3::new(1, 0, 0, 0, -1, 0, 0, 0, 1))
                            .translate(Vec3::new(0, 13, 0));
                        let support_3 = support_1
                            .rotate_about_min(Mat3::new(-1, 0, 0, 0, 1, 0, 0, 0, 1))
                            .translate(Vec3::new(13, 0, 0));
                        let support_4 = support_1
                            .rotate_about_min(Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1))
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
                                0,
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
                                    0,
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
                            .rotate_about_min(Mat3::new(-1, 0, 0, 0, 1, 0, 0, 0, 1));
                        let skirt3 = skirt2.translate(Vec3::new(3, 0, 0));

                        let skirtside1 = skirt1.union(skirt2).union(skirt3);
                        let skirtside2 = skirtside1
                            .rotate_about_min(Mat3::new(0, -1, 0, 1, 0, 0, 0, 0, 1))
                            .translate(Vec3::new(0, 1, 0));

                        let skirtcorner1 = skirtside1.union(skirtside2);
                        let skirtcorner2 = skirtcorner1
                            .rotate_about_min(Mat3::new(-1, 0, 0, 0, -1, 0, 0, 0, 1))
                            .translate(Vec3::new(11, 11, 0));

                        let skirt1 = skirtcorner1.union(skirtcorner2);
                        let skirt2 = skirt1
                            .rotate_about_min(Mat3::new(1, 0, 0, 0, -1, 0, 0, 0, 1))
                            .translate(Vec3::new(0, 11, 6));

                        let skirt = skirt1.union(skirt2).union(roof);
                        painter.fill(
                            skirt,
                            Fill::Brick(BlockKind::Leaves, Rgb::new(22, 36, 20), 24),
                        );

                        let towerplatform = platform_1.union(platform_2).union(platform_3);

                        painter.fill(
                            towerplatform,
                            Fill::Brick(BlockKind::Wood, Rgb::new(71, 33, 11), 24),
                        );
                        let towervertical = supports
                            .union(platform_1_supports)
                            .union(platform_2_supports)
                            .union(roof_pillars);

                        painter.fill(
                            towervertical,
                            Fill::Brick(BlockKind::Wood, Rgb::new(55, 25, 8), 24),
                        );
                    },
                }
            });

        // Create tunnels beneath the fortification
        let wood = Fill::Brick(BlockKind::Wood, Rgb::new(55, 25, 8), 24);
        let dirt = Fill::Brick(BlockKind::Earth, Rgb::new(55, 25, 8), 24);
        let alt = land.get_alt_approx(self.origin) as i32;
        let stump = painter
            .cylinder(Aabb {
                min: (self.tunnels.start.xy() - 10).with_z(alt - 15),
                max: (self.tunnels.start.xy() + 11).with_z(alt + 10),
            })
            .union(painter.cylinder(Aabb {
                min: (self.tunnels.start.xy() - 11).with_z(alt),
                max: (self.tunnels.start.xy() + 12).with_z(alt + 2),
            }))
            .union(painter.line(
                self.tunnels.start.xy().with_z(alt + 10),
                (self.tunnels.start.xy() + 15).with_z(alt - 8),
                5.0,
            ))
            .union(painter.line(
                self.tunnels.start.xy().with_z(alt + 10),
                Vec2::new(self.tunnels.start.x - 15, self.tunnels.start.y + 15).with_z(alt - 8),
                5.0,
            ))
            .union(painter.line(
                self.tunnels.start.xy().with_z(alt + 10),
                Vec2::new(self.tunnels.start.x + 15, self.tunnels.start.y - 15).with_z(alt - 8),
                5.0,
            ))
            .union(painter.line(
                self.tunnels.start.xy().with_z(alt + 10),
                (self.tunnels.start.xy() - 15).with_z(alt - 8),
                5.0,
            ))
            .without(
                painter.sphere_with_radius((self.tunnels.start.xy() + 10).with_z(alt + 26), 18.0),
            )
            .without(
                painter.sphere_with_radius((self.tunnels.start.xy() - 10).with_z(alt + 26), 18.0),
            );
        let entrance_hollow = painter.line(
            self.tunnels.start,
            self.tunnels.start.xy().with_z(alt + 10),
            9.0,
        );

        let boss_room_offset =
            (self.tunnels.end.xy() - self.tunnels.start.xy()).map(|e| if e < 0 { -20 } else { 20 });

        let boss_room = painter.ellipsoid(Aabb {
            min: (self.tunnels.end.xy() + boss_room_offset - 30).with_z(self.tunnels.end.z - 10),
            max: (self.tunnels.end.xy() + boss_room_offset + 30).with_z(self.tunnels.end.z + 10),
        });

        let boss_room_clear = painter.ellipsoid(Aabb {
            min: (self.tunnels.end.xy() + boss_room_offset - 29).with_z(self.tunnels.end.z - 9),
            max: (self.tunnels.end.xy() + boss_room_offset + 29).with_z(self.tunnels.end.z + 9),
        });

        let random_field = RandomField::new(self.seed);

        let mut tunnels = Vec::new();
        let mut path_tunnels = Vec::new();
        let mut tunnels_clear = Vec::new();
        let mut ferns = Vec::new();
        let mut velorite_ores = Vec::new();
        let mut fire_bowls = Vec::new();
        for branch in self.tunnels.branches.iter() {
            let tunnel_radius_i32 = 4 + branch.0.x % 4;
            let in_path =
                self.tunnels.path.contains(&branch.0) && self.tunnels.path.contains(&branch.1);
            let tunnel_radius = tunnel_radius_i32 as f32;
            let start = branch.0;
            let end = branch.1;
            let ctrl0_offset = start.x % 6 * if start.y % 2 == 0 { -1 } else { 1 };
            let ctrl1_offset = end.x % 6 * if end.y % 2 == 0 { -1 } else { 1 };
            let ctrl0 = (((start + end) / 2) + start) / 2 + ctrl0_offset;
            let ctrl1 = (((start + end) / 2) + end) / 2 + ctrl1_offset;
            let tunnel = painter.cubic_bezier(start, ctrl0, ctrl1, end, tunnel_radius);
            let tunnel_clear = painter.cubic_bezier(start, ctrl0, ctrl1, end, tunnel_radius - 1.0);
            let mut fern_scatter = painter.empty();
            let mut velorite_scatter = painter.empty();
            let mut fire_bowl_scatter = painter.empty();

            let min_z = branch.0.z.min(branch.1.z);
            let max_z = branch.0.z.max(branch.1.z);
            for i in branch.0.x - tunnel_radius_i32..branch.1.x + tunnel_radius_i32 {
                for j in branch.0.y - tunnel_radius_i32..branch.1.y + tunnel_radius_i32 {
                    if random_field.get(Vec3::new(i, j, min_z)) % 6 == 0 {
                        fern_scatter = fern_scatter.union(painter.aabb(Aabb {
                            min: Vec3::new(i, j, min_z),
                            max: Vec3::new(i + 1, j + 1, max_z),
                        }));
                    }
                    if random_field.get(Vec3::new(i, j, min_z)) % 30 == 0 {
                        velorite_scatter = velorite_scatter.union(painter.aabb(Aabb {
                            min: Vec3::new(i, j, min_z),
                            max: Vec3::new(i + 1, j + 1, max_z),
                        }));
                    }
                    if random_field.get(Vec3::new(i, j, min_z)) % 30 == 0 {
                        fire_bowl_scatter = fire_bowl_scatter.union(painter.aabb(Aabb {
                            min: Vec3::new(i, j, min_z),
                            max: Vec3::new(i + 1, j + 1, max_z),
                        }));
                    }
                }
            }
            let fern = tunnel_clear.intersect(fern_scatter);
            let velorite = tunnel_clear.intersect(velorite_scatter);
            let fire_bowl = tunnel_clear.intersect(fire_bowl_scatter);
            if in_path {
                path_tunnels.push(tunnel);
            } else {
                tunnels.push(tunnel);
            }
            tunnels_clear.push(tunnel_clear);
            ferns.push(fern);
            velorite_ores.push(velorite);
            fire_bowls.push(fire_bowl);
        }

        let mut rooms = Vec::new();
        let mut rooms_clear = Vec::new();
        let mut chests_ori_0 = Vec::new();
        let mut chests_ori_2 = Vec::new();
        let mut chests_ori_4 = Vec::new();
        for terminal in self.tunnels.terminals.iter() {
            let room = painter.sphere(Aabb {
                min: terminal - 8,
                max: terminal + 8 + 1,
            });
            let room_clear = painter.sphere(Aabb {
                min: terminal - 7,
                max: terminal + 7 + 1,
            });
            rooms.push(room);
            rooms_clear.push(room_clear);

            // FIRE!!!!!
            let fire_bowl = painter.aabb(Aabb {
                min: terminal.with_z(terminal.z - 7),
                max: terminal.with_z(terminal.z - 7) + 1,
            });
            fire_bowls.push(fire_bowl);

            // Chest
            let chest_seed = random_field.get(*terminal) % 5;
            if chest_seed < 4 {
                let chest_pos = Vec3::new(terminal.x, terminal.y - 4, terminal.z - 6);
                let chest = painter.aabb(Aabb {
                    min: chest_pos,
                    max: chest_pos + 1,
                });
                chests_ori_4.push(chest);
                if chest_seed < 2 {
                    let chest_pos = Vec3::new(terminal.x, terminal.y + 4, terminal.z - 6);
                    let chest = painter.aabb(Aabb {
                        min: chest_pos,
                        max: chest_pos + 1,
                    });
                    chests_ori_0.push(chest);
                    if chest_seed < 1 {
                        let chest_pos = Vec3::new(terminal.x - 4, terminal.y, terminal.z - 6);
                        let chest = painter.aabb(Aabb {
                            min: chest_pos,
                            max: chest_pos + 1,
                        });
                        chests_ori_2.push(chest);
                    }
                }
            }
        }
        tunnels
            .into_iter()
            .chain(rooms.into_iter())
            .chain(core::iter::once(boss_room))
            .chain(core::iter::once(stump))
            .for_each(|prim| prim.fill(wood.clone()));
        path_tunnels.into_iter().for_each(|t| t.fill(dirt.clone()));

        // Clear out insides after filling the walls in
        let mut sprite_clear = Vec::new();
        tunnels_clear
            .into_iter()
            .chain(rooms_clear.into_iter())
            .chain(core::iter::once(boss_room_clear))
            .for_each(|prim| {
                sprite_clear.push(prim.translate(Vec3::new(0, 0, 1)).intersect(prim));

                prim.clear();
            });

        // Place sprites
        ferns
            .into_iter()
            .for_each(|prim| prim.fill(Fill::Block(Block::air(SpriteKind::JungleFern))));
        velorite_ores
            .into_iter()
            .for_each(|prim| prim.fill(Fill::Block(Block::air(SpriteKind::Velorite))));
        fire_bowls
            .into_iter()
            .for_each(|prim| prim.fill(Fill::Block(Block::air(SpriteKind::FireBowlGround))));

        chests_ori_0.into_iter().for_each(|prim| {
            prim.fill(Fill::Block(
                Block::air(SpriteKind::DungeonChest0).with_ori(0).unwrap(),
            ))
        });
        chests_ori_2.into_iter().for_each(|prim| {
            prim.fill(Fill::Block(
                Block::air(SpriteKind::DungeonChest0).with_ori(2).unwrap(),
            ))
        });
        chests_ori_4.into_iter().for_each(|prim| {
            prim.fill(Fill::Block(
                Block::air(SpriteKind::DungeonChest0).with_ori(4).unwrap(),
            ))
        });

        entrance_hollow.clear();
        sprite_clear.into_iter().for_each(|prim| prim.clear());
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
    match rng.gen_range(0..4) {
        0 => gnarling_stalker(pos, rng),
        1 => gnarling_mugger(pos, rng),
        _ => gnarling_logger(pos, rng),
    }
}

fn gnarling_chieftain<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.gnarling.chieftain", rng)
        .with_no_flee()
}

fn deadwood<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.wild.aggressive.deadwood", rng)
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

#[derive(Default)]
struct Tunnels {
    start: Vec3<i32>,
    end: Vec3<i32>,
    branches: Vec<(Vec3<i32>, Vec3<i32>)>,
    path: Vec<Vec3<i32>>,
    terminals: Vec<Vec3<i32>>,
}

impl Tunnels {
    /// Attempts to find a path from a `start` to the `end` using a rapidly
    /// exploring random tree (RRT). A point is sampled from an AABB extending
    /// slightly beyond the start and end in the x and y axes and below the
    /// start in the z axis to slightly below the end. Nodes are stored in a
    /// k-d tree for quicker nearest node calculations. A maximum of 7000
    /// points are sampled until the tree connects the start to the end. A
    /// final path is then reconstructed from the nodes. Returns a `Tunnels`
    /// struct of the RRT branches, the complete path, and the location of
    /// dead ends. Each branch is a tuple of the start and end locations of
    /// each segment. The path is a vector of all the nodes along the
    /// complete path from the `start` to the `end`.
    fn new<F>(
        start: Vec3<i32>,
        end: Vec3<i32>,
        is_valid_edge: F,
        radius_range: (f32, f32),
        rng: &mut impl Rng,
    ) -> Option<Self>
    where
        F: Fn(Vec3<i32>, Vec3<i32>) -> bool,
    {
        let mut nodes = Vec::new();
        let mut node_index: usize = 0;

        // HashMap<ChildNode, ParentNode>
        let mut parents = HashMap::new();

        let mut kdtree = KdTree::new();
        let startf = start.map(|a| (a + 1) as f32);
        let endf = end.map(|a| (a + 1) as f32);

        let min = Vec3::new(
            startf.x.min(endf.x),
            startf.y.min(endf.y),
            startf.z.min(endf.z),
        );
        let max = Vec3::new(
            startf.x.max(endf.x),
            startf.y.max(endf.y),
            startf.z.max(endf.z),
        );

        kdtree
            .add(&[startf.x, startf.y, startf.z], node_index)
            .ok()?;
        nodes.push(startf);
        node_index += 1;
        let mut connect = false;

        for _i in 0..7000 {
            let radius: f32 = rng.gen_range(radius_range.0..radius_range.1);
            let radius_sqrd = radius.powi(2);
            if connect {
                break;
            }
            let sampled_point = Vec3::new(
                rng.gen_range(min.x - 20.0..max.x + 20.0),
                rng.gen_range(min.y - 20.0..max.y + 20.0),
                rng.gen_range(min.z - 20.0..max.z - 7.0),
            );
            let nearest_index = *kdtree
                .nearest_one(
                    &[sampled_point.x, sampled_point.y, sampled_point.z],
                    &squared_euclidean,
                )
                .ok()?
                .1;
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
            if new_point.distance_squared(endf) < radius.powi(2) {
                connect = true;
            }
        }

        let mut path = Vec::new();
        let nearest_index = *kdtree
            .nearest_one(&[endf.x, endf.y, endf.z], &squared_euclidean)
            .ok()?
            .1;
        kdtree.add(&[endf.x, endf.y, endf.z], node_index).ok()?;
        nodes.push(endf);
        parents.insert(node_index, nearest_index);
        path.push(endf);
        let mut current_node_index = node_index;
        while current_node_index > 0 {
            current_node_index = *parents.get(&current_node_index).unwrap();
            path.push(nodes[current_node_index]);
        }

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
        let path = path
            .iter()
            .map(|a| a.map(|e| e.floor() as i32))
            .collect::<Vec<Vec3<i32>>>();

        Some(Self {
            start,
            end,
            branches,
            path,
            terminals,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creating_entities() {
        let pos = Vec3::zero();
        let mut rng = thread_rng();

        gnarling_mugger(pos, &mut rng);
        gnarling_stalker(pos, &mut rng);
        gnarling_logger(pos, &mut rng);
        gnarling_chieftain(pos, &mut rng);
        deadwood(pos, &mut rng);
        mandragora(pos, &mut rng);
        wood_golem(pos, &mut rng);
        harvester_boss(pos, &mut rng);
    }
}
