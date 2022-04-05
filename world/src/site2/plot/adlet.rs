use super::*;
use crate::{
    assets::AssetHandle,
    site2::{gen::PrimitiveTransform, util::Dir},
    util::{attempt, sampler::Sampler, FastNoise, RandomField},
    Land, CanvasInfo,
};
use common::{
    generation::{ChunkSupplement, EntityInfo},
    terrain::{Structure as PrefabStructure, StructuresGroup},
};
use lazy_static::lazy_static;
use rand::prelude::*;
use std::{
    collections::HashMap,
    f32::consts::TAU,
    ops::{Add, Div, Mul, Sub},
};
use vek::*;

const ANGLE_SAMPLES: usize = 360;

pub struct AdletStronghold {
    name: String,
    seed: u32,
    entrance: Vec2<i32>,
    wall_center: Vec2<i32>,
    wall_radius: i32,
    wall_alt: f32,
    wall_alt_sample_positions: [[Vec2<i32>; 3]; ANGLE_SAMPLES],
    tunnel_length: i32,
    cavern_center: Vec2<i32>,
    cavern_alt: f32,
    cavern_radius: i32,
}

enum AdletStructure {
    Igloo,
}

impl AdletStructure {
    fn required_separation(&self, other: &Self) -> i32 {
        let radius = |structure: &Self| match structure {
            Self::Igloo => 10,
        };

        let additional_padding = match (self, other) {
            (Self::Igloo, Self::Igloo) => 50,
            _ => 0,
        };

        radius(self) + radius(other) + additional_padding
    }
}

impl AdletStronghold {
    pub fn generate(wpos: Vec2<i32>, land: &Land, rng: &mut impl Rng) -> Self {
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

        let mut wall_alt_sample_positions = [[Vec2::zero(); 3]; ANGLE_SAMPLES];
        for i in 0..ANGLE_SAMPLES {
            let theta = i as f32 / ANGLE_SAMPLES as f32 * TAU;
            // let sample_rpos = Vec2::new(
            //     theta.cos() * wall_radius as f32,
            //     theta.sin() * wall_radius as f32,
            // );
            // wall_alt_sample_positions[i] = sample_rpos.as_() + wall_center;
            for j in 0..3 {
                let sample_rpos = Vec2::new(
                    theta.cos() * (wall_radius as f32 + j as f32 - 1.0),
                    theta.sin() * (wall_radius as f32 + j as f32 - 1.0),
                );
                wall_alt_sample_positions[i][j] = sample_rpos.as_() + wall_center;
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

        let cavern_radius = {
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

        Self {
            name,
            seed,
            entrance,
            wall_center,
            wall_radius,
            wall_alt,
            wall_alt_sample_positions,
            tunnel_length,
            cavern_center,
            cavern_radius,
            cavern_alt,
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
        const WALL_DELTA: f32 = 4.0;
        // let mut wall_alt_samples = self.wall_alt_sample_positions.map(|pos| canvas.col(pos).map_or(land.get_alt_approx(pos), |col| col.alt));
        let mut wall_alt_samples = self.wall_alt_sample_positions.map(|poses| {
            let mut average = 0.0;
            for pos in poses.iter() {
                average += canvas.col(*pos).map_or(land.get_alt_approx(*pos), |col| col.alt);
            }
            average / 3.0
        });
        loop {
            let mut changed = false;
            for i in 0..wall_alt_samples.len() {
                let tmp = (wall_alt_samples[(i + 1) % ANGLE_SAMPLES] - WALL_DELTA).max(wall_alt_samples[(i + ANGLE_SAMPLES - 1) % ANGLE_SAMPLES] - WALL_DELTA);
                if tmp > wall_alt_samples[i] {
                    wall_alt_samples[i] = tmp;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
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
                let wall_center = self.wall_center;
                let theta = move |pos: Vec2<i32>| {
                    let rpos: Vec2<f32> = (pos - wall_center).as_();
                    let theta = rpos.y.atan2(rpos.x);
                    if theta > 0.0 {
                        theta
                    } else {
                        theta + TAU
                    }
                };
                move |pos, col| {
                    let index = (theta(pos.xy()) * ANGLE_SAMPLES as f32 / TAU).floor().max(0.0) as usize % ANGLE_SAMPLES;
                    (col.alt.sub(10.0)..wall_alt_samples[index].add(10.0).div(WALL_DELTA).floor().mul(WALL_DELTA))
                        .contains(&(pos.z as f32))
                }
            })
            .fill(wall_mat);

        // Tunnel
        let dist: f32 = self.cavern_center.as_().distance(self.entrance.as_());
        let tunnel_radius = 5.0;
        let tunnel_start = self
            .entrance
            .as_()
            .with_z(land.get_alt_approx(self.entrance));
        // Adds cavern radius to ensure that tunnel fully bores into cavern
        let tunnel_end =
            ((self.cavern_center.as_() - self.entrance.as_()) * self.tunnel_length as f32 / dist)
                .with_z(self.cavern_alt + tunnel_radius - 1.0)
                + self.entrance.as_();
        painter
            .line(tunnel_start, tunnel_end, tunnel_radius)
            .clear();
        painter
            .line(
                tunnel_end,
                self.cavern_center
                    .as_()
                    .with_z(self.cavern_alt + tunnel_radius),
                tunnel_radius,
            )
            .clear();
        painter
            .sphere_with_radius(
                self.entrance
                    .with_z(land.get_alt_approx(self.entrance) as i32 + 4),
                8.0,
            )
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
