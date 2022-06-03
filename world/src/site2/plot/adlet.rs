use super::*;
use crate::{
    assets::AssetHandle,
    site2::{gen::PrimitiveTransform, util::Dir},
    util::{attempt, sampler::Sampler, FastNoise, RandomField},
    IndexRef, Land,
};
use common::{
    generation::{ChunkSupplement, EntityInfo},
    terrain::{Structure as PrefabStructure, StructuresGroup},
};
use lazy_static::lazy_static;
use rand::prelude::*;
use std::{
    collections::HashMap,
    f32::consts::{PI, TAU},
    ops::{Add, Div, Mul, Sub},
};
use vek::*;

const ANGLE_SAMPLES: usize = 128;
const WALL_DELTA: f32 = 4.0;

pub struct AdletStronghold {
    name: String,
    seed: u32,
    entrance: Vec2<i32>,
    wall_center: Vec2<i32>,
    wall_radius: i32,
    wall_alt: f32,
    wall_alt_samples: [f32; ANGLE_SAMPLES],
    // Structure indicates the kind of structure it is, vec2 is relative position of structure
    // compared to wall_center, dir tells which way structure should face
    outer_structures: Vec<(AdletStructure, Vec2<i32>, Dir)>,
    tunnel_length: i32,
    cavern_center: Vec2<i32>,
    cavern_alt: f32,
    cavern_radius: i32,
    // Structure indicates the kind of structure it is, vec2 is relative position of structure
    // compared to cavern_center, dir tells which way structure should face
    cavern_structures: Vec<(AdletStructure, Vec2<i32>, Dir)>,
}

#[derive(Copy, Clone)]
enum AdletStructure {
    Igloo(u8),
    TunnelEntrance,
    SpeleothemCluster,
    CentralBonfire,
    YetiPit,
    Tannery,
    AnimalPen,
    CookFire,
    RockHut,
    BoneHut,
}

impl AdletStructure {
    fn required_separation(&self, other: &Self) -> i32 {
        let radius = |structure: &Self| match structure {
            Self::Igloo(radius) => *radius as i32 + 3,
            Self::TunnelEntrance => 16,
            Self::SpeleothemCluster => 8,
            Self::CentralBonfire => 10,
            Self::YetiPit => 20,
            Self::Tannery => 10,
            Self::AnimalPen => 16,
            Self::CookFire => 3,
            Self::RockHut => 6,
            Self::BoneHut => 8,
        };

        let additional_padding = match (self, other) {
            (Self::Igloo(_), Self::Igloo(_)) => 3,
            (Self::CookFire, a) | (a, Self::CookFire)
                if !matches!(a, Self::RockHut | Self::BoneHut) =>
            {
                30
            },
            _ => 0,
        };

        radius(self) + radius(other) + additional_padding
    }
}

impl AdletStronghold {
    pub fn generate(wpos: Vec2<i32>, land: &Land, rng: &mut impl Rng, index: IndexRef) -> Self {
        let name = NameGen::location(rng).generate_adlet();
        let seed = rng.gen();
        let entrance = wpos;

        let wall_radius = {
            let unit_size = rng.gen_range(8..11);
            let num_units = rng.gen_range(6..9);
            let variation = rng.gen_range(0..10);
            unit_size * num_units + variation
        };
        let wall_center = entrance.map(|x| x + rng.gen_range(-wall_radius / 4..wall_radius / 4));
        let wall_alt = land.get_alt_approx(wall_center) + 10.0;

        let mut wall_alt_sample_positions = [Vec2::zero(); ANGLE_SAMPLES];
        for i in 0..ANGLE_SAMPLES {
            let theta = i as f32 / ANGLE_SAMPLES as f32 * TAU;
            let sample_rpos = Vec2::new(
                theta.cos() * wall_radius as f32,
                theta.sin() * wall_radius as f32,
            );
            wall_alt_sample_positions[i] = sample_rpos.as_() + wall_center;
        }
        let mut wall_alt_samples = wall_alt_sample_positions.map(|pos| {
            land.column_sample(pos, index)
                .map_or(land.get_alt_approx(pos), |col| col.alt)
                .min(wall_alt)
        });
        loop {
            let mut changed = false;
            for i in 0..wall_alt_samples.len() {
                let tmp = (wall_alt_samples[(i + 1) % ANGLE_SAMPLES] - WALL_DELTA)
                    .max(wall_alt_samples[(i + ANGLE_SAMPLES - 1) % ANGLE_SAMPLES] - WALL_DELTA);
                if tmp > wall_alt_samples[i] {
                    wall_alt_samples[i] = tmp;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // Find direction that allows for deep enough site
        let angle_samples = (0..64).into_iter().map(|x| x as f32 / 64.0 * TAU);
        // Sample blocks 40-50 away, use angle where these positions are highest
        // relative to entrance
        let angle = angle_samples
            .max_by_key(|theta| {
                let entrance_height = land.get_alt_approx(entrance);
                let height =
                    |pos: Vec2<f32>| land.get_alt_approx(pos.as_() + entrance) - entrance_height;
                let (x, y) = (theta.cos(), theta.sin());
                (40..=50)
                    .into_iter()
                    .map(|r| {
                        let rpos = Vec2::new(r as f32 * x, r as f32 * y);
                        height(rpos) as i32
                    })
                    .sum::<i32>()
            })
            .unwrap_or(0.0);

        let cavern_radius: i32 = {
            let unit_size = rng.gen_range(10..15);
            let num_units = rng.gen_range(4..8);
            let variation = rng.gen_range(0..30);
            unit_size * num_units + variation
        };

        let tunnel_length = rng.gen_range(35_i32..50);

        let cavern_center = entrance
            + (Vec2::new(angle.cos(), angle.sin()) * (tunnel_length as f32 + cavern_radius as f32))
                .as_();

        let cavern_alt = (land.get_alt_approx(cavern_center) - cavern_radius as f32)
            .min(land.get_alt_approx(entrance));

        let mut outer_structures = Vec::<(AdletStructure, Vec2<i32>, Dir)>::new();

        outer_structures.push((
            AdletStructure::TunnelEntrance,
            entrance - wall_center,
            Dir::from_vector(entrance - cavern_center),
        ));

        let desired_structures = wall_radius.pow(2) / 100;
        for _ in 0..desired_structures {
            if let Some((rpos, kind)) = attempt(50, || {
                // Choose structure kind
                let structure_kind = match rng.gen_range(0..10) {
                    // TODO: Add more variants
                    _ => AdletStructure::Igloo(rng.gen_range(6..12)),
                };

                // Choose relative position
                let structure_center = {
                    let theta = rng.gen::<f32>() * TAU;
                    // 0.8 to keep structures not directly against wall
                    let radius = wall_radius as f32 * rng.gen::<f32>().sqrt() * 0.8;
                    let x = radius * theta.sin();
                    let y = radius * theta.cos();
                    Vec2::new(x, y).as_()
                };

                // Check that structure not in the water or too close to another structure
                if land
                    .get_chunk_wpos(structure_center.as_() + wall_center)
                    .map_or(false, |c| c.is_underwater())
                    || outer_structures.iter().any(|(kind, rpos, _dir)| {
                        structure_center.distance_squared(*rpos)
                            < structure_kind.required_separation(kind).pow(2)
                    })
                {
                    None
                } else {
                    Some((structure_center, structure_kind))
                }
            }) {
                let dir_to_wall = Dir::from_vector(rpos);
                let door_rng: u32 = rng.gen_range(0..9);
                let door_dir = match door_rng {
                    0..=3 => dir_to_wall,
                    4..=5 => dir_to_wall.rotated_cw(),
                    6..=7 => dir_to_wall.rotated_ccw(),
                    // Should only be 8
                    _ => dir_to_wall.opposite(),
                };
                outer_structures.push((kind, rpos, door_dir));
            }
        }

        let mut cavern_structures = Vec::<(AdletStructure, Vec2<i32>, Dir)>::new();

        fn valid_cavern_struct_pos(
            structures: &Vec<(AdletStructure, Vec2<i32>, Dir)>,
            structure: AdletStructure,
            rpos: Vec2<i32>,
        ) -> bool {
            structures.iter().all(|(kind, rpos2, _dir)| {
                rpos.distance_squared(*rpos2) > structure.required_separation(kind).pow(2)
            })
        }

        // Add speleothem clusters (stalagmites/stalactites)
        let desired_speleothem_clusters = cavern_radius.pow(2) / 1500;
        for _ in 0..desired_speleothem_clusters {
            if let Some(mut rpos) = attempt(25, || {
                let rpos = {
                    let theta = rng.gen_range(0.0..TAU);
                    // sqrt biases radius away from center, leading to even distribution in circle
                    let radius = rng.gen::<f32>().sqrt() * cavern_radius as f32;
                    Vec2::new(theta.cos() * radius, theta.sin() * radius).as_()
                };
                valid_cavern_struct_pos(&cavern_structures, AdletStructure::SpeleothemCluster, rpos)
                    .then_some(rpos)
            }) {
                // Dir doesn't matter since these are directionless
                cavern_structures.push((AdletStructure::SpeleothemCluster, rpos, Dir::X));
                let desired_adjacent_clusters = rng.gen_range(1..8);
                for _ in 0..desired_adjacent_clusters {
                    // Choose a relative position adjacent to initial speleothem cluster
                    let adj_rpos = {
                        let theta = rng.gen_range(0.0..TAU);
                        let radius = rng.gen_range(15.0..25.0);
                        let rrpos = Vec2::new(theta.cos() * radius, theta.sin() * radius).as_();
                        rpos + rrpos
                    };
                    if valid_cavern_struct_pos(
                        &cavern_structures,
                        AdletStructure::SpeleothemCluster,
                        adj_rpos,
                    ) {
                        cavern_structures.push((
                            AdletStructure::SpeleothemCluster,
                            adj_rpos,
                            Dir::X,
                        ));
                        // Set new rpos to next cluster is adjacent to most recently placed
                        rpos = adj_rpos;
                    } else {
                        // If any cluster ever fails to place, break loop and stop creating cluster
                        // chain
                        break;
                    }
                }
            }
        }

        // Attempt to place central bonfire
        if let Some(rpos) = attempt(50, || {
            let rpos = {
                let theta = rng.gen_range(0.0..TAU);
                let radius = rng.gen::<f32>() * cavern_radius as f32 * 0.5;
                Vec2::new(theta.cos() * radius, theta.sin() * radius).as_()
            };
            valid_cavern_struct_pos(&cavern_structures, AdletStructure::CentralBonfire, rpos)
                .then_some(rpos)
        })
        .or_else(|| {
            attempt(100, || {
                let rpos = {
                    let theta = rng.gen_range(0.0..TAU);
                    // If selecting a spot near the center failed, find a spot anywhere in the
                    // cavern
                    let radius = rng.gen::<f32>().sqrt() * cavern_radius as f32;
                    Vec2::new(theta.cos() * radius, theta.sin() * radius).as_()
                };
                valid_cavern_struct_pos(&cavern_structures, AdletStructure::CentralBonfire, rpos)
                    .then_some(rpos)
            })
        }) {
            // Direction doesn't matter for central bonfire
            cavern_structures.push((AdletStructure::CentralBonfire, rpos, Dir::X));
        }

        // Attempt to place yeti pit
        if let Some(rpos) = attempt(50, || {
            let rpos = {
                let theta = {
                    let angle_range = PI / 2.0;
                    let angle_offset = rng.gen_range(-angle_range..angle_range);
                    angle + angle_offset
                };
                let radius = (rng.gen::<f32>() + 3.0) / 4.0 * cavern_radius as f32;
                Vec2::new(theta.cos() * radius, theta.sin() * radius).as_()
            };
            valid_cavern_struct_pos(&cavern_structures, AdletStructure::YetiPit, rpos)
                .then_some(rpos)
        }) {
            // Direction doesn't matter for yeti pit
            cavern_structures.push((AdletStructure::YetiPit, rpos, Dir::X));
        }

        // Attempt to place some general structures somewhat near the center
        let desired_structures = cavern_radius.pow(2) / 500;
        for _ in 0..desired_structures {
            if let Some((structure, rpos)) = attempt(25, || {
                let rpos = {
                    let theta = rng.gen_range(0.0..TAU);
                    // sqrt biases radius away from center, leading to even distribution in circle
                    let radius = rng.gen::<f32>().sqrt() * cavern_radius as f32 * 0.6;
                    Vec2::new(theta.cos() * radius, theta.sin() * radius).as_()
                };
                let structure = match rng.gen_range(0..2) {
                    0 => AdletStructure::Tannery,
                    _ => AdletStructure::AnimalPen,
                };

                valid_cavern_struct_pos(&cavern_structures, structure, rpos)
                    .then_some((structure, rpos))
            }) {
                // Direction facing the central bonfire
                let dir = Dir::from_vector(rpos).opposite();
                cavern_structures.push((structure, rpos, dir));
            }
        }

        Self {
            name,
            seed,
            entrance,
            wall_center,
            wall_radius,
            wall_alt,
            wall_alt_samples,
            outer_structures,
            tunnel_length,
            cavern_center,
            cavern_radius,
            cavern_alt,
            cavern_structures,
        }
    }

    pub fn name(&self) -> &str { &self.name }

    // pub fn origin(&self) -> Vec2<i32> { self.cavern_center }

    pub fn radius(&self) -> i32 { self.cavern_radius + self.tunnel_length + 5 }

    pub fn plot_tiles(&self, origin: Vec2<i32>) -> (Aabr<i32>, Aabr<i32>) {
        // Cavern
        let size = self.cavern_radius / tile::TILE_SIZE as i32;
        let offset = (self.cavern_center - origin) / tile::TILE_SIZE as i32;
        let cavern_aabr = Aabr {
            min: Vec2::broadcast(-size) + offset,
            max: Vec2::broadcast(size) + offset,
        };
        // Wall
        let size = (self.wall_radius * 5 / 4) / tile::TILE_SIZE as i32;
        let offset = (self.wall_center - origin) / tile::TILE_SIZE as i32;
        let wall_aabr = Aabr {
            min: Vec2::broadcast(-size) + offset,
            max: Vec2::broadcast(size) + offset,
        };
        (cavern_aabr, wall_aabr)
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            waypoints: false,
            trees: wpos.distance_squared(self.entrance) > (self.wall_radius * 5 / 4).pow(2),
            ..SpawnRules::default()
        }
    }

    // TODO: Find a better way of spawning entities in site2
    pub fn apply_supplement<'a>(
        &'a self,
        // NOTE: Used only for dynamic elements like chests and entities!
        dynamic_rng: &mut impl Rng,
        wpos2d: Vec2<i32>,
        supplement: &mut ChunkSupplement,
    ) {
        let rpos = wpos2d - self.cavern_center;
        let area = Aabr {
            min: rpos,
            max: rpos + TerrainChunkSize::RECT_SIZE.map(|e| e as i32),
        };
    }
}

impl Structure for AdletStronghold {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_adletstronghold\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_adletstronghold")]
    fn render_inner(&self, _site: &Site, land: &Land, painter: &Painter) {
        let wall_mat = Fill::Brick(BlockKind::Snow, Rgb::new(175, 175, 175), 25);
        // Wall
        painter
            .cylinder_with_radius(
                self.wall_center
                    .with_z(self.wall_alt as i32 - self.wall_radius * 2),
                self.wall_radius as f32 + 3.0,
                self.wall_radius as f32 * 2.5,
            )
            .without(
                painter.cylinder_with_radius(
                    self.wall_center
                        .with_z(self.wall_alt as i32 - self.wall_radius * 2),
                    self.wall_radius as f32,
                    self.wall_radius as f32 * 2.5,
                ),
            )
            .sample_with_column({
                let wall_alt_samples = self.wall_alt_samples;
                let wall_center = self.wall_center;
                let theta = move |pos: Vec2<i32>| {
                    let rpos: Vec2<f32> = (pos - wall_center).as_();
                    let theta = rpos.y.atan2(rpos.x);
                    if theta > 0.0 { theta } else { theta + TAU }
                };
                move |pos, col| {
                    let index = (theta(pos.xy()) * ANGLE_SAMPLES as f32 / TAU)
                        .floor()
                        .max(0.0) as usize
                        % ANGLE_SAMPLES;
                    (col.alt.sub(10.0)
                        ..wall_alt_samples[index]
                            .add(12.0)
                            .div(WALL_DELTA)
                            .floor()
                            .mul(WALL_DELTA))
                        .contains(&(pos.z as f32))
                }
            })
            .fill(wall_mat);

        // Tunnel
        let dist: f32 = self.cavern_center.as_().distance(self.entrance.as_());
        let tunnel_radius = 10.0;
        let dir = Dir::from_vector(self.entrance - self.cavern_center);
        let tunnel_start: Vec3<f32> = match dir {
            Dir::X => Vec2::new(self.entrance.x + 7, self.entrance.y),
            Dir::Y => Vec2::new(self.entrance.x, self.entrance.y + 7),
            Dir::NegX => Vec2::new(self.entrance.x - 7, self.entrance.y),
            Dir::NegY => Vec2::new(self.entrance.x, self.entrance.y - 7),
        }
        .as_()
        .with_z(self.cavern_alt - 1.0);
        // Adds cavern radius to ensure that tunnel fully bores into cavern
        let tunnel_end =
            ((self.cavern_center.as_() - self.entrance.as_()) * self.tunnel_length as f32 / dist)
                .with_z(self.cavern_alt - 1.0)
                + self.entrance.as_();

        let tunnel_end = match dir {
            Dir::X => Vec3::new(tunnel_end.x - 7.0, tunnel_start.y, tunnel_end.z),
            Dir::Y => Vec3::new(tunnel_start.x, tunnel_end.y - 7.0, tunnel_end.z),
            Dir::NegX => Vec3::new(tunnel_end.x + 7.0, tunnel_start.y, tunnel_end.z),
            Dir::NegY => Vec3::new(tunnel_start.x, tunnel_end.y + 7.0, tunnel_end.z),
        };

        let stone_fill = Fill::Brick(BlockKind::Rock, Rgb::new(90, 110, 150), 21);
        // Platform
        painter
            .aabb(Aabb {
                min: (self.entrance - 20).with_z(self.cavern_alt as i32 - 30),
                max: (self.entrance + 20).with_z(self.cavern_alt as i32 + 1),
            })
            .without(painter.aabb(Aabb {
                min: (self.entrance - 19).with_z(self.cavern_alt as i32),
                max: (self.entrance + 19).with_z(self.cavern_alt as i32 + 2),
            }))
            .fill(stone_fill.clone());

        let valid_entrance = painter.segment_prism(tunnel_start, tunnel_end, 20.0, 30.0);
        painter
            .segment_prism(tunnel_start, tunnel_end, 10.0, 10.0)
            .clear();
        painter
            .line(
                tunnel_start + Vec3::new(0.0, 0.0, 10.0),
                tunnel_end + Vec3::new(0.0, 0.0, 10.0),
                10.0,
            )
            .clear();
        painter
            .line(
                tunnel_start
                    + match dir {
                        Dir::X => Vec3::new(0.0, 4.0, 7.0),
                        Dir::Y => Vec3::new(4.0, 0.0, 7.0),
                        Dir::NegX => Vec3::new(0.0, 4.0, 7.0),
                        Dir::NegY => Vec3::new(4.0, 0.0, 7.0),
                    },
                tunnel_end
                    + match dir {
                        Dir::X => Vec3::new(0.0, 4.0, 7.0),
                        Dir::Y => Vec3::new(4.0, 0.0, 7.0),
                        Dir::NegX => Vec3::new(0.0, 4.0, 7.0),
                        Dir::NegY => Vec3::new(4.0, 0.0, 7.0),
                    },
                8.0,
            )
            .intersect(valid_entrance)
            .clear();
        painter
            .line(
                tunnel_start
                    + match dir {
                        Dir::X => Vec3::new(0.0, -4.0, 7.0),
                        Dir::Y => Vec3::new(-4.0, 0.0, 7.0),
                        Dir::NegX => Vec3::new(0.0, -4.0, 7.0),
                        Dir::NegY => Vec3::new(-4.0, 0.0, 7.0),
                    },
                tunnel_end
                    + match dir {
                        Dir::X => Vec3::new(0.0, -4.0, 7.0),
                        Dir::Y => Vec3::new(-4.0, 0.0, 7.0),
                        Dir::NegX => Vec3::new(0.0, -4.0, 7.0),
                        Dir::NegY => Vec3::new(-4.0, 0.0, 7.0),
                    },
                8.0,
            )
            .intersect(valid_entrance)
            .clear();

        // Cavern
        painter
            .sphere_with_radius(
                self.cavern_center.with_z(self.cavern_alt as i32),
                self.cavern_radius as f32,
            )
            .intersect(painter.aabb(Aabb {
                min: (self.cavern_center - self.cavern_radius).with_z(self.cavern_alt as i32),
                max: self.cavern_center.with_z(self.cavern_alt as i32) + self.cavern_radius,
            }))
            .sample_with_column({
                let origin = self.cavern_center.with_z(self.cavern_alt as i32);
                let radius_sqr = self.cavern_radius.pow(2);
                move |pos, col| {
                    let alt = col.basement - col.cliff_offset;
                    let sphere_alt = ((radius_sqr - origin.xy().distance_squared(pos.xy())) as f32)
                        .sqrt()
                        + origin.z as f32;
                    // Some sort of smooth min
                    let alt = if alt < sphere_alt {
                        alt
                    } else if sphere_alt - alt < 10.0 {
                        f32::lerp(sphere_alt, alt, 1.0 / (alt - sphere_alt).max(1.0))
                    } else {
                        sphere_alt
                    };

                    let noise = FastNoise::new(333);
                    let alt_offset = noise.get(pos.with_z(0).as_() / 5.0).powi(2) * 15.0;

                    let alt = alt - alt_offset;

                    pos.z < alt as i32
                }
            })
            .clear();

        for (structure, wpos, alt, dir) in self
            .outer_structures
            .iter()
            .map(|(structure, rpos, dir)| {
                let wpos = rpos + self.wall_center;
                (structure, wpos, land.get_alt_approx(wpos), dir)
            })
            .chain(self.cavern_structures.iter().map(|(structure, rpos, dir)| {
                (
                    structure,
                    rpos + self.cavern_center,
                    self.cavern_alt as f32,
                    dir,
                )
            }))
        {
            let bone_fill = Fill::Brick(BlockKind::Misc, Rgb::new(200, 160, 140), 1);
            let snow_fill = Fill::Block(Block::new(BlockKind::Snow, Rgb::new(255, 255, 255)));
            let stone_fill = Fill::Block(Block::new(BlockKind::Rock, Rgb::new(100, 100, 100)));
            match structure {
                AdletStructure::TunnelEntrance => {
                    let rib_width_curve = |i: f32| 0.5 * (0.4 * i + 1.0).log2() + 5.5;
                    let spine_curve_amplitude = 0.0;
                    let spine_curve_wavelength = 1.0;
                    let spine_curve_function = |i: f32, amplitude: f32, wavelength: f32| {
                        amplitude * (2.0 * PI * (1.0 / wavelength) * i).sin()
                    };
                    let rib_cage_config = RibCageGenerator {
                        dir: *dir,
                        spine_radius: 2.5,
                        length: 40,
                        spine_curve_function,
                        spine_curve_amplitude,
                        spine_curve_wavelength,
                        spine_height: self.cavern_alt + 16.0,
                        spine_start_z_offset: 2.0,
                        spine_ctrl0_z_offset: 3.0,
                        spine_ctrl1_z_offset: 5.0,
                        spine_end_z_offset: 1.0,
                        spine_ctrl0_length_fraction: 0.3,
                        spine_ctrl1_length_fraction: 0.7,
                        rib_base_alt: self.cavern_alt - 1.0,
                        rib_spacing: 7,
                        rib_radius: 1.7,
                        rib_run: 5.0,
                        rib_ctrl0_run_fraction: 0.3,
                        rib_ctrl1_run_fraction: 0.5,
                        rib_ctrl0_width_offset: 5.0,
                        rib_ctrl1_width_offset: 3.0,
                        rib_width_curve,
                        rib_ctrl0_height_fraction: 0.8,
                        rib_ctrl1_height_fraction: 0.4,
                        vertebra_radius: 4.0,
                        vertebra_width: 1.0,
                        vertebra_z_offset: 0.3,
                    };
                    let rib_cage =
                        rib_cage_config.bones(wpos + 40 * dir.opposite().to_vec2(), painter);
                    for bone in rib_cage {
                        bone.fill(bone_fill.clone());
                    }
                },
                AdletStructure::Igloo(radius) => {
                    let center_pos = wpos.with_z(alt as i32 - *radius as i32);
                    painter
                        .sphere_with_radius(center_pos, *radius as f32)
                        .fill(snow_fill);
                    let room_radius = *radius as i32 * 2 / 5;
                    let room_center = |dir: Dir| center_pos + dir.to_vec2() * room_radius;
                    painter
                        .sphere_with_radius(room_center(dir.rotated_cw()), room_radius as f32)
                        .union(
                            painter.sphere_with_radius(
                                room_center(dir.opposite()),
                                room_radius as f32,
                            ),
                        )
                        .union(
                            painter.sphere_with_radius(
                                room_center(dir.rotated_ccw()),
                                room_radius as f32,
                            ),
                        )
                        .clear();

                    let mid_pos =
                        (center_pos + dir.to_vec2().with_z(1).mul(*radius as i32)).as_::<f32>();
                    painter
                        .line(center_pos.as_::<f32>(), mid_pos, 2.0)
                        .union(painter.line(
                            mid_pos,
                            center_pos + Vec3::unit_z().mul(2 * *radius as i32),
                            2.0,
                        ))
                        .clear();
                },
                AdletStructure::SpeleothemCluster => {
                    painter
                        .cylinder_with_radius(wpos.with_z(alt as i32), 6.0, 20.0)
                        .fill(stone_fill.clone());
                },
                AdletStructure::CentralBonfire => {
                    painter
                        .cylinder_with_radius(wpos.with_z(alt as i32), 3.0, 2.0)
                        .fill(stone_fill.clone());
                    painter.sprite(wpos.with_z(alt as i32 + 2), SpriteKind::FireBowlGround);
                },
                AdletStructure::YetiPit => {
                    painter
                        .aabb(Aabb {
                            min: wpos.with_z(alt as i32).map(|x| x - 5),
                            max: wpos.map(|x| x + 5).with_z(alt as i32),
                        })
                        .clear();
                },
                AdletStructure::Tannery => {
                    painter
                        .aabb(Aabb {
                            min: wpos.map(|x| x - 5).with_z(alt as i32),
                            max: wpos.with_z(alt as i32).map(|x| x + 5),
                        })
                        .fill(bone_fill.clone());
                },
                AdletStructure::AnimalPen => {
                    painter
                        .aabb(Aabb {
                            min: wpos.map(|x| x - 5).with_z(alt as i32),
                            max: wpos.with_z(alt as i32).map(|x| x + 5),
                        })
                        .fill(bone_fill.clone());
                    painter
                        .aabb(Aabb {
                            min: wpos.map(|x| x - 4).with_z(alt as i32),
                            max: wpos.with_z(alt as i32 + 1).map(|x| x + 4),
                        })
                        .clear();
                },
                _ => panic!(),
            }
        }
    }
}

struct RibCageGenerator {
    dir: Dir,
    length: u32,
    spine_height: f32,
    spine_radius: f32,
    /// Defines how the spine curves given the ratio along the spine from 0.0 to
    /// 1.0, the amplitude, and the wavelength
    spine_curve_function: fn(f32, f32, f32) -> f32,
    spine_curve_amplitude: f32,
    // FIXME: CAN CAUSE DIV BY 0 IF VALUE IS 0.0
    spine_curve_wavelength: f32,
    spine_start_z_offset: f32,
    spine_ctrl0_z_offset: f32,
    spine_ctrl1_z_offset: f32,
    spine_end_z_offset: f32,
    spine_ctrl0_length_fraction: f32,
    spine_ctrl1_length_fraction: f32,
    rib_base_alt: f32,
    rib_spacing: usize,
    rib_radius: f32,
    rib_run: f32,
    rib_ctrl0_run_fraction: f32,
    rib_ctrl1_run_fraction: f32,
    rib_ctrl0_width_offset: f32,
    rib_ctrl1_width_offset: f32,
    /// Defines how much ribs flare out as you go along the rib cage given the
    /// ratio along the spine from 0.0 to 1.0
    rib_width_curve: fn(f32) -> f32,
    rib_ctrl0_height_fraction: f32,
    rib_ctrl1_height_fraction: f32,
    vertebra_radius: f32,
    vertebra_width: f32,
    vertebra_z_offset: f32,
}

impl RibCageGenerator {
    fn bones<'a>(&self, origin: Vec2<i32>, painter: &'a Painter) -> Vec<PrimitiveRef<'a>> {
        let RibCageGenerator {
            dir,
            length,
            spine_height,
            spine_radius,
            spine_curve_function,
            spine_curve_amplitude,
            spine_curve_wavelength,
            spine_start_z_offset,
            spine_ctrl0_z_offset,
            spine_ctrl1_z_offset,
            spine_end_z_offset,
            spine_ctrl0_length_fraction,
            spine_ctrl1_length_fraction,
            rib_base_alt,
            rib_spacing,
            rib_radius,
            rib_run,
            rib_ctrl0_run_fraction,
            rib_ctrl1_run_fraction,
            rib_ctrl0_width_offset,
            rib_ctrl1_width_offset,
            rib_width_curve,
            rib_ctrl0_height_fraction,
            rib_ctrl1_height_fraction,
            vertebra_radius,
            vertebra_width,
            vertebra_z_offset,
        } = self;
        let length_f32 = *length as f32;

        let mut bones = Vec::new();

        let spine_start = origin
            .map(|e| e as f32)
            .with_z(spine_height + spine_start_z_offset)
            + spine_curve_function(0.0, *spine_curve_amplitude, *spine_curve_wavelength)
                * Vec3::unit_y();
        let spine_ctrl0 = origin
            .map(|e| e as f32)
            .with_z(spine_height + spine_ctrl0_z_offset)
            + length_f32 * spine_ctrl0_length_fraction * Vec3::unit_x()
            + spine_curve_function(
                length_f32 * spine_ctrl0_length_fraction,
                *spine_curve_amplitude,
                *spine_curve_wavelength,
            ) * Vec3::unit_y();
        let spine_ctrl1 = origin
            .map(|e| e as f32)
            .with_z(spine_height + spine_ctrl1_z_offset)
            + length_f32 * spine_ctrl1_length_fraction * Vec3::unit_x()
            + spine_curve_function(
                length_f32 * spine_ctrl1_length_fraction,
                *spine_curve_amplitude,
                *spine_curve_wavelength,
            ) * Vec3::unit_y();
        let spine_end = origin
            .map(|e| e as f32)
            .with_z(spine_height + spine_end_z_offset)
            + length_f32 * Vec3::unit_x()
            + spine_curve_function(length_f32, *spine_curve_amplitude, *spine_curve_wavelength)
                * Vec3::unit_y();
        let spine_bezier = CubicBezier3 {
            start: spine_start,
            ctrl0: spine_ctrl0,
            ctrl1: spine_ctrl1,
            end: spine_end,
        };
        let spine = painter.cubic_bezier(
            spine_start,
            spine_ctrl0,
            spine_ctrl1,
            spine_end,
            *spine_radius,
        );

        let rotation_origin = Vec3::new(
            spine_start.x as f32,
            spine_start.y as f32 + 0.5,
            spine_start.z as f32,
        );
        let rotate = |prim: PrimitiveRef<'a>, dir: &Dir| -> PrimitiveRef<'a> {
            match dir {
                Dir::X => prim,
                Dir::Y => prim.rotate_about(Mat3::rotation_z(0.5 * PI).as_(), rotation_origin),
                Dir::NegX => prim.rotate_about(Mat3::rotation_z(PI).as_(), rotation_origin),
                Dir::NegY => prim.rotate_about(Mat3::rotation_z(1.5 * PI).as_(), rotation_origin),
            }
        };

        let spine_rotated = rotate(spine, dir);
        bones.push(spine_rotated);

        for i in (0..*length).step_by(*rib_spacing) {
            enum Side {
                Left,
                Right,
            }

            let rib = |side| -> PrimitiveRef {
                let y_offset_multiplier = match side {
                    Side::Left => 1.0,
                    Side::Right => -1.0,
                };
                let rib_start: Vec3<f32> = spine_bezier
                    .evaluate((i as f32 / length_f32).clamped(0.0, 1.0))
                    + y_offset_multiplier * Vec3::unit_y();
                let rib_ctrl0 = Vec3::new(
                    rib_start.x + rib_ctrl0_run_fraction * rib_run,
                    rib_start.y
                        + y_offset_multiplier * rib_ctrl0_width_offset
                        + y_offset_multiplier * rib_width_curve(i as f32),
                    rib_base_alt + rib_ctrl0_height_fraction * (rib_start.z - rib_base_alt),
                );
                let rib_ctrl1 = Vec3::new(
                    rib_start.x + rib_ctrl1_run_fraction * rib_run,
                    rib_start.y
                        + y_offset_multiplier * rib_ctrl1_width_offset
                        + y_offset_multiplier * rib_width_curve(i as f32),
                    rib_base_alt + rib_ctrl1_height_fraction * (rib_start.z - rib_base_alt),
                );
                let rib_end = Vec3::new(
                    rib_start.x + rib_run,
                    rib_start.y + y_offset_multiplier * rib_width_curve(i as f32),
                    *rib_base_alt,
                );
                painter.cubic_bezier(rib_start, rib_ctrl0, rib_ctrl1, rib_end, *rib_radius)
            };
            let l_rib = rib(Side::Left);
            let l_rib_rotated = rotate(l_rib, dir);
            bones.push(l_rib_rotated);

            let r_rib = rib(Side::Right);
            let r_rib_rotated = rotate(r_rib, dir);
            bones.push(r_rib_rotated);

            let vertebra_start: Vec3<f32> =
                spine_bezier.evaluate((i as f32 / length_f32).clamped(0.0, 1.0)) + Vec3::unit_y();
            let vertebra = painter.ellipsoid(Aabb {
                min: Vec3::new(
                    vertebra_start.x - vertebra_width,
                    vertebra_start.y - 1.0 - vertebra_radius,
                    vertebra_start.z - vertebra_radius + vertebra_z_offset,
                )
                .map(|e| e.round() as i32),
                max: Vec3::new(
                    vertebra_start.x + vertebra_width,
                    vertebra_start.y - 1.0 + vertebra_radius,
                    vertebra_start.z + vertebra_radius + vertebra_z_offset,
                )
                .map(|e| e.round() as i32),
            });
            let vertebra_rotated = rotate(vertebra, dir);
            bones.push(vertebra_rotated);
        }

        bones
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creating_entities() {
        // let pos = Vec3::zero();
        // let mut rng = thread_rng();

        // gnarling_mugger(pos, &mut rng);
        // gnarling_stalker(pos, &mut rng);
        // gnarling_logger(pos, &mut rng);
        // gnarling_chieftain(pos, &mut rng);
        // deadwood(pos, &mut rng);
        // mandragora(pos, &mut rng);
        // wood_golem(pos, &mut rng);
        // harvester_boss(pos, &mut rng);
    }
}
