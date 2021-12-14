use super::*;
use crate::Land;
use inline_tweak::tweak;
use rand::prelude::*;
use vek::*;

pub struct GnarlingFortification {
    name: String,
    seed: u32,
    origin: Vec2<i32>,
    radius: i32,
    ordered_wall_points: Vec<Vec2<i32>>,
}

impl GnarlingFortification {
    pub fn generate(wpos: Vec2<i32>, land: &Land, rng: &mut impl Rng) -> Self {
        let name = String::from("Gnarling Fortification");
        let seed = rng.gen();
        let origin = wpos;

        let radius = {
            let unit_size = rng.gen_range(10..20);
            let num_units = rng.gen_range(5..10);
            let variation = rng.gen_range(0..50);
            unit_size * num_units + variation
        };

        let num_points = (radius / 15).max(4);
        let ordered_wall_points = (0..num_points)
            .into_iter()
            .map(|a| {
                let angle = a as f32 / num_points as f32 * core::f32::consts::TAU;
                Vec2::new(angle.cos(), angle.sin()).map(|a| (a * radius as f32) as i32)
            })
            .map(|point| {
                point.map(|a| {
                    let variation = radius / 5;
                    a + rng.gen_range(-variation..=variation)
                })
            })
            .collect::<Vec<_>>();
        let ordered_wall_points = ordered_wall_points
            .iter()
            .enumerate()
            .flat_map(|(i, point)| {
                let next_point = if let Some(point) = ordered_wall_points.get(i + 1) {
                    *point
                } else {
                    ordered_wall_points[0]
                };
                let mid_point_1 = point + (next_point - point) / 3;
                let mid_point_2 = point + (next_point - point) * 2 / 3;
                [*point, mid_point_1, mid_point_2]
            })
            .collect::<Vec<_>>();

        Self {
            name,
            seed,
            origin,
            radius,
            ordered_wall_points,
        }
    }

    pub fn name(&self) -> &str { &self.name }

    pub fn radius(&self) -> i32 { self.radius }
}

impl Structure for GnarlingFortification {
    fn render(&self, _site: &Site, land: &Land, painter: &Painter) {
        // Create outer wall
        for (i, point) in self.ordered_wall_points.iter().enumerate() {
            // Other point of wall segment
            let next_point = if let Some(point) = self.ordered_wall_points.get(i + 1) {
                *point
            } else {
                self.ordered_wall_points[0]
            };
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

            let wall_base_segment = LineSegment3 { start, end };
            let wall_base_thickness = 3.0;
            let wall_base_height = 3.0;

            painter.fill(
                painter.prim(Primitive::SegmentPrism(
                    wall_base_segment,
                    wall_base_thickness,
                    wall_base_height + wall_depth as f32,
                )),
                Fill::Block(Block::new(BlockKind::Wood, Rgb::new(55, 25, 8))),
            );

            // Middle of wall
            let start = start_wpos.as_().with_z(land.get_alt_approx(start_wpos));
            let end = end_wpos.as_().with_z(land.get_alt_approx(end_wpos));

            let wall_mid_segment = LineSegment3 { start, end };
            let wall_mid_thickness = 1.0;
            let wall_mid_height = 5.0 + wall_base_height;

            painter.fill(
                painter.prim(Primitive::SegmentPrism(
                    wall_mid_segment,
                    wall_mid_thickness,
                    wall_mid_height,
                )),
                Fill::Block(Block::new(BlockKind::Wood, Rgb::new(55, 25, 8))),
            );

            // Top of wall
            let start = start_wpos
                .as_()
                .with_z(land.get_alt_approx(start_wpos) + wall_mid_height);
            let end = end_wpos
                .as_()
                .with_z(land.get_alt_approx(end_wpos) + wall_mid_height);

            let wall_top_segment = LineSegment3 { start, end };
            let wall_top_thickness = 2.0;
            let wall_top_height = 1.0;

            painter.fill(
                painter.prim(Primitive::SegmentPrism(
                    wall_top_segment,
                    wall_top_thickness,
                    wall_top_height,
                )),
                Fill::Block(Block::new(BlockKind::Wood, Rgb::new(55, 25, 8))),
            );

            // Wall parapets
            let start = Vec3::new(
                point.x as f32 * (self.radius as f32 + 1.0) / (self.radius as f32)
                    + self.origin.x as f32,
                point.y as f32 * (self.radius as f32 + 1.0) / (self.radius as f32)
                    + self.origin.y as f32,
                land.get_alt_approx(start_wpos) + wall_mid_height + wall_top_height - 1.0,
            );
            let end = Vec3::new(
                next_point.x as f32 * (self.radius as f32 + 1.0) / (self.radius as f32)
                    + self.origin.x as f32,
                next_point.y as f32 * (self.radius as f32 + 1.0) / (self.radius as f32)
                    + self.origin.y as f32,
                land.get_alt_approx(end_wpos) + wall_mid_height + wall_top_height - 1.0,
            );

            let wall_par_segment = LineSegment3 { start, end };
            let wall_par_thickness = tweak!(0.8);
            let wall_par_height = 1.0;

            painter.fill(
                painter.prim(Primitive::SegmentPrism(
                    wall_par_segment,
                    wall_par_thickness,
                    wall_par_height + 1.0,
                )),
                Fill::Block(Block::new(BlockKind::Wood, Rgb::new(55, 25, 8))),
            );
        }
    }
}
