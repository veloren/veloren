use crate::{
    CONFIG, Index, IndexRef,
    civ::airship_travel::{Airships, DockNode},
    sim::{WorldSim, get_horizon_map, sample_pos, sample_wpos},
    util::{DHashMap, DHashSet},
};
use common::terrain::{
    map::{MapConfig, MapSample, MapSizeLg},
    uniform_idx_as_vec2,
};
use delaunator::{Point, Triangulation};
use image::{DynamicImage, GenericImage, ImageEncoder, codecs::png::PngEncoder};
use line_drawing::{BresenhamCircle, XiaolinWu};
use serde::Deserialize;
use tiny_skia::{
    IntRect, IntSize,
    Pixmap, PixmapPaint,
    Transform
};

use std::{
    env,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use tracing::error;
use vek::*;

trait PixmapExt {
    fn bounds(&self) -> IntRect;
    fn copy_region(&self, rect: IntRect) -> Option<Pixmap>;
}

impl PixmapExt for Pixmap {

    /// Returns the Pixmap bounds or a 1x1 rectangle if the width or height is invalid
    /// (which should not happen due to the defensive design of IntRect and Pixmap).
    fn bounds(&self) -> IntRect {
        if let Some(bounds) = IntRect::from_xywh(0, 0, self.width(), self.height()) {
            bounds
        } else {
            IntRect::from_xywh(0, 0, 1, 1).unwrap()
        }
    }

    fn copy_region(&self, rect: IntRect) -> Option<Pixmap> {
        if self.bounds().contains(&rect) &&
            let Some(region_size) = IntSize::from_wh(rect.width(), rect.height()) &&
            let Some(from_rect) = self.bounds().intersect(&rect)
        {
            let stride = self.width() as i32 * tiny_skia::BYTES_PER_PIXEL as i32;
            let mut region_data = 
                Vec::with_capacity((from_rect.width() * from_rect.height()) as usize * tiny_skia::BYTES_PER_PIXEL);

            for y in from_rect.top()..from_rect.bottom() {
                let row_start = y * stride + from_rect.left() * tiny_skia::BYTES_PER_PIXEL as i32;
                let row_end = row_start + from_rect.width() as i32 * tiny_skia::BYTES_PER_PIXEL as i32;
                region_data.extend_from_slice(
                    &self.data()[row_start as usize..row_end as usize]);
            }
            Pixmap::from_vec(region_data, region_size)
        } else {
            None
        }
    }
}

#[derive(Deserialize, Debug)]
struct TinySkiaSpriteMeta {
    id: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Deserialize, Debug)]
struct TinySkiaSpriteMapMeta {
    texture_width: u32,
    texture_height: u32,
    sprites_meta: Vec<TinySkiaSpriteMeta>,
}

pub struct TinySkiaSpriteMap {
    sprites: Vec<tiny_skia::Pixmap>,
    sprite_ids: DHashMap<String, usize>,
}

impl TinySkiaSpriteMap {
    fn new(image_path: &str, meta_path: &str) -> Self {
        let mut sprites = Vec::default();
        let mut sprite_ids = DHashMap::default();
        match std::fs::File::open(&meta_path) {
            Ok(file) => {
                match ron::de::from_reader::<_, TinySkiaSpriteMapMeta>(&file) {
                    Ok(sprite_map_meta) => {
                        match Pixmap::load_png(&image_path) {
                            Ok(spritesheet) => {
                            for sprite_meta in sprite_map_meta.sprites_meta.iter() {
                                    if let Some(sprite_frame) = IntRect::from_xywh(
                                        sprite_meta.x,
                                        sprite_meta.y,
                                        sprite_meta.width,
                                        sprite_meta.height,
                                    ) &&
                                    let Some(sprite) = spritesheet.copy_region(sprite_frame) {
                                        // sprite.set_transform(Transform::from_scale(1.0, -1.0));
                                        sprites.push(sprite);
                                        sprite_ids.insert(sprite_meta.id.clone(), sprites.len() - 1);
                                    }
                                } 
                            }
                            Err(error) => error!(?error, ?image_path, "Couldn't decode image file"),
                        }
                    }
                    Err(error) => error!(?error, ?file, "Couldn't parse SpriteMap meta file"),
                }
            },
            Err(error) => error!(?error, ?meta_path, "Couldn't open SpriteMap meta file"),
        }
        TinySkiaSpriteMap { sprites, sprite_ids }
    }

    fn get_sprite(&self, id: &str) -> Option<&Pixmap> {
        if let Some(index) = self.sprite_ids.get(id) {
            return Some(&self.sprites[*index]);
        }
        None
    }
}


/// Fills a rectangle in the image with the given color.
/// See https://stackoverflow.com/questions/10061146/how-to-rasterize-rotated-rectangle-in-2d-by-setpixel
/// for the basic algorithm. This uses Xiaolin Wu's line algorithm to generate the edges and accounts
/// for the alpha values along the edges.
fn fill_rect(
    image: &mut DynamicImage,
    pts: &[Vec2<f32>;4],
    color: [u8; 3],
) {
    // buffer values are (y coordinate, alpha)
    let mut buf_x0 = vec![(0i32, 0.0f32); 1024];
    let mut buf_x1 = vec![(0i32, 0.0f32); 1024];

    // pts are assumed to be in clockwise winding order. I.e, the rectangle
    // would be drawn from pts[0] to pts[1], then pts[1] to pts[2], etc., in
    // a clockwise manner.

    let mut min_buf_y = i32::MAX;
    let mut max_buf_y = i32::MIN;
    let mut min_buf_x = i32::MAX;
    let mut max_buf_x = i32::MIN;

    for i in 0..4 {
        let j = (i+1) % 4;
        let dy = pts[j].y - pts[i].y;
        let mut minx = i32::MAX;
        let mut maxx = i32::MIN;
        let mut miny = i32::MAX;
        let mut maxy = i32::MIN;
        let mut minx_value = 0.0f32;
        let mut maxx_value = 0.0f32;

        for ((x, y), value) in XiaolinWu::<f32, i32>::new(
            (pts[i].x, pts[i].y),
            (pts[j].x, pts[j].y),
        ) {
            if x < min_buf_x {
                min_buf_x = x;
            }
            if x > max_buf_x {
                max_buf_x = x;
            }
            if y < min_buf_y {
                min_buf_y = y;
            }
            if y > max_buf_y {
                max_buf_y = y;
            }
            if dy < 0.0 {
                buf_x0[y as usize] = (x, value);
            } else if dy > 0.0 {
                buf_x1[y as usize] = (x, value);
            } else {
                if x < minx {
                    minx = x;
                    minx_value = value;
                }
                if x > maxx {
                    maxx = x;
                    maxx_value = value;
                }
                if y < miny {
                    miny = y;
                }
                if y > maxy {
                    maxy = y;
                }
            }
        }
        if !(dy < 0.0 || dy > 0.0) {
            for y in miny..=maxy {
                buf_x0[y as usize] = (minx, minx_value);
                buf_x1[y as usize] = (maxx, maxx_value);
            }
        }  
    }

    for y in min_buf_y..=max_buf_y {
        let (x0, value0) = buf_x0[y as usize];
        let (x1, value1) = buf_x1[y as usize];
        if x0 > x1 {
            continue;
        }
        if x0 < min_buf_x {
            continue;
        }
        if x1 > max_buf_x {
            continue;
        }
        image.put_pixel(x0 as u32, y as u32, [color[0], color[1], color[2], (value0 * 255.0) as u8].into());
        image.put_pixel(x1 as u32, y as u32, [color[0], color[1], color[2], (value1 * 255.0) as u8].into());

        for x in x0 + 1..x1 {
            image.put_pixel(x as u32, y as u32, [color[0], color[1], color[2], 255].into());
        }
    }   
}

fn fill_line(
    image: &mut DynamicImage,
    start: &Vec2<f32>,
    end: &Vec2<f32>,
    width: f32,
    color: [u8; 3],
) {
    let dir1 = (end - start).normalized();
    let dir1cw = Vec2::new(-dir1.y, dir1.x);
    let line = [
        start + dir1cw * width / 2.0,
        start - dir1cw * width / 2.0,
        end - dir1cw * width,
        end + dir1cw * width,
    ];
    fill_rect(image, &line, color);
}

fn draw_dock_pos_indicator_arrows(
    image: &mut DynamicImage,
    p1: &Vec2<f32>,
    p2: &Vec2<f32>,
    out_index: usize,
    in_index: usize,
    width: f32,
    color: [u8; 3],
) {
    let dir = (p2 - p1).normalized();
    let arrow_dir1 = dir.rotated_z(3.0 * std::f32::consts::FRAC_PI_4);
    let arrow_dir2 = Vec2::new(-arrow_dir1.y, arrow_dir1.x);
    let p21 = p2 - dir * 10.0;
    let arrow_p2 = p21 + arrow_dir1 * 20.0;
    fill_line(image, &p21, &arrow_p2, width, color);
    if in_index > 1 {
        let arrow_p2 = p21 + arrow_dir2 * 20.0;
        fill_line(image, &p21, &arrow_p2, width, color);
    };
    if in_index > 2 {
       let arrow_p1 = p21 - dir * 10.0;
       let arrow_p2 = arrow_p1 + arrow_dir1 * 20.0; 
       fill_line(image, &arrow_p1, &arrow_p2, width, color);
    } if in_index > 3 {
        let arrow_p1 = p21 - dir * 10.0;
        let arrow_p2 = arrow_p1 + arrow_dir2 * 20.0; 
        fill_line(image, &arrow_p1, &arrow_p2, width, color);
    }
    let p11 = p1 + dir * 15.0;
    let arrow_p2 = p11 + arrow_dir1 * 20.0;
    fill_line(image, &p11, &arrow_p2, width, color);
    if out_index > 1 {
        let arrow_p2 = p11 + arrow_dir2 * 20.0;
        fill_line(image, &p11, &arrow_p2, width, color);
    };
    if out_index > 2 {
       let arrow_p1 = p11 + dir * 10.0;
       let arrow_p2 = arrow_p1 + arrow_dir1 * 20.0; 
       fill_line(image, &arrow_p1, &arrow_p2, width, color);
    } if out_index > 3 {
        let arrow_p1 = p11 + dir * 10.0;
        let arrow_p2 = arrow_p1 + arrow_dir2 * 20.0; 
        fill_line(image, &arrow_p1, &arrow_p2, width, color);
    }
}

fn draw_dock_pos_indicator_lines(
    image: &mut DynamicImage,
    p1: &Vec2<f32>,
    p2: &Vec2<f32>,
    out_index: usize,
    in_index: usize,
    width: f32,
    color: [u8; 3],
) {
    const WHITE_COLOR: [u8; 3] = [255, 255, 255];
    let dir = (p2 - p1).normalized();
    let line_dir1 = Vec2::new(-dir.y, dir.x);
    let line_dir2 = -line_dir1;
    let p1p2len = p1.distance(*p2);
    let p11 = p1 + (p1p2len * 0.25).max(10.0) * dir;
    let p21 = p2 - (p1p2len * 0.25).max(10.0) * dir;

    let line_p2 = p11 + line_dir1 * 20.0;
    fill_line(image, &p11, &line_p2, width, WHITE_COLOR);
    if out_index > 0 {
        let line_p2 = p11 + line_dir2 * 20.0;
        fill_line(image, &p11, &line_p2, width, WHITE_COLOR);
    };
    if out_index > 1 {
       let line_p1 = p11 + dir * 8.0;
       let line_p2 = line_p1 + line_dir1 * 20.0; 
       fill_line(image, &line_p1, &line_p2, width, WHITE_COLOR);
    } 
    if out_index > 2 {
        let line_p1 = p11 + dir * 8.0;
        let line_p2 = line_p1 + line_dir2 * 20.0; 
        fill_line(image, &line_p1, &line_p2, width, WHITE_COLOR);
    }

    let line_p2 = p21 + line_dir1 * 20.0;
    fill_line(image, &p21, &line_p2, width, color);
    if in_index > 0 {
        let line_p2 = p21 + line_dir2 * 20.0;
        fill_line(image, &p21, &line_p2, width, color);
    };
    if in_index > 1 {
       let line_p1 = p21 - dir * 8.0;
       let line_p2 = line_p1 + line_dir1 * 20.0; 
       fill_line(image, &line_p1, &line_p2, width, color);
    } if in_index > 2 {
        let line_p1 = p21 - dir * 8.0;
        let line_p2 = line_p1 + line_dir2 * 20.0; 
        fill_line(image, &line_p1, &line_p2, width, color);
    }
}

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
    let mut image = DynamicImage::new(map_w as u32, map_h as u32, image::ColorType::Rgba8);

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

fn airship_routes_map(
    airships: &mut Airships,
    image_size: MapSizeLg,
    index: &Index,
    sampler: &WorldSim,
) -> DynamicImage {
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

    // route coordinates are in world blocks, convert to map pixels and invert y
    // axis
    for route in airships.routes.values() {
        let dock_centers = [
            Vec2::new(
                route.approaches[0].dock_center.x / world_blocks.x * map_w,
                map_h - route.approaches[0].dock_center.y / world_blocks.y * map_h,
            ),
            Vec2::new(
                route.approaches[1].dock_center.x / world_blocks.x * map_w,
                map_h - route.approaches[1].dock_center.y / world_blocks.y * map_h,
            ),
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

        // calculate where the route_line intersects a circle of radius 20 around each
        // dock center
        let route_dir = (dock_centers[1] - dock_centers[0]).normalized();
        let endpoints = [
            dock_centers[0] + route_dir * 10.0,
            dock_centers[1] - route_dir * 10.0,
        ];

        // Draw a line between the endpoints that intersect the circles
        for ((x, y), value) in XiaolinWu::<f32, i64>::new(
            (endpoints[0].x, endpoints[0].y),
            (endpoints[1].x, endpoints[1].y),
        ) {
            image.put_pixel(
                x as u32,
                y as u32,
                [route_r, route_g, route_b, (value * 255.0) as u8].into(),
            );
        }
    }

    image
}

fn airship_route_segments_map(
    segments: &Vec<Vec<usize>>,
    points: &Vec<Point>,
    image_size: MapSizeLg,
    index: &Index,
    sampler: &WorldSim,
) -> DynamicImage {
    let mut image = basic_world_map(image_size, index, sampler);
    let world_chunks = sampler.map_size_lg().chunks();
    let world_blocks = world_chunks.map(|u| u as f32) * 32.0;
    let map_w = image_size.chunks().x as f32;
    let map_h = image_size.chunks().y as f32;

    let map_points = points
        .iter()
        .map(|p| {
            Vec2::new(
                (p.x / world_blocks.x as f64 * map_w as f64) as f32,
                (map_h as f64 - (p.y / world_blocks.y as f64 * map_h as f64)) as f32,
            )
        })
        .collect::<Vec<_>>();

    let dock_color = [105u8, 231u8, 255u8, 255u8];
    // Draw a circle around the points
    for dock_center in map_points.iter() {
        for (x, y) in BresenhamCircle::new(dock_center.x as i32, dock_center.y as i32, 10) {
            if x < 0 || y < 0 || x >= map_w as i32 || y >= map_h as i32 {
                continue;
            }
            image.put_pixel(x as u32, y as u32, dock_color.into());
        }
    }

    let segment_colors = [
        [255u8, 0u8, 0u8],
        [0u8, 255u8, 0u8],
        [0u8, 0u8, 255u8],
        [255u8, 255u8, 0u8],
    ];

    for (i, segment) in segments.iter().enumerate() {

        let mut dock_pos_map: DHashMap<usize, DHashSet<usize>> = DHashMap::default();
        let mut outgoing_dock_pos_index = 0usize;
    
        let color = segment_colors[i % segment_colors.len()];
        
        for i in 0..segment.len() - 1 {
            let p1 = map_points[segment[i]];
            let p2 = map_points[segment[i + 1]];
            let dir = (p2 - p1).normalized();
            let ep1 = p1 + dir * 10.0;
            let ep2 = p2 - dir * 10.0;
            fill_line(
                &mut image,
                &ep1,
                &ep2,
                3.0,
                color,
            );

            // calculate the docking position index for the next point as the
            // first index not in use.
            let to_dock_pos_index = {
                if let Some(dock_pos_set) = dock_pos_map.get_mut(&segment[i+1]) {
                    let mut dock_pos_index = 0usize;
                    while dock_pos_set.contains(&dock_pos_index) {
                        dock_pos_index += 1;
                    }
                    if dock_pos_index > 3 {
                        error!("Dock position index must be less than 4");
                        dock_pos_index = 3;
                    }
                    dock_pos_set.insert(dock_pos_index);
                    dock_pos_index
                } else {
                    let mut dock_pos_set = DHashSet::default();
                    dock_pos_set.insert(0usize);
                    dock_pos_map.insert(segment[i+1], dock_pos_set);
                    0usize
                }
            };
            println!("Segment {}, {} to {}, to_dock_pos_index: {}, outgoing_dock_pos_index: {}", i, segment[i], segment[i+1], to_dock_pos_index, outgoing_dock_pos_index);
            draw_dock_pos_indicator_lines(&mut image, &ep1, &ep2, outgoing_dock_pos_index, to_dock_pos_index, 2.5, color);
            outgoing_dock_pos_index = to_dock_pos_index;
        }
    }
    
    image
}

fn dock_sites_optimized_tesselation_map(
    _triangulation: &Triangulation,
    points: &Vec<Point>,
    node_connections: &DHashMap<usize, DockNode>,
    image_size: MapSizeLg,
    index: &Index,
    sampler: &WorldSim,
) -> DynamicImage {
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

    // coordinates are in world blocks (0 at bottom),
    // convert to map pixels and invert y axis (0 at top)
    // macro_rules! map_triangle_points {
    //     ($vec:expr) => {
    //         Vec2 {
    //             x: $vec.x as f32,
    //             y: $vec.y as f32,
    //         }
    //     };
    // }

    // macro_rules! flip_y {
    //     ($vec:expr) => {
    //         Vec2 {
    //             x: $vec.x,
    //             y: map_h - $vec.y,
    //         }
    //     };
    // }

    // Draw a circle around the points
    for dock_center in points.iter() {
        let dcx = (dock_center.x / world_blocks.x as f64 * map_w as f64) as i32;
        let dcy = (map_h as f64 - (dock_center.y / world_blocks.y as f64 * map_h as f64)) as i32;
        for (x, y) in BresenhamCircle::new(dcx, dcy, 10) {
            if x < 0 || y < 0 || x >= map_w as i32 || y >= map_h as i32 {
                continue;
            }
            image.put_pixel(x as u32, y as u32, [site_r, site_g, site_b, 255].into());
        }
    }

    // Draw the dock node connections
    let mut lines_drawn: DHashSet<(usize, usize)> = DHashSet::default();
    for (_, dock_node) in node_connections.iter() {
        if let Some(dp1) = points.get(dock_node.node_id) {
            dock_node.connected.iter().for_each(|cpid| {
                if let Some(dp2) = points.get(*cpid) {
                    if !lines_drawn.contains(&(dock_node.node_id, *cpid)) {
                        let p1mx = (dp1.x / world_blocks.x as f64 * map_w as f64) as f32;
                        let p1my =
                            (map_h as f64 - (dp1.y / world_blocks.y as f64 * map_h as f64)) as f32;
                        let p2mx = (dp2.x / world_blocks.x as f64 * map_w as f64) as f32;
                        let p2my =
                            (map_h as f64 - (dp2.y / world_blocks.y as f64 * map_h as f64)) as f32;
                        let p1 = Vec2::new(p1mx, p1my);
                        let p2 = Vec2::new(p2mx, p2my);

                        // calculate where the line intersects a circle of radius 10 around
                        // each point
                        let dir = (p2 - p1).normalized();
                        let endpoints = [p1 + dir * 10.0, p2 - dir * 10.0];
                        // Draw the line between the endpoints 
                        fill_line(
                            &mut image,
                            &endpoints[0],
                            &endpoints[1],
                            2.5,
                            [route_r, route_g, route_b],
                        );
                        lines_drawn.insert((dock_node.node_id, *cpid));
                    }
                }
            });
        }
    }

    image
}

fn dock_sites_triangulation_map(
    triangulation: &Triangulation,
    points: &Vec<Point>,
    image_size: MapSizeLg,
    index: &Index,
    sampler: &WorldSim,
) -> DynamicImage {
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

    // the triangles are triplets in a Vec<usize> so we need to iterate over them in
    // groups of 3. The macros are used to convert the points from world blocks
    // to map pixels and flip the y axis. map_triangles is a Vec of arrays of 3
    // Vec2s representing the 3 points of each triangle.
    let map_triangles = triangulation
        .triangles
        .chunks(3)
        .map(|triangle| {
            [
                flip_y!(map_triangle_points!(points[triangle[0]]) / world_blocks * map_w),
                flip_y!(map_triangle_points!(points[triangle[1]]) / world_blocks * map_w),
                flip_y!(map_triangle_points!(points[triangle[2]]) / world_blocks * map_w),
            ]
        })
        .collect::<Vec<_>>();

    for triangle in map_triangles.iter() {
        // triangle is an array of 3 Vec2<f32> representing the 3 points of the
        // triangle.

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
            // calculate where the triangle edge intersects a circle of radius 10 around
            // each point
            let dir1 = (p2 - p1).normalized();
            let start_edge_center = p1 + dir1 * 10.0;
            let end_edge_center = p2 - dir1 * 10.0;

            // This is a simplified rectangle fill for the line to get more thickness.
            fill_line(
                &mut image,
                &start_edge_center,
                &end_edge_center,
                3.0,
                [route_r, route_g, route_b],
            );
        }
    }

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

pub fn save_airship_routes_triangulation(
    triangulation: &Triangulation,
    points: &Vec<Point>,
    index: &Index,
    sampler: &WorldSim,
) {
    let airship_routes_log_folder = env::var("AIRSHIP_ROUTES_LOG_FOLDER").ok();
    if let Some(routes_log_folder) = airship_routes_log_folder {
        let world_map_file_path = format!(
            "{}/airship_docks_triangulation_{}",
            routes_log_folder, index.seed
        );
        let base_path = PathBuf::from(world_map_file_path);
        let image = dock_sites_triangulation_map(
            triangulation,
            points,
            sampler.map_size_lg(),
            index,
            sampler,
        );
        save_image_file(&image, sampler.map_size_lg(), &base_path);
    }
}

pub fn save_airship_routes_optimized_tesselation(
    triangulation: &Triangulation,
    points: &Vec<Point>,
    node_connections: &DHashMap<usize, DockNode>,
    index: &Index,
    sampler: &WorldSim,
) {
    let airship_routes_log_folder = env::var("AIRSHIP_ROUTES_LOG_FOLDER").ok();
    if let Some(routes_log_folder) = airship_routes_log_folder {
        let world_map_file_path = format!(
            "{}/airship_docks_opt_tesselation{}",
            routes_log_folder, index.seed
        );
        let base_path = PathBuf::from(world_map_file_path);
        let image = dock_sites_optimized_tesselation_map(
            triangulation,
            points,
            node_connections,
            sampler.map_size_lg(),
            index,
            sampler,
        );
        save_image_file(&image, sampler.map_size_lg(), &base_path);
    }
}

pub fn save_airship_route_segments(
    segments: &Vec<Vec<usize>>,
    points: &Vec<Point>,
    index: &Index,
    sampler: &WorldSim,
) {
    let airship_routes_log_folder = env::var("AIRSHIP_ROUTES_LOG_FOLDER").ok();
    if let Some(routes_log_folder) = airship_routes_log_folder {
        let world_map_file_path = format!(
            "{}/best_route_segments{}",
            routes_log_folder, index.seed
        );
        let base_path = PathBuf::from(world_map_file_path);
        let image = airship_route_segments_map(
            segments,
            points,
            sampler.map_size_lg(),
            index,
            sampler,
        );
        save_image_file(&image, sampler.map_size_lg(), &base_path);
    }
}


#[cfg(test)]
mod tests {
    use line_drawing::BresenhamCircle;
    use tiny_skia::*;
    //use serde;//::{Deserialize, Serialize};
    use tracing::{error, warn};
    use crate::util::DHashMap;
    use super::TinySkiaSpriteMap;
    
    #[test]
    fn bresenham_circle_test() {
        let bres_circle = BresenhamCircle::new(10, 10, 5);
        for (x, y) in bres_circle {
            println!("({}, {})", x, y);
        }
    }

    #[test]
    fn packed_sprites_test() {
        let file_path = "/Users/ronw/Projects/Games/Veloren/NewRoutes/airship_node_number_frames.ron";
        println!("Reading file: {}", file_path);
        match std::fs::File::open(&file_path) {
            Ok(file) => match ron::de::from_reader::<_, DHashMap<String, Vec<u32>>>(&file) {
                Ok(frames) => println!("Frames: {:?}", frames),
                Err(error) => error!(?error, ?file, "Couldn't read airship_node_number_frames file"),
            },
            Err(error) => error!(?error, ?file_path, "Couldn't open airship_node_number_frames file"),
        }
    }

    #[test]
    fn tiny_skia_test() {
        let mut paint1 = Paint::default();
        paint1.set_color_rgba8(50, 127, 150, 200);
        paint1.anti_alias = true;
    
        let mut paint2 = Paint::default();
        paint2.set_color_rgba8(220, 140, 75, 180);
        paint2.anti_alias = false;
    
        let path1 = {
            let mut pb = PathBuilder::new();
            pb.move_to(60.0, 60.0);
            pb.line_to(160.0, 940.0);
            pb.cubic_to(380.0, 840.0, 660.0, 800.0, 940.0, 800.0);
            pb.cubic_to(740.0, 460.0, 440.0, 160.0, 60.0, 60.0);
            pb.close();
            pb.finish().unwrap()
        };
    
        let path2 = {
            let mut pb = PathBuilder::new();
            pb.move_to(940.0, 60.0);
            pb.line_to(840.0, 940.0);
            pb.cubic_to(620.0, 840.0, 340.0, 800.0, 60.0, 800.0);
            pb.cubic_to(260.0, 460.0, 560.0, 160.0, 940.0, 60.0);
            pb.close();
            pb.finish().unwrap()
        };
    
        let mut pixmap = Pixmap::new(1000, 1000).unwrap();
        pixmap.fill_path(
            &path1,
            &paint1,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        pixmap.fill_path(
            &path2,
            &paint2,
            FillRule::Winding,
            Transform::identity(),
            None,
        );

        let mut txt_paint = PixmapPaint::default();
        txt_paint.quality = FilterQuality::Bicubic;
        let txt_transform = Transform::from_scale(2.0, 2.0);
        let txt_half_transform = Transform::from_scale(0.5, 0.5);
    
        let num15 = Pixmap::load_png("/Users/ronw/Projects/Games/Veloren/NewRoutes/NodeNumberSprites/15.png").unwrap();
        pixmap.draw_pixmap(5, 5, num15.as_ref(), &txt_paint, txt_transform, None);
        pixmap.draw_pixmap(100, 10, num15.as_ref(), &txt_paint, Transform::identity(), None);
        pixmap.draw_pixmap(300, 20, num15.as_ref(), &txt_paint, txt_half_transform, None);

        let numbers_sprite_map = TinySkiaSpriteMap::new(
            "/Users/ronw/Projects/Games/Veloren/NewRoutes/blue_numbers.png",
            "/Users/ronw/Projects/Games/Veloren/NewRoutes/blue_numbers.ron",
        );
        for row in 0..5 {
            for col in 0..10 {
                let node_num = row * 10 + col + 1;
                let node_num_str = format!("_{}_BLUE", node_num);
                if let Some(sprite) = numbers_sprite_map.get_sprite(&node_num_str) {
                    pixmap.draw_pixmap(5 + (col * 50), 50 + (row * 50), sprite.as_ref(), &txt_paint, Transform::identity(), None);
                }
            }
        }

        pixmap.save_png("/Users/ronw/Projects/Games/Veloren/NewRoutes/tiny-skia-image.png").unwrap();
    }
    
}
