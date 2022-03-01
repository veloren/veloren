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
use lazy_static::lazy_static;
use rand::prelude::*;
use std::{collections::HashMap, f32::consts::TAU};
use vek::*;

pub struct AdletStronghold {
    name: String,
    seed: u32,
    origin: Vec2<i32>,
    cavern_alt: f32,
    cavern_radius: i32,
    entrance: Vec2<i32>,
    tunnel_length: i32,
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

        let tunnel_length = rng.gen_range(25_i32..40);

        let origin = entrance
            + (Vec2::new(angle.cos(), angle.sin()) * (tunnel_length as f32 + cavern_radius as f32))
                .as_();

        // Go 50% below minimum height needed, unless entrance already below that height
        // TODO: Get better heuristic for this
        let cavern_alt = (land.get_alt_approx(origin) - cavern_radius as f32 * 1.5)
            .min(land.get_alt_approx(entrance));

        Self {
            name,
            seed,
            origin,
            cavern_radius,
            cavern_alt,
            entrance,
            tunnel_length,
        }
    }

    pub fn name(&self) -> &str { &self.name }

    pub fn origin(&self) -> Vec2<i32> { self.origin }

    pub fn radius(&self) -> i32 { self.cavern_radius + self.tunnel_length + 5 }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            waypoints: false,
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
        let rpos = wpos2d - self.origin;
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
        // Tunnel
        let dist: f32 = self.origin.as_().distance(self.entrance.as_());
        let tunnel_start = self
            .entrance
            .as_()
            .with_z(land.get_alt_approx(self.entrance));
        let tunnel_end = ((self.origin.as_() - self.entrance.as_()) * self.tunnel_length as f32
            / dist)
            .with_z(self.cavern_alt)
            + self.entrance.as_();
        painter.line(tunnel_start, tunnel_end, 5.0).clear();

        // Cavern
        painter
            .sphere_with_radius(
                self.origin.with_z(self.cavern_alt as i32),
                self.cavern_radius as f32,
            )
            .intersect(painter.aabb(Aabb {
                min: (self.origin - self.cavern_radius).with_z(self.cavern_alt as i32),
                max: self.origin.with_z(self.cavern_alt as i32) + self.cavern_radius,
            }))
            .clear();

        // Create outer wall
        // for (point, next_point) in self.wall_segments.iter() {
        //     // This adds additional points for the wall on the line between
        // two points,     // allowing the wall to better handle slopes
        //     const SECTIONS_PER_WALL_SEGMENT: usize = 3;

        //     (0..(SECTIONS_PER_WALL_SEGMENT as i32))
        //         .into_iter()
        //         .map(move |a| {
        //             let get_point =
        //                 |a| point + (next_point - point) * a /
        // (SECTIONS_PER_WALL_SEGMENT as i32);
        // (get_point(a), get_point(a + 1))         })
        //         .for_each(|(point, next_point)| {
        //             // 2d world positions of each point in wall segment
        //             let point = point;
        //             let start_wpos = point + self.origin;
        //             let end_wpos = next_point + self.origin;

        //             let lightstone = Fill::Brick(BlockKind::Wood,
        // Rgb::new(107, 107, 107), 18);             let midstone =
        // Fill::Brick(BlockKind::Wood, Rgb::new(70, 70, 70), 18);
        //             let darkstone = Fill::Brick(BlockKind::Wood, Rgb::new(42,
        // 42, 42), 18);             let darkpelt =
        // Fill::Brick(BlockKind::Wood, Rgb::new(80, 47, 13), 35);
        //             let lightpelt = Fill::Brick(BlockKind::Wood, Rgb::new(54,
        // 25, 1), 25);

        //             let start = (start_wpos + 2)
        //                 .as_()
        //                 .with_z(land.get_alt_approx(start_wpos) + 0.0);
        //             let end = (end_wpos + 2)
        //                 .as_()
        //                 .with_z(land.get_alt_approx(end_wpos) + 0.0);
        //             let randstart = start % 10.0 - 5.;
        //             let randend = end % 10.0 - 5.0;
        //             let mid = (start + end) / 2.0;

        //             let start =
        // start_wpos.as_().with_z(land.get_alt_approx(start_wpos));
        //             let end =
        // end_wpos.as_().with_z(land.get_alt_approx(end_wpos));

        //             let wall_base_height = 3.0;
        //             let wall_mid_thickness = 2.0;
        //             let wall_mid_height = 15.0 + wall_base_height;

        //             let highwall =
        //                 painter.segment_prism(start, end, wall_mid_thickness,
        // wall_mid_height);             painter.fill(highwall,
        // lightstone.clone());

        //             painter
        //                 .segment_prism(start, end, wall_mid_thickness + 1.0,
        // wall_mid_height - 8.0)
        // .fill(midstone.clone());             let wallexterior =
        // painter                 .segment_prism(start, end,
        // wall_mid_thickness + 2.0, wall_mid_height - 15.0)
        // .translate(Vec3::new(0, 0, 8));             let wallstrut =
        // painter                 .segment_prism(start, end,
        // wall_mid_thickness - 1.0, wall_mid_height)
        // .translate(Vec3::new(0, 0, 8));             let
        // wallexteriorlow = wallexterior.translate(Vec3::new(0, 0, -2));
        //             painter.fill(wallexterior, darkpelt.clone());
        //             painter.fill(wallexteriorlow, lightpelt.clone());

        //             let exclusion = painter.line(
        //                 Vec3::new(
        //                     start.x as i32,
        //                     start.y as i32,
        //                     start.z as i32 + wall_mid_height as i32 + 2,
        //                 ),
        //                 Vec3::new(
        //                     end.x as i32,
        //                     end.y as i32,
        //                     end.z as i32 + wall_mid_height as i32 + 2,
        //                 ),
        //                 5.0,
        //             );

        //             let top = painter
        //                 .line(
        //                     Vec3::new(
        //                         start.x as i32,
        //                         start.y as i32,
        //                         start.z as i32 + wall_mid_height as i32 + 5,
        //                     ),
        //                     Vec3::new(
        //                         end.x as i32,
        //                         end.y as i32,
        //                         end.z as i32 + wall_mid_height as i32 + 5,
        //                     ),
        //                     4.0,
        //                 )
        //                 .without(exclusion);

        //             let overtop = top.translate(Vec3::new(0, 0, 1));

        //             let cyl = painter.cylinder_with_radius(
        //                 Vec3::new(start.x as i32, start.y as i32, start.z as
        // i32),                 5.0,
        //                 wall_mid_height - 8.0,
        //             );
        //             painter.fill(cyl, darkstone.clone());
        //             let cylsupport = painter.cylinder_with_radius(
        //                 Vec3::new(start.x as i32, start.y as i32, start.z as
        // i32),                 5.0,
        //                 wall_mid_height + 20.0,
        //             );
        //             let cylsub = painter.cylinder_with_radius(
        //                 Vec3::new(start.x as i32, start.y as i32, start.z as
        // i32),                 4.0,
        //                 wall_mid_height + 20.0,
        //             );
        //             let hollowcyl = cylsupport.without(cylsub);
        //             let roofpoles = wallstrut.intersect(hollowcyl);
        //             painter.fill(roofpoles, midstone.clone());

        //             painter.fill(top, darkpelt.clone());

        //             let startshift =
        //                 Vec3::new(randstart.x * 3.0, randstart.y * 3.0,
        // randstart.z * 1.0);             let endshift =
        // Vec3::new(randend.x * 3.0, randend.y * 3.0, randend.z * 1.0);

        //             painter
        //                 .cubic_bezier(start, mid + startshift, mid +
        // endshift, end, 2.5)                 .translate(Vec3::new(0,
        // 0, 27))                 .intersect(overtop)
        //                 .fill(lightpelt.clone());

        //             painter
        //                 .sphere_with_radius(
        //                     Vec3::new(
        //                         start.x as i32,
        //                         start.y as i32,
        //                         start.z as i32 + wall_mid_height as i32 - 8,
        //                     ),
        //                     5.0,
        //                 )
        //                 .without(cyl)
        //                 .clear();
        //         })
        // }
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
