use super::*;
use crate::{
    assets::AssetHandle,
    site2::{gen::PrimitiveTransform, util::Dir},
    util::{attempt, sampler::Sampler, FastNoise, RandomField, NEIGHBORS, NEIGHBORS3},
    IndexRef, Land,
};
use common::{
    generation::{ChunkSupplement, EntityInfo},
    terrain::{Structure as PrefabStructure, StructuresGroup},
};
use lazy_static::lazy_static;
use rand::prelude::*;
use std::{
    f32::consts::{PI, TAU},
    sync::Arc,
};
use vek::{num_integer::Roots, *};

pub struct AdletStronghold {
    name: String,
    entrance: Vec2<i32>,
    surface_radius: i32,
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
    Igloo,
    TunnelEntrance,
    SpeleothemCluster,
    Bonfire,
    YetiPit,
    Tannery,
    AnimalPen,
    CookFire,
    RockHut,
    BoneHut,
    BossBoneHut,
}

impl AdletStructure {
    fn required_separation(&self, other: &Self) -> i32 {
        let radius = |structure: &Self| match structure {
            Self::Igloo => 20,
            Self::TunnelEntrance => 32,
            Self::SpeleothemCluster => 4,
            Self::YetiPit => 32,
            Self::Tannery => 6,
            Self::AnimalPen => 8,
            Self::CookFire => 3,
            Self::RockHut => 4,
            Self::BoneHut => 11,
            Self::BossBoneHut => 14,
            Self::Bonfire => 10,
        };

        let additional_padding = match (self, other) {
            (Self::Igloo, Self::Igloo) => 3,
            (Self::BoneHut, Self::BoneHut) => 3,
            (Self::Tannery, Self::Tannery) => 5,
            (Self::CookFire, Self::CookFire) => 20,
            (Self::AnimalPen, Self::AnimalPen) => 8,
            // Keep these last
            (Self::SpeleothemCluster, Self::SpeleothemCluster) => 0,
            (Self::SpeleothemCluster, _) | (_, Self::SpeleothemCluster) => 5,
            _ => 0,
        };

        radius(self) + radius(other) + additional_padding
    }
}

impl AdletStronghold {
    pub fn generate(wpos: Vec2<i32>, land: &Land, rng: &mut impl Rng, _index: IndexRef) -> Self {
        let name = NameGen::location(rng).generate_adlet();
        let entrance = wpos;

        let surface_radius: i32 = {
            let unit_size = rng.gen_range(10..12);
            let num_units = rng.gen_range(4..8);
            let variation = rng.gen_range(20..30);
            unit_size * num_units + variation
        };

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
            let num_units = rng.gen_range(5..8);
            let variation = rng.gen_range(20..40);
            unit_size * num_units + variation
        };

        let tunnel_length = rng.gen_range(35_i32..50);

        let cavern_center = entrance
            + (Vec2::new(angle.cos(), angle.sin()) * (tunnel_length as f32 + cavern_radius as f32))
                .as_();

        let cavern_alt = (land.get_alt_approx(cavern_center) - cavern_radius as f32)
            .min(land.get_alt_approx(entrance));

        let mut outer_structures = Vec::<(AdletStructure, Vec2<i32>, Dir)>::new();

        let entrance_dir = Dir::from_vector(entrance - cavern_center);
        outer_structures.push((AdletStructure::TunnelEntrance, Vec2::zero(), entrance_dir));

        let desired_structures = surface_radius.pow(2) / 100;
        for _ in 0..desired_structures {
            if let Some((rpos, kind)) = attempt(50, || {
                let structure_kind = AdletStructure::Igloo;
                /*
                // Choose structure kind
                let structure_kind = match rng.gen_range(0..10) {
                    // TODO: Add more variants
                    _ => AdletStructure::Igloo,
                };
                 */
                // Choose relative position
                let structure_center = {
                    let theta = rng.gen::<f32>() * TAU;
                    // 0.8 to keep structures not directly against wall
                    let radius = surface_radius as f32 * rng.gen::<f32>().sqrt() * 0.8;
                    let x = radius * theta.sin();
                    let y = radius * theta.cos();
                    Vec2::new(x, y).as_()
                };

                let tunnel_line = LineSegment2 {
                    start: entrance,
                    end: entrance - entrance_dir.to_vec2() * 100,
                };

                // Check that structure not in the water or too close to another structure
                if land
                    .get_chunk_wpos(structure_center.as_() + entrance)
                    .map_or(false, |c| c.is_underwater())
                    || outer_structures.iter().any(|(kind, rpos, _dir)| {
                        structure_center.distance_squared(*rpos)
                            < structure_kind.required_separation(kind).pow(2)
                    })
                    || tunnel_line
                        .as_::<f32>()
                        .distance_to_point((structure_center + entrance).as_::<f32>())
                        < 25.0
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
            structures: &[(AdletStructure, Vec2<i32>, Dir)],
            structure: AdletStructure,
            rpos: Vec2<i32>,
        ) -> bool {
            structures.iter().all(|(kind, rpos2, _dir)| {
                rpos.distance_squared(*rpos2) > structure.required_separation(kind).pow(2)
            })
        }

        // Add speleothem clusters (stalagmites/stalactites)
        let desired_speleothem_clusters = cavern_radius.pow(2) / 2500;
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
                let desired_adjacent_clusters = rng.gen_range(1..5);
                for _ in 0..desired_adjacent_clusters {
                    // Choose a relative position adjacent to initial speleothem cluster
                    let adj_rpos = {
                        let theta = rng.gen_range(0.0..TAU);
                        let radius = rng.gen_range(1.0..5.0);
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

        // Attempt to place central boss bone hut
        if let Some(rpos) = attempt(50, || {
            let rpos = {
                let theta = rng.gen_range(0.0..TAU);
                let radius = rng.gen::<f32>() * cavern_radius as f32 * 0.5;
                Vec2::new(theta.cos() * radius, theta.sin() * radius).as_()
            };
            valid_cavern_struct_pos(&cavern_structures, AdletStructure::BossBoneHut, rpos)
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
                valid_cavern_struct_pos(&cavern_structures, AdletStructure::BossBoneHut, rpos)
                    .then_some(rpos)
            })
        }) {
            // Direction doesn't matter for boss bonehut
            cavern_structures.push((AdletStructure::BossBoneHut, rpos, Dir::X));
        }

        // Attempt to place yetipit near the cavern edge
        if let Some(rpos) = attempt(50, || {
            let rpos = {
                let theta = rng.gen_range(0.0..TAU);
                let radius = cavern_radius as f32;
                Vec2::new(theta.cos() * radius, theta.sin() * radius).as_()
            };
            valid_cavern_struct_pos(&cavern_structures, AdletStructure::YetiPit, rpos)
                .then_some(rpos)
        })
        .or_else(|| {
            attempt(100, || {
                let rpos = {
                    let theta = rng.gen_range(0.0..TAU);
                    // If selecting a spot near the cavern edge failed, find a spot anywhere in the
                    // cavern
                    let radius = rng.gen::<f32>().sqrt() * cavern_radius as f32;
                    Vec2::new(theta.cos() * radius, theta.sin() * radius).as_()
                };
                valid_cavern_struct_pos(&cavern_structures, AdletStructure::YetiPit, rpos)
                    .then_some(rpos)
            })
        }) {
            // Direction doesn't matter for yetipit
            cavern_structures.push((AdletStructure::YetiPit, rpos, Dir::X));
        }

        // Attempt to place big bonfire
        if let Some(rpos) = attempt(50, || {
            let rpos = {
                let theta = rng.gen_range(0.0..TAU);
                let radius = rng.gen::<f32>().sqrt() * cavern_radius as f32 * 0.9;
                Vec2::new(theta.cos() * radius, theta.sin() * radius).as_()
            };
            valid_cavern_struct_pos(&cavern_structures, AdletStructure::Bonfire, rpos)
                .then_some(rpos)
        }) {
            // Direction doesn't matter for central bonfire
            cavern_structures.push((AdletStructure::Bonfire, rpos, Dir::X));
        }

        // Attempt to place some rock huts around the outer edge
        let desired_rock_huts = cavern_radius / 5;
        for _ in 0..desired_rock_huts {
            if let Some(rpos) = attempt(25, || {
                let rpos = {
                    let theta = rng.gen_range(0.0..TAU);
                    let radius = cavern_radius as f32 - 1.0;
                    Vec2::new(theta.cos() * radius, theta.sin() * radius).as_()
                };
                valid_cavern_struct_pos(&cavern_structures, AdletStructure::RockHut, rpos)
                    .then_some(rpos)
            }) {
                // Rock huts need no direction
                cavern_structures.push((AdletStructure::RockHut, rpos, Dir::X));
            }
        }

        // Attempt to place some general structures
        let desired_structures = cavern_radius.pow(2) / 200;
        for _ in 0..desired_structures {
            if let Some((structure, rpos)) = attempt(45, || {
                let rpos = {
                    let theta = rng.gen_range(0.0..TAU);
                    // sqrt biases radius away from center, leading to even distribution in circle
                    let radius = rng.gen::<f32>().sqrt() * cavern_radius as f32 * 0.9;
                    Vec2::new(theta.cos() * radius, theta.sin() * radius).as_()
                };
                let structure = match rng.gen_range(0..7) {
                    0..=2 => AdletStructure::BoneHut,
                    3..=4 => AdletStructure::CookFire,
                    5 => AdletStructure::Tannery,
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
            entrance,
            surface_radius,
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
        // Surface
        let size = (self.surface_radius * 5 / 4) / tile::TILE_SIZE as i32;
        let offset = (self.entrance - origin) / tile::TILE_SIZE as i32;
        let surface_aabr = Aabr {
            min: Vec2::broadcast(-size) + offset,
            max: Vec2::broadcast(size) + offset,
        };
        (cavern_aabr, surface_aabr)
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            waypoints: false,
            trees: wpos.distance_squared(self.entrance) > (self.surface_radius * 5 / 4).pow(2),
            ..SpawnRules::default()
        }
    }

    // TODO: Find a better way of spawning entities in site2
    pub fn apply_supplement(
        &self,
        // NOTE: Used only for dynamic elements like chests and entities!
        _dynamic_rng: &mut impl Rng,
        wpos2d: Vec2<i32>,
        _supplement: &mut ChunkSupplement,
    ) {
        let rpos = wpos2d - self.cavern_center;
        let _area = Aabr {
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
        let snow_ice_fill = Fill::Sampling(Arc::new(|wpos| {
            Some(match (RandomField::new(0).get(wpos)) % 250 {
                0..=2 => Block::new(BlockKind::Ice, Rgb::new(120, 160, 255)),
                3..=10 => Block::new(BlockKind::ArtSnow, Rgb::new(138, 147, 217)),
                11..=20 => Block::new(BlockKind::ArtSnow, Rgb::new(213, 213, 242)),
                21..=35 => Block::new(BlockKind::ArtSnow, Rgb::new(231, 230, 247)),
                36..=62 => Block::new(BlockKind::ArtSnow, Rgb::new(180, 181, 227)),
                _ => Block::new(BlockKind::ArtSnow, Rgb::new(209, 212, 238)),
            })
        }));
        let snow_ice_air_fill = Fill::Sampling(Arc::new(|wpos| {
            Some(match (RandomField::new(0).get(wpos)) % 250 {
                0..=2 => Block::new(BlockKind::Ice, Rgb::new(120, 160, 255)),
                3..=5 => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
                6..=10 => Block::new(BlockKind::ArtSnow, Rgb::new(138, 147, 217)),
                11..=20 => Block::new(BlockKind::ArtSnow, Rgb::new(213, 213, 242)),
                21..=35 => Block::new(BlockKind::ArtSnow, Rgb::new(231, 230, 247)),
                36..=62 => Block::new(BlockKind::ArtSnow, Rgb::new(180, 181, 227)),
                _ => Block::new(BlockKind::ArtSnow, Rgb::new(209, 212, 238)),
            })
        }));
        let bone_fill = Fill::Brick(BlockKind::Misc, Rgb::new(200, 160, 140), 1);
        let ice_fill = Fill::Block(Block::new(BlockKind::Ice, Rgb::new(120, 160, 255)));
        let dirt_fill = Fill::Brick(BlockKind::Earth, Rgb::new(55, 25, 8), 24);
        let grass_fill = Fill::Sampling(Arc::new(|wpos| {
            Some(match (RandomField::new(0).get(wpos)) % 5 {
                1 => Block::air(SpriteKind::ShortGrass),
                _ => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
            })
        }));
        let rock_fill = Fill::Sampling(Arc::new(|wpos| {
            Some(match (RandomField::new(0).get(wpos)) % 4 {
                0 => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
                _ => Block::new(BlockKind::Rock, Rgb::new(90, 110, 150)),
            })
        }));
        let bone_shrub = Fill::Sampling(Arc::new(|wpos| {
            Some(match (RandomField::new(0).get(wpos)) % 40 {
                0 => Block::new(BlockKind::Misc, Rgb::new(200, 160, 140)),
                _ => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
            })
        }));
        let yeti_sprites_fill = Fill::Sampling(Arc::new(|wpos| {
            Some(match (RandomField::new(0).get(wpos)) % 275 {
                0..=8 => Block::air(SpriteKind::Bones),
                9..=19 => Block::air(SpriteKind::GlowIceCrystal),
                20..=28 => Block::air(SpriteKind::IceCrystal),
                29..=30 => Block::air(SpriteKind::DungeonChest1),
                _ => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
            })
        }));
        let yeti_bones_fill = Fill::Sampling(Arc::new(|wpos| {
            Some(match (RandomField::new(0).get(wpos)) % 20 {
                0 => Block::air(SpriteKind::Bones),
                _ => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
            })
        }));
        let mut rng = thread_rng();

        // Tunnel
        let dist: f32 = self.cavern_center.as_().distance(self.entrance.as_());
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
        let raw_tunnel_end =
            ((self.cavern_center.as_() - self.entrance.as_()) * self.tunnel_length as f32 / dist)
                .with_z(self.cavern_alt - 1.0)
                + self.entrance.as_();

        let offset = 15.0;
        let tunnel_end = match dir {
            Dir::X => Vec3::new(raw_tunnel_end.x - offset, tunnel_start.y, raw_tunnel_end.z),
            Dir::Y => Vec3::new(tunnel_start.x, raw_tunnel_end.y - offset, raw_tunnel_end.z),
            Dir::NegX => Vec3::new(raw_tunnel_end.x + offset, tunnel_start.y, raw_tunnel_end.z),
            Dir::NegY => Vec3::new(tunnel_start.x, raw_tunnel_end.y + offset, raw_tunnel_end.z),
        };
        // Platform
        painter
            .sphere(Aabb {
                min: (self.entrance - 15).with_z(self.cavern_alt as i32 - 15),
                max: (self.entrance + 15).with_z(self.cavern_alt as i32 + 15),
            })
            .fill(snow_ice_fill.clone());

        painter
            .cylinder(Aabb {
                min: (self.entrance - 15).with_z(self.cavern_alt as i32),
                max: (self.entrance + 15).with_z(self.cavern_alt as i32 + 20),
            })
            .clear();
        painter
            .cylinder(Aabb {
                min: (self.entrance - 14).with_z(self.cavern_alt as i32 - 1),
                max: (self.entrance + 14).with_z(self.cavern_alt as i32),
            })
            .clear();
        painter
            .cylinder(Aabb {
                min: (self.entrance - 12).with_z(self.cavern_alt as i32 - 40),
                max: (self.entrance + 12).with_z(self.cavern_alt as i32 - 10),
            })
            .fill(snow_ice_air_fill.clone());

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
        // Ensure there is a path to the cave if the above is weird (e.g. when it is at
        // or near a 45 degrees angle)
        painter
            .line(
                tunnel_end.with_z(tunnel_end.z + 4.0),
                raw_tunnel_end.with_z(raw_tunnel_end.z + 4.0),
                4.0,
            )
            .clear();

        // Cavern
        let cavern = painter
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
            });
        let alt = self.cavern_alt;
        cavern.clear();

        // snow cylinder for cavern ground and to carve out yetipit
        painter
            .cylinder(Aabb {
                min: (self.cavern_center - self.cavern_radius).with_z(alt as i32 - 200),
                max: (self.cavern_center + self.cavern_radius).with_z(alt as i32),
            })
            .fill(snow_ice_fill.clone());

        for (structure, wpos, alt, dir) in self
            .outer_structures
            .iter()
            .map(|(structure, rpos, dir)| {
                let wpos = rpos + self.entrance;
                (structure, wpos, land.get_alt_approx(wpos), dir)
            })
            .chain(self.cavern_structures.iter().map(|(structure, rpos, dir)| {
                (structure, rpos + self.cavern_center, self.cavern_alt, dir)
            }))
        {
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
                AdletStructure::Igloo => {
                    let igloo_pos = wpos;
                    let igloo_size = 8.0;
                    let height_handle = 0;
                    let bones_size = igloo_size as i32;
                    painter
                        .cylinder_with_radius(
                            (igloo_pos).with_z(alt as i32 - 5 + height_handle),
                            11.0,
                            45.0,
                        )
                        .clear();
                    // Foundation
                    let foundation = match RandomField::new(0).get((wpos).with_z(alt as i32)) % 5 {
                        0 => painter
                            .sphere(Aabb {
                                min: (igloo_pos - 15).with_z(alt as i32 - 45 + height_handle),
                                max: (igloo_pos + 15).with_z(alt as i32 - 15 + height_handle),
                            })
                            .union(painter.sphere(Aabb {
                                min: (igloo_pos - 10).with_z(alt as i32 - 20 + height_handle),
                                max: (igloo_pos + 10).with_z(alt as i32 - 5 + height_handle),
                            })),
                        _ => painter
                            .sphere(Aabb {
                                min: (igloo_pos - 15).with_z(alt as i32 - 60 + height_handle),
                                max: (igloo_pos + 15).with_z(alt as i32 - 30 + height_handle),
                            })
                            .union(painter.cone(Aabb {
                                min: (igloo_pos - 15).with_z(alt as i32 - 45 + height_handle),
                                max: (igloo_pos + 15).with_z(alt as i32 + 8 + height_handle),
                            })),
                    };
                    foundation.fill(snow_ice_air_fill.clone());
                    foundation.intersect(cavern).clear();
                    // Platform
                    painter
                        .sphere(Aabb {
                            min: (igloo_pos - 13).with_z(alt as i32 - 11 + height_handle),
                            max: (igloo_pos + 13).with_z(alt as i32 + 11 + height_handle),
                        })
                        .fill(snow_ice_air_fill.clone());

                    painter
                        .cylinder(Aabb {
                            min: (igloo_pos - 13).with_z(alt as i32 - 4 + height_handle),
                            max: (igloo_pos + 13).with_z(alt as i32 + 16 + height_handle),
                        })
                        .clear();
                    // 2 igloo variants
                    match RandomField::new(0).get((igloo_pos).with_z(alt as i32)) % 4 {
                        0 => {
                            // clear room
                            painter
                                .sphere_with_radius(
                                    igloo_pos.with_z(alt as i32 - 1 + height_handle),
                                    (igloo_size as i32 - 2) as f32,
                                )
                                .clear();
                            let pos_var = RandomField::new(0).get(igloo_pos.with_z(alt as i32)) % 5;
                            let radius = 8 + pos_var;
                            let bones = 8.0 + pos_var as f32;
                            let phi = TAU / bones;
                            for n in 1..=bones as i32 {
                                let bone_hide_fill = Fill::Sampling(Arc::new(|pos| {
                                    Some(match (RandomField::new(0).get(pos)) % 35 {
                                        0 => Block::new(BlockKind::Wood, Rgb::new(73, 29, 0)),
                                        1 => Block::new(BlockKind::Wood, Rgb::new(78, 67, 43)),
                                        2 => Block::new(BlockKind::Wood, Rgb::new(83, 74, 41)),
                                        3 => Block::new(BlockKind::Wood, Rgb::new(14, 36, 34)),
                                        _ => Block::new(BlockKind::Misc, Rgb::new(200, 160, 140)),
                                    })
                                }));

                                let pos = Vec2::new(
                                    igloo_pos.x + (radius as f32 * ((n as f32 * phi).cos())) as i32,
                                    igloo_pos.y + (radius as f32 * ((n as f32 * phi).sin())) as i32,
                                );
                                let bone_var = RandomField::new(0).get(pos.with_z(alt as i32)) % 5;

                                match RandomField::new(0).get((igloo_pos - 1).with_z(alt as i32))
                                    % 3
                                {
                                    0 => {
                                        painter
                                            .line(
                                                pos.with_z(alt as i32 - 6 + height_handle),
                                                igloo_pos.with_z(alt as i32 + 8 + height_handle),
                                                1.0,
                                            )
                                            .fill(bone_hide_fill.clone());
                                    },
                                    _ => {
                                        painter
                                            .cubic_bezier(
                                                pos.with_z(alt as i32 - 6 + height_handle),
                                                (pos - ((igloo_pos - pos) / 2)).with_z(
                                                    alt as i32
                                                        + 12
                                                        + bone_var as i32
                                                        + height_handle,
                                                ),
                                                (pos + ((igloo_pos - pos) / 2))
                                                    .with_z(alt as i32 + 9 + height_handle),
                                                igloo_pos.with_z(alt as i32 + 5 + height_handle),
                                                1.0,
                                            )
                                            .fill(bone_hide_fill.clone());
                                    },
                                };
                            }
                            let outside_wolfs = 2
                                + (RandomField::new(0)
                                    .get((igloo_pos - 1).with_z(alt as i32 - 5 + height_handle))
                                    % 5) as i32;
                            for _ in 0..outside_wolfs {
                                let igloo_mob_spawn =
                                    (igloo_pos - 1).with_z(alt as i32 - 5 + height_handle);
                                painter.spawn(wolf(igloo_mob_spawn.as_(), &mut rng))
                            }
                        },
                        _ => {
                            // igloo snow
                            painter
                                .sphere_with_radius(igloo_pos.with_z(alt as i32 - 1), igloo_size)
                                .fill(snow_ice_fill.clone());
                            // 4 hide pieces
                            for dir in CARDINALS {
                                let hide_size = 5
                                    + (RandomField::new(0)
                                        .get((igloo_pos + dir).with_z(alt as i32))
                                        % 4);
                                let hide_color = match RandomField::new(0)
                                    .get((igloo_pos + dir).with_z(alt as i32))
                                    % 4
                                {
                                    0 => Fill::Block(Block::new(
                                        BlockKind::Wood,
                                        Rgb::new(73, 29, 0),
                                    )),
                                    1 => Fill::Block(Block::new(
                                        BlockKind::Wood,
                                        Rgb::new(78, 67, 43),
                                    )),
                                    2 => Fill::Block(Block::new(
                                        BlockKind::Wood,
                                        Rgb::new(83, 74, 41),
                                    )),
                                    _ => Fill::Block(Block::new(
                                        BlockKind::Wood,
                                        Rgb::new(14, 36, 34),
                                    )),
                                };
                                painter
                                    .sphere_with_radius(
                                        (igloo_pos + (2 * dir))
                                            .with_z((alt as i32) + 1 + height_handle),
                                        hide_size as f32,
                                    )
                                    .fill(hide_color.clone());
                            }
                            // clear room
                            painter
                                .sphere_with_radius(
                                    igloo_pos.with_z(alt as i32 - 1 + height_handle),
                                    (igloo_size as i32 - 2) as f32,
                                )
                                .clear();
                            // clear entries
                            painter
                                .aabb(Aabb {
                                    min: Vec2::new(
                                        igloo_pos.x - 1,
                                        igloo_pos.y - igloo_size as i32 - 2,
                                    )
                                    .with_z(alt as i32 - 4 + height_handle),
                                    max: Vec2::new(
                                        igloo_pos.x + 1,
                                        igloo_pos.y + igloo_size as i32 + 2,
                                    )
                                    .with_z(alt as i32 - 2 + height_handle),
                                })
                                .clear();
                            painter
                                .aabb(Aabb {
                                    min: Vec2::new(
                                        igloo_pos.x - igloo_size as i32 - 2,
                                        igloo_pos.y - 1,
                                    )
                                    .with_z(alt as i32 - 4 + height_handle),
                                    max: Vec2::new(
                                        igloo_pos.x + igloo_size as i32 + 2,
                                        igloo_pos.y + 1,
                                    )
                                    .with_z(alt as i32 - 2 + height_handle),
                                })
                                .clear();
                            // bones
                            for h in 0..(bones_size + 4) {
                                painter
                                    .line(
                                        (igloo_pos - bones_size)
                                            .with_z((alt as i32) - 5 + h + height_handle),
                                        (igloo_pos + bones_size)
                                            .with_z((alt as i32) - 5 + h + height_handle),
                                        0.5,
                                    )
                                    .intersect(painter.sphere_with_radius(
                                        igloo_pos.with_z((alt as i32) - 2 + height_handle),
                                        9.0,
                                    ))
                                    .fill(bone_fill.clone());

                                painter
                                    .line(
                                        Vec2::new(
                                            igloo_pos.x - bones_size,
                                            igloo_pos.y + bones_size,
                                        )
                                        .with_z((alt as i32) - 4 + h + height_handle),
                                        Vec2::new(
                                            igloo_pos.x + bones_size,
                                            igloo_pos.y - bones_size,
                                        )
                                        .with_z((alt as i32) - 4 + h + height_handle),
                                        0.5,
                                    )
                                    .intersect(painter.sphere_with_radius(
                                        igloo_pos.with_z((alt as i32) - 2 + height_handle),
                                        9.0,
                                    ))
                                    .fill(bone_fill.clone());
                            }
                            painter
                                .sphere_with_radius(
                                    igloo_pos.with_z((alt as i32) - 2 + height_handle),
                                    5.0,
                                )
                                .clear();

                            // WallSconce
                            painter.rotated_sprite(
                                Vec2::new(
                                    igloo_pos.x - bones_size + 4,
                                    igloo_pos.y + bones_size - 5,
                                )
                                .with_z((alt as i32) - 1 + height_handle),
                                SpriteKind::WallSconce,
                                0_u8,
                            );
                            let igloo_mobs = 1
                                + (RandomField::new(0)
                                    .get((igloo_pos - 1).with_z(alt as i32 - 5 + height_handle))
                                    % 2) as i32;

                            for _ in 0..igloo_mobs {
                                let igloo_mob_spawn =
                                    (igloo_pos - 1).with_z(alt as i32 - 5 + height_handle);
                                painter.spawn(random_adlet(igloo_mob_spawn.as_(), &mut rng));
                            }
                        },
                    };
                    // igloo floor
                    painter
                        .cylinder_with_radius(
                            (igloo_pos).with_z(alt as i32 - 7 + height_handle),
                            (igloo_size as i32 - 4) as f32,
                            2.0,
                        )
                        .fill(snow_ice_fill.clone());
                    // top decor bone with some hide
                    painter
                        .aabb(Aabb {
                            min: igloo_pos.with_z((alt as i32) + bones_size + height_handle),
                            max: (igloo_pos + 1)
                                .with_z((alt as i32) + bones_size + 3 + height_handle),
                        })
                        .fill(bone_fill.clone());
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(igloo_pos.x, igloo_pos.y - 1)
                                .with_z((alt as i32) + bones_size + 3 + height_handle),
                            max: Vec2::new(igloo_pos.x + 1, igloo_pos.y + 2)
                                .with_z((alt as i32) + bones_size + 4 + height_handle),
                        })
                        .fill(bone_fill.clone());
                    painter
                        .aabb(Aabb {
                            min: igloo_pos.with_z((alt as i32) + bones_size + 3 + height_handle),
                            max: (igloo_pos + 1)
                                .with_z((alt as i32) + bones_size + 4 + height_handle),
                        })
                        .clear();
                    let top_color = Fill::Sampling(Arc::new(|igloo_pos| {
                        Some(match (RandomField::new(0).get(igloo_pos)) % 10 {
                            0 => Block::new(BlockKind::Wood, Rgb::new(73, 29, 0)),
                            1 => Block::new(BlockKind::Wood, Rgb::new(78, 67, 43)),
                            2 => Block::new(BlockKind::Wood, Rgb::new(83, 74, 41)),
                            3 => Block::new(BlockKind::Wood, Rgb::new(14, 36, 34)),
                            _ => Block::new(BlockKind::Rock, Rgb::new(200, 160, 140)),
                        })
                    }));
                    painter
                        .aabb(Aabb {
                            min: (igloo_pos - 1).with_z((alt as i32) + bones_size + height_handle),
                            max: (igloo_pos + 2)
                                .with_z((alt as i32) + bones_size + 1 + height_handle),
                        })
                        .fill(top_color.clone());

                    // FireBowl
                    painter.sprite(
                        igloo_pos.with_z(alt as i32 - 5 + height_handle),
                        SpriteKind::FireBowlGround,
                    );
                },
                AdletStructure::SpeleothemCluster => {
                    let layer_color = Fill::Sampling(Arc::new(|wpos| {
                        Some(
                            match (RandomField::new(0).get(Vec3::new(wpos.z, 0, 0))) % 6 {
                                0 => Block::new(BlockKind::Rock, Rgb::new(100, 128, 179)),
                                1 => Block::new(BlockKind::Rock, Rgb::new(95, 127, 178)),
                                2 => Block::new(BlockKind::Rock, Rgb::new(101, 121, 169)),
                                3 => Block::new(BlockKind::Rock, Rgb::new(61, 109, 145)),
                                4 => Block::new(BlockKind::Rock, Rgb::new(74, 128, 168)),
                                _ => Block::new(BlockKind::Rock, Rgb::new(69, 123, 162)),
                            },
                        )
                    }));
                    for dir in NEIGHBORS {
                        let cone_radius = 3
                            + (RandomField::new(0).get((wpos + dir).with_z(alt as i32)) % 3) as i32;
                        let cone_offset = 3
                            + (RandomField::new(0).get((wpos + 1 + dir).with_z(alt as i32)) % 4)
                                as i32;
                        let cone_height = 15
                            + (RandomField::new(0).get((wpos + 2 + dir).with_z(alt as i32)) % 50)
                                as i32;
                        // cones
                        painter
                            .cone_with_radius(
                                (wpos + (dir * cone_offset)).with_z(alt as i32 - (cone_height / 8)),
                                cone_radius as f32,
                                cone_height as f32,
                            )
                            .fill(layer_color.clone());
                        // small cone tops
                        let top_pos = (RandomField::new(0).get((wpos + 3 + dir).with_z(alt as i32))
                            % 2) as i32;
                        painter
                            .aabb(Aabb {
                                min: (wpos + (dir * cone_offset) - top_pos).with_z(alt as i32),
                                max: (wpos + (dir * cone_offset) + 1 - top_pos)
                                    .with_z((alt as i32) + cone_height - (cone_height / 6)),
                            })
                            .fill(layer_color.clone());
                    }
                },
                AdletStructure::Bonfire => {
                    let bonfire_pos = wpos;
                    let fire_fill = Fill::Sampling(Arc::new(|bonfire_pos| {
                        Some(match (RandomField::new(0).get(bonfire_pos)) % 24 {
                            0 => Block::air(SpriteKind::Ember),
                            _ => Block::air(SpriteKind::FireBlock),
                        })
                    }));
                    let fire_pos = bonfire_pos.with_z(alt as i32 + 2);
                    lazy_static! {
                        pub static ref FIRE: AssetHandle<StructuresGroup> =
                            PrefabStructure::load_group("site_structures.adlet.bonfire");
                    }
                    let fire_rng = RandomField::new(0).get(fire_pos) % 10;
                    let fire = FIRE.read();
                    let fire = fire[fire_rng as usize % fire.len()].clone();
                    painter
                        .prim(Primitive::Prefab(Box::new(fire.clone())))
                        .translate(fire_pos)
                        .fill(Fill::Prefab(Box::new(fire), fire_pos, fire_rng));
                    painter
                        .sphere_with_radius((bonfire_pos).with_z(alt as i32 + 5), 4.0)
                        .fill(fire_fill.clone());
                    painter
                        .cylinder_with_radius((bonfire_pos).with_z(alt as i32 + 2), 6.5, 1.0)
                        .fill(fire_fill);
                },
                AdletStructure::YetiPit => {
                    let yetipit_center = self.cavern_center;
                    let yetipit_entrance_pos = wpos;

                    let storeys = (3 + RandomField::new(0).get((yetipit_center).with_z(alt as i32))
                        % 2) as i32;
                    for s in 0..storeys {
                        let down = 10_i32;
                        let level = (alt as i32) - 50 - (s * (3 * down));
                        let room_size = (25
                            + RandomField::new(0).get((yetipit_center * s).with_z(level)) % 5)
                            as i32;
                        if s == (storeys - 1) {
                            // yeti room
                            painter
                                .cylinder_with_radius(
                                    yetipit_center.with_z(level - (3 * down) - 5),
                                    room_size as f32,
                                    ((room_size / 3) + 5) as f32,
                                )
                                .clear();
                            painter
                                .cylinder_with_radius(
                                    yetipit_center.with_z(level - (3 * down) - 6),
                                    room_size as f32,
                                    1.0,
                                )
                                .fill(snow_ice_fill.clone());
                            // sprites: icecrystals, bones
                            for r in 0..4 {
                                painter
                                    .cylinder_with_radius(
                                        yetipit_center.with_z(level - (3 * down) - 2 - r),
                                        (room_size + 2 - r) as f32,
                                        1.0,
                                    )
                                    .fill(snow_ice_fill.clone());
                                painter
                                    .cylinder_with_radius(
                                        yetipit_center.with_z(level - (3 * down) - 1 - r),
                                        (room_size - r) as f32,
                                        1.0,
                                    )
                                    .fill(yeti_sprites_fill.clone());
                                painter
                                    .cylinder_with_radius(
                                        yetipit_center.with_z(level - (3 * down) - 2 - r),
                                        (room_size - 1 - r) as f32,
                                        2.0,
                                    )
                                    .clear();
                            }
                            painter
                                .cylinder_with_radius(
                                    yetipit_center.with_z(level - (3 * down) - 5),
                                    (room_size - 4) as f32,
                                    1.0,
                                )
                                .fill(yeti_bones_fill.clone());
                            painter
                                .cone_with_radius(
                                    yetipit_center.with_z(level - (3 * down) + (room_size / 3) - 2),
                                    room_size as f32,
                                    (room_size / 3) as f32,
                                )
                                .clear();
                            // snow covered speleothem cluster
                            for dir in NEIGHBORS {
                                let cluster_pos = yetipit_center + dir * room_size - 3;
                                for dir in NEIGHBORS3 {
                                    let cone_radius = 3
                                        + (RandomField::new(0)
                                            .get((cluster_pos + dir).with_z(alt as i32))
                                            % 3) as i32;
                                    let cone_offset = 3
                                        + (RandomField::new(0)
                                            .get((cluster_pos + 1 + dir).with_z(alt as i32))
                                            % 4) as i32;
                                    let cone_height = 15
                                        + (RandomField::new(0)
                                            .get((cluster_pos + 2 + dir).with_z(alt as i32))
                                            % 10) as i32;
                                    // cones
                                    painter
                                        .cone_with_radius(
                                            (cluster_pos + (dir * cone_offset))
                                                .with_z(level - (3 * down) - 4 - (cone_height / 8)),
                                            cone_radius as f32,
                                            cone_height as f32,
                                        )
                                        .fill(snow_ice_fill.clone());
                                    // small cone tops
                                    let top_pos = (RandomField::new(0)
                                        .get((cluster_pos + 3 + dir).with_z(level))
                                        % 2)
                                        as i32;
                                    painter
                                        .aabb(Aabb {
                                            min: (cluster_pos + (dir * cone_offset) - top_pos)
                                                .with_z(level - (3 * down) - 3),
                                            max: (cluster_pos + (dir * cone_offset) + 1 - top_pos)
                                                .with_z(
                                                    (level - (3 * down) - 2) + cone_height
                                                        - (cone_height / 6)
                                                        + 3,
                                                ),
                                        })
                                        .fill(snow_ice_fill.clone());
                                }
                            }
                            // ceiling snow covered speleothem cluster
                            for dir in NEIGHBORS {
                                for c in 0..8 {
                                    let cluster_pos = yetipit_center + dir * (c * (room_size / 5));
                                    for dir in NEIGHBORS3 {
                                        let cone_radius = 3
                                            + (RandomField::new(0)
                                                .get((cluster_pos + dir).with_z(alt as i32))
                                                % 3)
                                                as i32;
                                        let cone_offset = 3
                                            + (RandomField::new(0)
                                                .get((cluster_pos + 1 + dir).with_z(alt as i32))
                                                % 4)
                                                as i32;
                                        let cone_height = 15
                                            + (RandomField::new(0)
                                                .get((cluster_pos + 2 + dir).with_z(alt as i32))
                                                % 10)
                                                as i32;
                                        // cones
                                        painter
                                            .cone_with_radius(
                                                (cluster_pos + (dir * cone_offset)).with_z(
                                                    level - (3 * down) - 4 - (cone_height / 8),
                                                ),
                                                cone_radius as f32,
                                                (cone_height - 1) as f32,
                                            )
                                            .rotate_about(
                                                Mat3::rotation_x(PI).as_(),
                                                yetipit_center.with_z(level - (2 * down) - 1),
                                            )
                                            .fill(snow_ice_fill.clone());
                                        // small cone tops
                                        let top_pos = (RandomField::new(0)
                                            .get((cluster_pos + 3 + dir).with_z(level))
                                            % 2)
                                            as i32;
                                        painter
                                            .aabb(Aabb {
                                                min: (cluster_pos + (dir * cone_offset) - top_pos)
                                                    .with_z(level - (3 * down) - 3),
                                                max: (cluster_pos + (dir * cone_offset) + 1
                                                    - top_pos)
                                                    .with_z(
                                                        (level - (3 * down) - 2) + cone_height
                                                            - (cone_height / 6)
                                                            - 2,
                                                    ),
                                            })
                                            .rotate_about(
                                                Mat3::rotation_x(PI).as_(),
                                                yetipit_center.with_z(level - (2 * down)),
                                            )
                                            .fill(snow_ice_fill.clone());
                                    }
                                }
                            }
                            // frozen ponds
                            for dir in NEIGHBORS3 {
                                let pond_radius = (RandomField::new(0)
                                    .get((yetipit_center + dir).with_z(alt as i32))
                                    % 8) as i32;
                                let pond_pos =
                                    yetipit_center + (dir * ((room_size / 4) + (pond_radius)));
                                painter
                                    .cylinder_with_radius(
                                        pond_pos.with_z(level - (3 * down) - 6),
                                        pond_radius as f32,
                                        1.0,
                                    )
                                    .fill(ice_fill.clone());
                            }
                            // yeti
                            let yeti_spawn = yetipit_center.with_z(level - (3 * down) - 4);
                            painter.spawn(yeti(yeti_spawn.as_(), &mut rng));
                        } else {
                            // mob rooms
                            painter
                                .cylinder_with_radius(
                                    yetipit_center.with_z(level - (3 * down) - 5),
                                    room_size as f32,
                                    ((room_size / 3) + 5) as f32,
                                )
                                .clear();
                            // sprites: icecrystals, bones
                            for r in 0..4 {
                                painter
                                    .cylinder_with_radius(
                                        yetipit_center.with_z(level - (3 * down) - 2 - r),
                                        (room_size + 2 - r) as f32,
                                        1.0,
                                    )
                                    .fill(snow_ice_fill.clone());
                                painter
                                    .cylinder_with_radius(
                                        yetipit_center.with_z(level - (3 * down) - 1 - r),
                                        (room_size - r) as f32,
                                        1.0,
                                    )
                                    .fill(yeti_sprites_fill.clone());
                                painter
                                    .cylinder_with_radius(
                                        yetipit_center.with_z(level - (3 * down) - 2 - r),
                                        (room_size - 1 - r) as f32,
                                        2.0,
                                    )
                                    .clear();
                            }
                            let yetipit_mobs = 1
                                + (RandomField::new(0)
                                    .get(yetipit_center.with_z(level - (3 * down) - 3))
                                    % 2) as i32;
                            for _ in 0..yetipit_mobs {
                                let yetipit_mob_spawn =
                                    yetipit_center.with_z(level - (3 * down) - 3);
                                painter
                                    .spawn(random_yetipit_mob(yetipit_mob_spawn.as_(), &mut rng));
                            }
                            painter
                                .cone_with_radius(
                                    yetipit_center.with_z(level - (3 * down) + (room_size / 3) - 2),
                                    room_size as f32,
                                    (room_size / 3) as f32,
                                )
                                .clear();
                            // snow covered speleothem cluster
                            for dir in NEIGHBORS {
                                let cluster_pos = yetipit_center + dir * room_size;
                                for dir in NEIGHBORS {
                                    let cone_radius = 3
                                        + (RandomField::new(0)
                                            .get((cluster_pos + dir).with_z(alt as i32))
                                            % 3) as i32;
                                    let cone_offset = 3
                                        + (RandomField::new(0)
                                            .get((cluster_pos + 1 + dir).with_z(alt as i32))
                                            % 4) as i32;
                                    let cone_height = 15
                                        + (RandomField::new(0)
                                            .get((cluster_pos + 2 + dir).with_z(alt as i32))
                                            % 10) as i32;
                                    // cones
                                    painter
                                        .cone_with_radius(
                                            (cluster_pos + (dir * cone_offset))
                                                .with_z(level - (3 * down) - 4 - (cone_height / 8)),
                                            cone_radius as f32,
                                            cone_height as f32,
                                        )
                                        .fill(snow_ice_fill.clone());
                                    // small cone tops
                                    let top_pos = (RandomField::new(0)
                                        .get((cluster_pos + 3 + dir).with_z(level))
                                        % 2)
                                        as i32;
                                    painter
                                        .aabb(Aabb {
                                            min: (cluster_pos + (dir * cone_offset) - top_pos)
                                                .with_z(level - (3 * down) - 3),
                                            max: (cluster_pos + (dir * cone_offset) + 1 - top_pos)
                                                .with_z(
                                                    (level - (3 * down) - 2) + cone_height
                                                        - (cone_height / 6)
                                                        + 3,
                                                ),
                                        })
                                        .fill(snow_ice_fill.clone());
                                }
                            }
                            // ceiling snow covered speleothem cluster
                            for dir in NEIGHBORS {
                                for c in 0..5 {
                                    let cluster_pos = yetipit_center + dir * (c * (room_size / 3));
                                    for dir in NEIGHBORS {
                                        let cone_radius = 3
                                            + (RandomField::new(0)
                                                .get((cluster_pos + dir).with_z(alt as i32))
                                                % 3)
                                                as i32;
                                        let cone_offset = 3
                                            + (RandomField::new(0)
                                                .get((cluster_pos + 1 + dir).with_z(alt as i32))
                                                % 4)
                                                as i32;
                                        let cone_height = 15
                                            + (RandomField::new(0)
                                                .get((cluster_pos + 2 + dir).with_z(alt as i32))
                                                % 10)
                                                as i32;
                                        // cones
                                        painter
                                            .cone_with_radius(
                                                (cluster_pos + (dir * cone_offset)).with_z(
                                                    level - (3 * down) - 4 - (cone_height / 8),
                                                ),
                                                cone_radius as f32,
                                                (cone_height - 1) as f32,
                                            )
                                            .rotate_about(
                                                Mat3::rotation_x(PI).as_(),
                                                yetipit_center.with_z(level - (2 * down) - 1),
                                            )
                                            .fill(snow_ice_fill.clone());
                                        // small cone tops
                                        let top_pos = (RandomField::new(0)
                                            .get((cluster_pos + 3 + dir).with_z(level))
                                            % 2)
                                            as i32;
                                        painter
                                            .aabb(Aabb {
                                                min: (cluster_pos + (dir * cone_offset) - top_pos)
                                                    .with_z(level - (3 * down) - 3),
                                                max: (cluster_pos + (dir * cone_offset) + 1
                                                    - top_pos)
                                                    .with_z(
                                                        (level - (3 * down) - 2) + cone_height
                                                            - (cone_height / 6)
                                                            - 2,
                                                    ),
                                            })
                                            .rotate_about(
                                                Mat3::rotation_x(PI).as_(),
                                                yetipit_center.with_z(level - (2 * down)),
                                            )
                                            .fill(snow_ice_fill.clone());
                                    }
                                }
                            }
                            // frozen pond
                            painter
                                .cylinder_with_radius(
                                    yetipit_center.with_z(level - (3 * down) - 4),
                                    (room_size / 8) as f32,
                                    1.0,
                                )
                                .fill(ice_fill.clone());
                        }
                        let tunnels = (2 + RandomField::new(0)
                            .get((yetipit_center + s).with_z(level))
                            % 2) as i32;
                        for t in 1..tunnels {
                            let away1 = (50
                                + RandomField::new(0).get((yetipit_center * (s + t)).with_z(level))
                                    % 20) as i32;
                            let away2 = (50
                                + RandomField::new(0)
                                    .get((yetipit_center * (s + (2 * t))).with_z(level))
                                    % 20) as i32;
                            let away3 = (50
                                + RandomField::new(0)
                                    .get((yetipit_center * (s + (3 * t))).with_z(level))
                                    % 20) as i32;
                            let away4 = (50
                                + RandomField::new(0)
                                    .get((yetipit_center * (s + (4 * t))).with_z(level))
                                    % 20) as i32;

                            let dir1 = 1 - 2
                                * (RandomField::new(0).get((yetipit_center).with_z(t * level)) % 2)
                                    as i32;
                            let dir2 = 1 - 2
                                * (RandomField::new(0)
                                    .get((yetipit_center).with_z((2 * t) * level))
                                    % 2) as i32;
                            // caves
                            painter
                                .cubic_bezier(
                                    yetipit_center.with_z(level - 3),
                                    Vec2::new(
                                        yetipit_center.x + ((away1 + (s * (down / 4))) * dir1),
                                        yetipit_center.y + ((away2 + (s * (down / 4))) * dir2),
                                    )
                                    .with_z(level - down),
                                    Vec2::new(
                                        yetipit_center.x + ((away3 + (s * (down / 4))) * dir1),
                                        yetipit_center.y + ((away4 + (s * (down / 4))) * dir2),
                                    )
                                    .with_z(level - (2 * down)),
                                    yetipit_center.with_z(level - (3 * down)),
                                    6.0,
                                )
                                .clear();
                        }
                    }
                    // yetipit entrance
                    // rocks
                    painter
                        .sphere(Aabb {
                            min: (yetipit_entrance_pos - 8).with_z(alt as i32 - 8),
                            max: (yetipit_entrance_pos + 8).with_z(alt as i32 + 8),
                        })
                        .fill(rock_fill.clone());
                    // repaint ground
                    painter
                        .cylinder(Aabb {
                            min: (yetipit_entrance_pos - 8).with_z(alt as i32 - 20),
                            max: (yetipit_entrance_pos + 8).with_z(alt as i32),
                        })
                        .fill(snow_ice_fill.clone());
                    // tunnel
                    let door_dist = self.cavern_center - yetipit_entrance_pos;
                    let door_dir = door_dist
                        / Vec2::new((door_dist.x).pow(2).sqrt(), (door_dist.y).pow(2).sqrt());
                    painter
                        .cubic_bezier(
                            (yetipit_entrance_pos + door_dir * 10).with_z(alt as i32 + 2),
                            (yetipit_entrance_pos - door_dir * 16).with_z(alt as i32 - 10),
                            (yetipit_entrance_pos + door_dir * 20).with_z((alt as i32) - 30),
                            self.cavern_center.with_z((alt as i32) - 50),
                            4.0,
                        )
                        .clear();
                    // bone door
                    painter
                        .cylinder(Aabb {
                            min: Vec2::new(yetipit_entrance_pos.x - 7, yetipit_entrance_pos.y - 7)
                                .with_z(alt as i32 - 8),
                            max: Vec2::new(yetipit_entrance_pos.x + 7, yetipit_entrance_pos.y + 7)
                                .with_z((alt as i32) - 7),
                        })
                        .fill(snow_ice_fill.clone());

                    painter
                        .cylinder(Aabb {
                            min: Vec2::new(yetipit_entrance_pos.x - 3, yetipit_entrance_pos.y - 3)
                                .with_z(alt as i32 - 8),
                            max: Vec2::new(yetipit_entrance_pos.x + 3, yetipit_entrance_pos.y + 3)
                                .with_z((alt as i32) - 7),
                        })
                        .fill(Fill::Block(Block::air(SpriteKind::BoneKeyDoor)));
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(yetipit_entrance_pos.x - 1, yetipit_entrance_pos.y)
                                .with_z(alt as i32 - 8),
                            max: Vec2::new(yetipit_entrance_pos.x, yetipit_entrance_pos.y + 1)
                                .with_z((alt as i32) - 7),
                        })
                        .fill(Fill::Block(Block::air(SpriteKind::BoneKeyhole)));
                },
                AdletStructure::Tannery => {
                    // shattered bone pieces
                    painter
                        .cylinder_with_radius(wpos.with_z(alt as i32), 7.0, 1.0)
                        .fill(bone_shrub.clone());
                    // bones upright
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 6, wpos.y).with_z(alt as i32),
                            max: Vec2::new(wpos.x + 6, wpos.y + 1).with_z((alt as i32) + 8),
                        })
                        .fill(bone_fill.clone());
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 5, wpos.y).with_z(alt as i32),
                            max: Vec2::new(wpos.x + 5, wpos.y + 1).with_z((alt as i32) + 8),
                        })
                        .clear();
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 6, wpos.y - 1).with_z(alt as i32 + 8),
                            max: Vec2::new(wpos.x + 6, wpos.y + 2).with_z((alt as i32) + 9),
                        })
                        .fill(bone_fill.clone());
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 5, wpos.y - 1).with_z(alt as i32 + 8),
                            max: Vec2::new(wpos.x + 5, wpos.y + 2).with_z((alt as i32) + 9),
                        })
                        .clear();
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 6, wpos.y).with_z(alt as i32 + 8),
                            max: Vec2::new(wpos.x + 6, wpos.y + 1).with_z((alt as i32) + 9),
                        })
                        .clear();
                    // bones lying
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 6, wpos.y + 3).with_z(alt as i32),
                            max: Vec2::new(wpos.x + 6, wpos.y + 4).with_z((alt as i32) + 2),
                        })
                        .fill(bone_fill.clone());
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 5, wpos.y + 3).with_z(alt as i32),
                            max: Vec2::new(wpos.x - 3, wpos.y + 4).with_z((alt as i32) + 1),
                        })
                        .clear();
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x + 3, wpos.y + 3).with_z(alt as i32),
                            max: Vec2::new(wpos.x + 5, wpos.y + 4).with_z((alt as i32) + 1),
                        })
                        .clear();
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 2, wpos.y + 3).with_z(alt as i32),
                            max: Vec2::new(wpos.x + 2, wpos.y + 4).with_z((alt as i32) + 1),
                        })
                        .clear();
                    // hide
                    for n in 0..10 {
                        let hide_color =
                            match RandomField::new(0).get((wpos + n).with_z(alt as i32)) % 4 {
                                0 => Fill::Block(Block::new(BlockKind::Wood, Rgb::new(73, 29, 0))),
                                1 => Fill::Block(Block::new(BlockKind::Wood, Rgb::new(78, 67, 43))),
                                2 => Fill::Block(Block::new(BlockKind::Wood, Rgb::new(83, 74, 41))),
                                _ => Fill::Block(Block::new(BlockKind::Wood, Rgb::new(14, 36, 34))),
                            };
                        let rand_length =
                            (RandomField::new(0).get((wpos - n).with_z(alt as i32)) % 7) as i32;
                        painter
                            .aabb(Aabb {
                                min: Vec2::new(wpos.x - 5, wpos.y).with_z(alt as i32 + rand_length),
                                max: Vec2::new(wpos.x - 4 + n, wpos.y + 1).with_z((alt as i32) + 8),
                            })
                            .fill(hide_color.clone());
                    }
                    let tannery_mobs =
                        1 + (RandomField::new(0).get(wpos.with_z(alt as i32)) % 2) as i32;
                    for _ in 0..tannery_mobs {
                        let tannery_mob_spawn = wpos.with_z(alt as i32);
                        painter.spawn(random_adlet(tannery_mob_spawn.as_(), &mut rng));
                    }
                },
                AdletStructure::AnimalPen => {
                    let pen_size = 8.0;
                    painter
                        .sphere_with_radius(
                            wpos.with_z(alt as i32 + (pen_size as i32 / 4) - 1),
                            pen_size as f32,
                        )
                        .fill(bone_fill.clone());
                    painter
                        .sphere_with_radius(
                            wpos.with_z(alt as i32 + (pen_size as i32 / 2) - 1),
                            pen_size as f32,
                        )
                        .clear();
                    painter
                        .cylinder(Aabb {
                            min: (wpos - (pen_size as i32)).with_z(alt as i32 - (pen_size as i32)),
                            max: (wpos + (pen_size as i32)).with_z(alt as i32),
                        })
                        .fill(dirt_fill.clone());
                    painter
                        .cylinder(Aabb {
                            min: (wpos - (pen_size as i32) + 1).with_z(alt as i32),
                            max: (wpos + (pen_size as i32) - 1).with_z(alt as i32 + 1),
                        })
                        .fill(grass_fill.clone());
                    enum AnimalPenKind {
                        Rat,
                        Wolf,
                        Bear,
                    }
                    let (kind, num) = {
                        let rand_field = RandomField::new(1).get(wpos.with_z(alt as i32));
                        match RandomField::new(0).get(wpos.with_z(alt as i32)) % 4 {
                            0 => (AnimalPenKind::Bear, 1 + rand_field % 2),
                            1 => (AnimalPenKind::Wolf, 2 + rand_field % 3),
                            _ => (AnimalPenKind::Rat, 5 + rand_field % 5),
                        }
                    };
                    for _ in 0..num {
                        let animalpen_mob_spawn = wpos.with_z(alt as i32);
                        match kind {
                            AnimalPenKind::Rat => {
                                painter.spawn(rat(animalpen_mob_spawn.as_(), &mut rng))
                            },
                            AnimalPenKind::Wolf => {
                                painter.spawn(wolf(animalpen_mob_spawn.as_(), &mut rng))
                            },
                            AnimalPenKind::Bear => {
                                painter.spawn(bear(animalpen_mob_spawn.as_(), &mut rng))
                            },
                        }
                    }
                },
                AdletStructure::CookFire => {
                    painter
                        .cylinder(Aabb {
                            min: (wpos - 3).with_z(alt as i32),
                            max: (wpos + 4).with_z(alt as i32 + 1),
                        })
                        .fill(bone_fill.clone());
                    let cook_sprites = Fill::Sampling(Arc::new(|wpos| {
                        Some(match (RandomField::new(0).get(wpos)) % 120 {
                            0..=5 => Block::air(SpriteKind::Pot),
                            6..=10 => Block::air(SpriteKind::Bowl),
                            11..=15 => Block::air(SpriteKind::Pot),
                            16..=20 => Block::air(SpriteKind::VialEmpty),
                            21..=30 => Block::air(SpriteKind::Lantern),
                            31..=32 => Block::air(SpriteKind::DungeonChest1),
                            _ => Block::air(SpriteKind::Empty),
                        })
                    }));
                    painter
                        .cylinder(Aabb {
                            min: (wpos - 3).with_z(alt as i32 + 1),
                            max: (wpos + 4).with_z(alt as i32 + 2),
                        })
                        .fill(cook_sprites);
                    painter
                        .cylinder(Aabb {
                            min: (wpos - 2).with_z(alt as i32),
                            max: (wpos + 3).with_z(alt as i32 + 2),
                        })
                        .clear();
                    painter
                        .aabb(Aabb {
                            min: (wpos).with_z(alt as i32),
                            max: (wpos + 1).with_z(alt as i32 + 1),
                        })
                        .fill(bone_fill.clone());
                    painter.sprite(wpos.with_z(alt as i32 + 1), SpriteKind::FireBowlGround);
                    let cookfire_mobs =
                        1 + (RandomField::new(0).get(wpos.with_z(alt as i32)) % 2) as i32;
                    for _ in 0..cookfire_mobs {
                        let cookfire_mob_spawn = wpos.with_z(alt as i32);
                        painter.spawn(random_adlet(cookfire_mob_spawn.as_(), &mut rng));
                    }
                },
                AdletStructure::RockHut => {
                    painter
                        .sphere_with_radius(wpos.with_z(alt as i32), 5.0)
                        .fill(rock_fill.clone());
                    painter
                        .sphere_with_radius(wpos.with_z(alt as i32), 4.0)
                        .clear();
                    // clear entries
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 6, wpos.y - 1).with_z(alt as i32),
                            max: Vec2::new(wpos.x + 6, wpos.y + 1).with_z(alt as i32 + 2),
                        })
                        .clear();
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 1, wpos.y - 6).with_z(alt as i32),
                            max: Vec2::new(wpos.x + 1, wpos.y + 6).with_z(alt as i32 + 2),
                        })
                        .clear();
                    // fill with dirt
                    painter
                        .cylinder(Aabb {
                            min: (wpos - 5).with_z((alt as i32) - 5),
                            max: (wpos + 5).with_z(alt as i32 - 1),
                        })
                        .fill(dirt_fill.clone());
                    painter.sprite(wpos.with_z(alt as i32) - 1, SpriteKind::FireBowlGround);
                    let rockhut_mobs =
                        1 + (RandomField::new(0).get(wpos.with_z(alt as i32)) % 2) as i32;
                    for _ in 0..rockhut_mobs {
                        let rockhut_mob_spawn = wpos.with_z(alt as i32);
                        painter.spawn(random_adlet(rockhut_mob_spawn.as_(), &mut rng));
                    }
                },
                AdletStructure::BoneHut => {
                    let hut_radius = 5;
                    // 4 hide pieces
                    for dir in CARDINALS {
                        let hide_size =
                            6 + (RandomField::new(0).get((wpos + dir).with_z(alt as i32)) % 2);
                        let hide_color =
                            match RandomField::new(0).get((wpos + dir).with_z(alt as i32)) % 4 {
                                0 => Fill::Block(Block::new(BlockKind::Wood, Rgb::new(73, 29, 0))),
                                1 => Fill::Block(Block::new(BlockKind::Wood, Rgb::new(78, 67, 43))),
                                2 => Fill::Block(Block::new(BlockKind::Wood, Rgb::new(83, 74, 41))),
                                _ => Fill::Block(Block::new(BlockKind::Wood, Rgb::new(14, 36, 34))),
                            };
                        painter
                            .sphere_with_radius(
                                (wpos + (2 * dir)).with_z((alt as i32) + 2),
                                hide_size as f32,
                            )
                            .fill(hide_color.clone());
                    }
                    // clear room
                    painter
                        .sphere_with_radius(wpos.with_z((alt as i32) + 2), 6.0)
                        .intersect(painter.aabb(Aabb {
                            min: (wpos - 6).with_z(alt as i32),
                            max: (wpos + 6).with_z(alt as i32 + 2 * hut_radius),
                        }))
                        .clear();
                    //clear entries
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x - 1, wpos.y - hut_radius - 6).with_z(alt as i32),
                            max: Vec2::new(wpos.x + 1, wpos.y + hut_radius + 6)
                                .with_z((alt as i32) + 3),
                        })
                        .clear();

                    // bones
                    for h in 0..(hut_radius + 4) {
                        painter
                            .line(
                                (wpos - hut_radius).with_z((alt as i32) + h),
                                (wpos + hut_radius).with_z((alt as i32) + h),
                                0.5,
                            )
                            .intersect(
                                painter.sphere_with_radius(wpos.with_z((alt as i32) + 2), 9.0),
                            )
                            .fill(bone_fill.clone());

                        painter
                            .line(
                                Vec2::new(wpos.x - hut_radius, wpos.y + hut_radius)
                                    .with_z((alt as i32) + h),
                                Vec2::new(wpos.x + hut_radius, wpos.y - hut_radius)
                                    .with_z((alt as i32) + h),
                                0.5,
                            )
                            .intersect(
                                painter.sphere_with_radius(wpos.with_z((alt as i32) + 2), 9.0),
                            )
                            .fill(bone_fill.clone());
                    }
                    painter
                        .sphere_with_radius(wpos.with_z((alt as i32) + 2), 5.0)
                        .intersect(painter.aabb(Aabb {
                            min: (wpos - 5).with_z(alt as i32),
                            max: (wpos + 5).with_z((alt as i32) + 2 * hut_radius),
                        }))
                        .clear();

                    // top decor bone with some hide
                    painter
                        .aabb(Aabb {
                            min: wpos.with_z((alt as i32) + hut_radius + 4),
                            max: (wpos + 1).with_z((alt as i32) + hut_radius + 7),
                        })
                        .fill(bone_fill.clone());
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(wpos.x, wpos.y - 1)
                                .with_z((alt as i32) + hut_radius + 7),
                            max: Vec2::new(wpos.x + 1, wpos.y + 2)
                                .with_z((alt as i32) + hut_radius + 8),
                        })
                        .fill(bone_fill.clone());
                    painter
                        .aabb(Aabb {
                            min: wpos.with_z((alt as i32) + hut_radius + 7),
                            max: (wpos + 1).with_z((alt as i32) + hut_radius + 8),
                        })
                        .clear();
                    let top_color = Fill::Sampling(Arc::new(|wpos| {
                        Some(match (RandomField::new(0).get(wpos)) % 10 {
                            0 => Block::new(BlockKind::Wood, Rgb::new(73, 29, 0)),
                            1 => Block::new(BlockKind::Wood, Rgb::new(78, 67, 43)),
                            2 => Block::new(BlockKind::Wood, Rgb::new(83, 74, 41)),
                            3 => Block::new(BlockKind::Wood, Rgb::new(14, 36, 34)),
                            _ => Block::new(BlockKind::Rock, Rgb::new(200, 160, 140)),
                        })
                    }));
                    painter
                        .aabb(Aabb {
                            min: (wpos - 1).with_z((alt as i32) + hut_radius + 4),
                            max: (wpos + 2).with_z((alt as i32) + hut_radius + 5),
                        })
                        .fill(top_color.clone());
                    // WallSconce
                    painter.rotated_sprite(
                        Vec2::new(wpos.x - hut_radius + 1, wpos.y + hut_radius - 2)
                            .with_z((alt as i32) + 3),
                        SpriteKind::WallSconce,
                        0_u8,
                    );
                    // FireBowl
                    painter.sprite(wpos.with_z(alt as i32), SpriteKind::FireBowlGround);
                    let bonehut_mobs =
                        1 + (RandomField::new(0).get(wpos.with_z(alt as i32)) % 2) as i32;
                    for _ in 0..bonehut_mobs {
                        let bonehut_mob_spawn = wpos.with_z(alt as i32);
                        painter.spawn(random_adlet(bonehut_mob_spawn.as_(), &mut rng));
                    }
                },
                AdletStructure::BossBoneHut => {
                    let bosshut_pos = wpos;
                    let hut_radius = 10;
                    // 4 hide pieces
                    for dir in CARDINALS {
                        let hide_size = 10
                            + (RandomField::new(0).get((bosshut_pos + dir).with_z(alt as i32)) % 4);
                        let hide_color = match RandomField::new(0)
                            .get((bosshut_pos + dir).with_z(alt as i32))
                            % 4
                        {
                            0 => Fill::Block(Block::new(BlockKind::Wood, Rgb::new(73, 29, 0))),
                            1 => Fill::Block(Block::new(BlockKind::Wood, Rgb::new(78, 67, 43))),
                            2 => Fill::Block(Block::new(BlockKind::Wood, Rgb::new(83, 74, 41))),
                            _ => Fill::Block(Block::new(BlockKind::Wood, Rgb::new(14, 36, 34))),
                        };
                        painter
                            .sphere_with_radius(
                                (bosshut_pos + (3 * dir)).with_z((alt as i32) + 2),
                                hide_size as f32,
                            )
                            .fill(hide_color.clone());
                    }
                    // bones
                    for h in 0..(hut_radius + 4) {
                        painter
                            .line(
                                (bosshut_pos - hut_radius + 1).with_z((alt as i32) + h),
                                (bosshut_pos + hut_radius - 1).with_z((alt as i32) + h),
                                1.5,
                            )
                            .intersect(
                                painter
                                    .sphere_with_radius(bosshut_pos.with_z((alt as i32) + 2), 14.0),
                            )
                            .intersect(
                                painter.aabb(Aabb {
                                    min: (bosshut_pos - 2 * hut_radius).with_z(alt as i32),
                                    max: (bosshut_pos + 2 * hut_radius)
                                        .with_z((alt as i32) + 2 * hut_radius),
                                }),
                            )
                            .fill(bone_fill.clone());

                        painter
                            .line(
                                Vec2::new(
                                    bosshut_pos.x - hut_radius + 1,
                                    bosshut_pos.y + hut_radius - 2,
                                )
                                .with_z((alt as i32) + h),
                                Vec2::new(
                                    bosshut_pos.x + hut_radius - 1,
                                    bosshut_pos.y - hut_radius + 2,
                                )
                                .with_z((alt as i32) + h),
                                1.5,
                            )
                            .intersect(
                                painter
                                    .sphere_with_radius(bosshut_pos.with_z((alt as i32) + 2), 14.0),
                            )
                            .intersect(
                                painter.aabb(Aabb {
                                    min: (bosshut_pos - 2 * hut_radius).with_z(alt as i32),
                                    max: (bosshut_pos + 2 * hut_radius)
                                        .with_z((alt as i32) + 2 * hut_radius),
                                }),
                            )
                            .fill(bone_fill.clone());
                    }
                    painter
                        .sphere_with_radius(bosshut_pos.with_z((alt as i32) + 2), 9.0)
                        .intersect(painter.aabb(Aabb {
                            min: (bosshut_pos - 9).with_z(alt as i32),
                            max: (bosshut_pos + 9).with_z(alt as i32 + 11),
                        }))
                        .clear();

                    for n in 0..2 {
                        // large entries

                        painter
                            .sphere_with_radius(
                                (Vec2::new(
                                    bosshut_pos.x,
                                    bosshut_pos.y - hut_radius + (2 * (hut_radius * n)),
                                ))
                                .with_z((alt as i32) + 2),
                                7.0,
                            )
                            .intersect(
                                painter.aabb(Aabb {
                                    min: Vec2::new(
                                        bosshut_pos.x - 7,
                                        bosshut_pos.y - hut_radius + (2 * (hut_radius * n) - 7),
                                    )
                                    .with_z(alt as i32),
                                    max: Vec2::new(
                                        bosshut_pos.x + 7,
                                        bosshut_pos.y - hut_radius + (2 * (hut_radius * n) + 7),
                                    )
                                    .with_z(alt as i32 + 9),
                                }),
                            )
                            .clear();
                        let entry_start = Vec2::new(
                            bosshut_pos.x - hut_radius + 3,
                            bosshut_pos.y - hut_radius - 2 + (n * ((2 * hut_radius) + 4)),
                        )
                        .with_z(alt as i32);
                        let entry_peak = Vec2::new(
                            bosshut_pos.x,
                            bosshut_pos.y - hut_radius + (n * (2 * hut_radius)),
                        )
                        .with_z(alt as i32 + hut_radius + 2);
                        let entry_end = Vec2::new(
                            bosshut_pos.x + hut_radius - 3,
                            bosshut_pos.y - hut_radius - 2 + (n * ((2 * hut_radius) + 4)),
                        )
                        .with_z(alt as i32);
                        painter
                            .cubic_bezier(entry_start, entry_peak, entry_peak, entry_end, 1.0)
                            .fill(bone_fill.clone());
                    }

                    // top decor bone with some hide
                    painter
                        .aabb(Aabb {
                            min: bosshut_pos.with_z((alt as i32) + hut_radius + 5),
                            max: (bosshut_pos + 1).with_z((alt as i32) + hut_radius + 8),
                        })
                        .fill(bone_fill.clone());
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(bosshut_pos.x, bosshut_pos.y - 1)
                                .with_z((alt as i32) + hut_radius + 8),
                            max: Vec2::new(bosshut_pos.x + 1, bosshut_pos.y + 2)
                                .with_z((alt as i32) + hut_radius + 9),
                        })
                        .fill(bone_fill.clone());
                    painter
                        .aabb(Aabb {
                            min: bosshut_pos.with_z((alt as i32) + hut_radius + 8),
                            max: (bosshut_pos + 1).with_z((alt as i32) + hut_radius + 9),
                        })
                        .clear();

                    let top_color = Fill::Sampling(Arc::new(|bosshut_pos| {
                        Some(match (RandomField::new(0).get(bosshut_pos)) % 10 {
                            0 => Block::new(BlockKind::Wood, Rgb::new(73, 29, 0)),
                            1 => Block::new(BlockKind::Wood, Rgb::new(78, 67, 43)),
                            2 => Block::new(BlockKind::Wood, Rgb::new(83, 74, 41)),
                            3 => Block::new(BlockKind::Wood, Rgb::new(14, 36, 34)),
                            _ => Block::new(BlockKind::Rock, Rgb::new(200, 160, 140)),
                        })
                    }));
                    painter
                        .aabb(Aabb {
                            min: (bosshut_pos - 1).with_z((alt as i32) + hut_radius + 5),
                            max: (bosshut_pos + 2).with_z((alt as i32) + hut_radius + 6),
                        })
                        .fill(top_color);
                    // WallSconces
                    for dir in SQUARE_4 {
                        let corner_pos = Vec2::new(
                            bosshut_pos.x - (hut_radius / 2) - 1,
                            bosshut_pos.y - (hut_radius / 2) - 2,
                        );
                        let sprite_pos = Vec2::new(
                            corner_pos.x + dir.x * (hut_radius + 1),
                            corner_pos.y + dir.y * (hut_radius + 3),
                        )
                        .with_z(alt as i32 + 3);
                        painter.rotated_sprite(
                            sprite_pos,
                            SpriteKind::WallSconce,
                            (2 + (4 * dir.x)) as u8,
                        );
                    }
                    let boss_spawn = wpos.with_z(alt as i32);
                    painter.spawn(adlet_elder(boss_spawn.as_(), &mut rng));
                },
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

        let rotation_origin = Vec3::new(spine_start.x, spine_start.y + 0.5, spine_start.z);
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

fn adlet_hunter<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.adlet.hunter", rng)
}

fn adlet_icepicker<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.adlet.icepicker", rng)
}

fn adlet_tracker<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.adlet.tracker", rng)
}

fn random_adlet<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    match rng.gen_range(0..3) {
        0 => adlet_hunter(pos, rng),
        1 => adlet_icepicker(pos, rng),
        _ => adlet_tracker(pos, rng),
    }
}

fn adlet_elder<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.dungeon.adlet.elder", rng)
}

fn rat<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32)).with_asset_expect("common.entity.wild.peaceful.rat", rng)
}

fn wolf<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.wild.aggressive.wolf", rng)
}

fn bear<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.wild.aggressive.bear", rng)
}

fn frostfang<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.wild.aggressive.frostfang", rng)
}

fn roshwalr<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.wild.aggressive.roshwalr", rng)
}

fn icedrake<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.wild.aggressive.icedrake", rng)
}

fn tursus<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32))
        .with_asset_expect("common.entity.wild.aggressive.tursus", rng)
}

fn random_yetipit_mob<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    match rng.gen_range(0..4) {
        0 => frostfang(pos, rng),
        1 => roshwalr(pos, rng),
        2 => icedrake(pos, rng),
        _ => tursus(pos, rng),
    }
}

fn yeti<R: Rng>(pos: Vec3<i32>, rng: &mut R) -> EntityInfo {
    EntityInfo::at(pos.map(|x| x as f32)).with_asset_expect("common.entity.dungeon.adlet.yeti", rng)
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
