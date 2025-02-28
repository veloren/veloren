use crate::{
    civ::airship_travel::Airships,
    Index, IndexRef, CONFIG,
    sim::{
        sample_pos, sample_wpos,
        get_horizon_map,
    },    
    sim::WorldSim,
};
use common::terrain::{
    map::{MapConfig, MapSample, MapSizeLg},
    uniform_idx_as_vec2,
};
use delaunator::{Point, Triangulation};
use image::{DynamicImage, GenericImage, ImageEncoder, codecs::png::PngEncoder};
use line_drawing::{XiaolinWu, BresenhamCircle};
use std::{
    env,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use tracing::error;
use vek::*;

fn basic_world_map(image_size: MapSizeLg, index: &Index, sampler: &WorldSim) -> DynamicImage {
    let horizons = get_horizon_map(
        image_size,
        Aabr {
            min: Vec2::zero(),
            max: image_size.chunks().map(|e| e as i32),
        },
        CONFIG.sea_level,
        CONFIG.sea_level + sampler.max_height,
        |posi| {
            let sample = sampler.get(uniform_idx_as_vec2(image_size, posi)).unwrap();

            sample.basement.max(sample.water_alt)
        },
        |a| a,
        |h| h,
    )
    .ok();

    let colors = index.colors();
    let features = index.features();
    let index_ref = IndexRef {
        colors: &colors,
        features: &features,
        index,
    };

    let mut map_config = MapConfig::orthographic(image_size, 0.0..=sampler.max_height);
    map_config.horizons = horizons.as_ref();
    map_config.is_shaded = true;
    map_config.is_stylized_topo = true;
    let map = sampler.get_map(index_ref, None);
    let map_w = image_size.chunks().x as f32;
    let map_h = image_size.chunks().y as f32;
    let mut image = DynamicImage::new(
        map_w as u32,
        map_h as u32,
        image::ColorType::Rgba8,
    );

    map_config.generate(
        |pos| {
            let default_sample = sample_pos(&map_config, sampler, index_ref, None, pos);
            let [r, g, b, _a] = map.rgba[pos].to_le_bytes();

            MapSample {
                rgb: Rgb::new(r, g, b),
                ..default_sample
            }
        },
        |wpos| sample_wpos(&map_config, sampler, wpos),
        |pos, (r, g, b, a)| {
            image.put_pixel(
                pos.x as u32,
                image_size.chunks().y as u32 - pos.y as u32 - 1,
                [r, g, b, a].into(),
            )
        },
    );

    image
}

fn airship_routes_map(airships: &mut Airships, image_size: MapSizeLg, index: &Index, sampler: &WorldSim) -> DynamicImage {
    let mut image = basic_world_map(image_size, index, sampler);
    let world_chunks = sampler.map_size_lg().chunks();
    let world_blocks = world_chunks.map(|u| u as f32) * 32.0;
    let map_w = image_size.chunks().x as f32;
    let map_h = image_size.chunks().y as f32;

    // Draw route lines

    // colors
    let route_r = 0u8;
    let route_g = 255u8;
    let route_b = 255u8;
    let site_r = 105u8;
    let site_g = 231u8;
    let site_b = 255u8;

    // route coordinates are in world blocks, convert to map pixels and invert y axis
    for route in airships.routes.values() {
        let dock_centers = [
            Vec2::new(
                route.approaches[0].dock_center.x / world_blocks.x * map_w,
                map_h - route.approaches[0].dock_center.y / world_blocks.y * map_h
        )   ,
            Vec2::new(
                route.approaches[1].dock_center.x / world_blocks.x * map_w,
                map_h - route.approaches[1].dock_center.y / world_blocks.y * map_h
            )
        ];

        // Draw a circle around the dock centers
        for dock_center in dock_centers.iter() {
            for (x, y) in BresenhamCircle::new(dock_center.x as i32, dock_center.y as i32, 10) {
                if x < 0 || y < 0 || x >= map_w as i32 || y >= map_h as i32 {
                    continue;
                }
                image.put_pixel(x as u32, y as u32, [site_r, site_g, site_b, 255].into());
            }
        }

        // calculate where the route_line intersects a circle of radius 20 around each dock center
        let route_dir = (dock_centers[1] - dock_centers[0]).normalized();
        let endpoints = [
            dock_centers[0] + route_dir * 10.0,
            dock_centers[1] - route_dir * 10.0,
        ];

        // Draw a line between the endpoints that intersect the circles
        for ((x, y), value) in XiaolinWu::<f32, i64>::new((endpoints[0].x, endpoints[0].y), (endpoints[1].x, endpoints[1].y)) {
            image.put_pixel(
                x as u32,
                y as u32,
                [route_r, route_g, route_b, (value * 255.0) as u8].into(),
            );
        }
    }

    image
}

fn dock_sites_triangulation_map(triangulation: &Triangulation, points: &Vec<Point>, image_size: MapSizeLg, index: &Index, sampler: &WorldSim) -> DynamicImage {
    let mut image = basic_world_map(image_size, index, sampler);
    let world_chunks = sampler.map_size_lg().chunks();
    let world_blocks = world_chunks.map(|u| u as f32) * 32.0;
    let map_w = image_size.chunks().x as f32;
    let map_h = image_size.chunks().y as f32;

    // Draw triangles

    // colors
    let route_r = 0u8;
    let route_g = 255u8;
    let route_b = 255u8;
    let site_r = 105u8;
    let site_g = 231u8;
    let site_b = 255u8;

    // coordinates are in world blocks, convert to map pixels and invert y axis
    macro_rules! map_triangle_points {
        ($vec:expr) => {
            Vec2 {
                x: $vec.x as f32,
                y: $vec.y as f32,
            }
        };
    }

    macro_rules! flip_y {
        ($vec:expr) => {
            Vec2 {
                x: $vec.x,
                y: map_h - $vec.y,
            }
        };
    }
    
    // the triangles are triplets in a Vec<usize> so we need to iterate over them in groups of 3.
    // The macros are used to convert the points from world blocks to map pixels and flip the y axis.
    // map_triangles is a Vec of arrays of 3 Vec2s representing the 3 points of each triangle.
    let map_triangles = triangulation.triangles.chunks(3).map(|triangle| {
        [
            flip_y!(map_triangle_points!(points[triangle[0]]) / world_blocks * map_w),
            flip_y!(map_triangle_points!(points[triangle[1]]) / world_blocks * map_w),
            flip_y!(map_triangle_points!(points[triangle[2]]) / world_blocks * map_w),
        ]
    })
    .collect::<Vec<_>>();

    for triangle in map_triangles.iter() {
        // triangle is an array of 3 Vec2<f32> representing the 3 points of the triangle.
        
        // Draw a circle around the points
        for p in triangle.iter() {
            for (x, y) in BresenhamCircle::new(p.x as i32, p.y as i32, 10) {
                if x < 0 || y < 0 || x >= map_w as i32 || y >= map_h as i32 {
                    continue;
                }
                image.put_pixel(x as u32, y as u32, [site_r, site_g, site_b, 255].into());
            }
        }
        
        // Now draw the triangle lines
        for i in 0..3 {
            let p1 = triangle[i];
            let p2 = triangle[(i + 1) % 3];
            // calculate where the triangle edge intersects a circle of radius 10 around each point
            let dir = (p2 - p1).normalized();
            let endpoints = [
                p1 + dir * 10.0,
                p2 - dir * 10.0,
            ];
            // Draw the line between the endpoints so that it touches the circles at each end.
            for ((x, y), value) in XiaolinWu::<f32, i64>::new((endpoints[0].x, endpoints[0].y), (endpoints[1].x, endpoints[1].y)) {
                image.put_pixel(
                    x as u32,
                    y as u32,
                    [route_r, route_g, route_b, (value * 255.0) as u8].into(),
                );
            }
        }
    }

    // Draw the hull
    // let hull_points = triangulation.hull.iter()
    //     .map(|&i| flip_y!(map_triangle_points!(points[i]) / world_blocks * map_w))
    //     .collect::<Vec<_>>();
    // for i in 0..hull_points.len() {
    //     let p1 = hull_points[i];
    //     let p2 = hull_points[(i + 1) % hull_points.len()];
    //     for ((x, y), value) in XiaolinWu::<f32, i64>::new((p1.x, p1.y), (p2.x, p2.y)) {
    //         image.put_pixel(
    //             x as u32,
    //             y as u32,
    //             [255u8, 0u8, 0u8, (value * 255.0) as u8].into(),
    //         );
    //     }
    // }

    image
}

fn save_image_file(image: &DynamicImage, image_size: MapSizeLg, base_path: &Path) {
    let mut image_file =
        File::create(base_path.with_extension("png")).expect("Could not create map file");

    if let Err(error) = PngEncoder::new(&mut image_file).write_image(
        image.as_bytes(),
        image_size.chunks().x as u32,
        image_size.chunks().y as u32,
        image::ExtendedColorType::Rgba8,
    ) {
        error!(?error, "Could not write image data");
    }

    let _ = image_file.flush();
}

pub fn save_airship_routes_map(airships: &mut Airships, index: &Index, sampler: &WorldSim) {
    let airship_routes_log_folder = env::var("AIRSHIP_ROUTES_LOG_FOLDER").ok();
    if let Some(routes_log_folder) = airship_routes_log_folder {
        let world_map_file_path =
            format!("{}/airship_routes_map_{}", routes_log_folder, index.seed);
        let base_path = PathBuf::from(world_map_file_path);
        let image = airship_routes_map(airships, sampler.map_size_lg(), index, sampler);
        save_image_file(&image, sampler.map_size_lg(), &base_path);
    }
}

pub fn save_airship_routes_triangulation(triangulation: &Triangulation, points: &Vec<Point>, index: &Index, sampler: &WorldSim) {
    let airship_routes_log_folder = env::var("AIRSHIP_ROUTES_LOG_FOLDER").ok();
    if let Some(routes_log_folder) = airship_routes_log_folder {
        let world_map_file_path =
            format!("{}/airship_docks_triangulation_{}", routes_log_folder, index.seed);
        let base_path = PathBuf::from(world_map_file_path);
        let image = dock_sites_triangulation_map(triangulation, points, sampler.map_size_lg(), index, sampler);
        save_image_file(&image, sampler.map_size_lg(), &base_path);
    }
}


#[cfg(test)]
mod tests {
    use line_drawing::BresenhamCircle;

    #[test]
    fn bresenham_circle_test() {
        let bres_circle = BresenhamCircle::new(10, 10, 5);
        for (x, y) in bres_circle {
            println!("({}, {})", x, y);
        }
    }
}