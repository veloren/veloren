use common::{terrain::TerrainChunkSize, vol::RectVolSize};
use std::f32;
use vek::*;
use veloren_world::{
    sim::{RiverKind, WORLD_SIZE},
    World, CONFIG,
};

const W: usize = 1024;
const H: usize = 1024;

fn main() {
    pretty_env_logger::init();

    let world = World::generate(1337);

    let sampler = world.sim();

    let mut win =
        minifb::Window::new("World Viewer", W, H, minifb::WindowOptions::default()).unwrap();

    let mut focus = Vec2::zero();
    let mut gain = CONFIG.mountain_scale;
    let mut scale = (WORLD_SIZE.x / W) as i32;

    let light_direction = Vec3::new(-0.8, -1.0, 0.3).normalized();
    let light_res = 3;

    let mut is_basement = false;
    let mut is_shaded = true;
    let mut is_temperature = true;
    let mut is_humidity = true;

    while win.is_open() {
        let mut buf = vec![0; W * H];
        const QUADRANTS: usize = 4;
        let mut quads = [[0u32; QUADRANTS]; QUADRANTS];
        let mut rivers = 0u32;
        let mut lakes = 0u32;
        let mut oceans = 0u32;

        // let water_light = (light_direction.z + 1.0) / 2.0 * 0.8 + 0.2;

        for i in 0..W {
            for j in 0..H {
                let pos = focus + Vec2::new(i as i32, j as i32) * scale;
                /* let top_left = pos;
                let top_right = focus + Vec2::new(i as i32 + light_res, j as i32) * scale;
                let bottom_left = focus + Vec2::new(i as i32, j as i32 + light_res) * scale; */

                let (alt, basement, water_alt, humidity, temperature, downhill, river_kind) = sampler
                    .get(pos)
                    .map(|sample| {
                        (
                            sample.alt,
                            sample.basement,
                            sample.water_alt,
                            sample.humidity,
                            sample.temp,
                            sample.downhill,
                            sample.river.river_kind,
                        )
                    })
                    .unwrap_or((CONFIG.sea_level, CONFIG.sea_level, CONFIG.sea_level, 0.0, 0.0, None, None));
                let humidity = humidity.min(1.0).max(0.0);
                let temperature = temperature.min(1.0).max(-1.0) * 0.5 + 0.5;
                let downhill_pos = (downhill
                    .map(|downhill_pos| downhill_pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / sz as i32))
                    .unwrap_or(pos)
                    - pos)/* * scale*/
                    + pos;
                let downhill_alt = sampler
                    .get(downhill_pos)
                    .map(|s| if is_basement { s.basement } else { s.alt })
                    .unwrap_or(CONFIG.sea_level);
                let alt = if is_basement { basement } else { alt };
                /* let alt_tl = sampler.get(top_left).map(|s| s.alt)
                    .unwrap_or(CONFIG.sea_level);
                let alt_tr = sampler.get(top_right).map(|s| s.alt)
                    .unwrap_or(CONFIG.sea_level);
                let alt_bl = sampler.get(bottom_left).map(|s| s.alt)
                    .unwrap_or(CONFIG.sea_level); */
                let cross_pos = pos
                    + ((downhill_pos - pos)
                        .map(|e| e as f32)
                        .rotated_z(f32::consts::FRAC_PI_2)
                        .map(|e| e as i32));
                let cross_alt = sampler
                    .get(cross_pos)
                    .map(|s| if is_basement { s.basement } else { s.alt })
                    .unwrap_or(CONFIG.sea_level);
                let forward_vec = Vec3::new(
                    (downhill_pos.x - pos.x) as f64,
                    (downhill_pos.y - pos.y) as f64,
                    (downhill_alt - alt) as f64,
                );
                let up_vec = Vec3::new(
                    (cross_pos.x - pos.x) as f64,
                    (cross_pos.y - pos.y) as f64,
                    (cross_alt - alt) as f64,
                );
                let surface_normal = forward_vec.cross(up_vec).normalized();
                // let surface_normal = Vec3::new((alt_tl - alt_tr) as f64, 1.0, (alt_tl - alt_bl) as f64).normalized();
                let light = (surface_normal.dot(light_direction) + 1.0) / 2.0;
                let light = (light * 0.8) + 0.2;

                let water_alt = ((alt.max(water_alt) - CONFIG.sea_level) as f64 / gain as f64)
                    .min(1.0)
                    .max(0.0);
                let true_alt = (alt - CONFIG.sea_level) as f64 / gain as f64;
                let water_depth = (water_alt - true_alt)
                    .min(1.0)
                    .max(0.0);
                let alt = true_alt
                    .min(1.0)
                    .max(0.0);
                let quad =
                    |x: f32| ((x as f64 * QUADRANTS as f64).floor() as usize).min(QUADRANTS - 1);
                if river_kind.is_none() || humidity != 0.0 {
                    quads[quad(humidity)][quad(temperature)] += 1;
                }
                match river_kind {
                    Some(RiverKind::River { .. }) => {
                        rivers += 1;
                    }
                    Some(RiverKind::Lake { .. }) => {
                        lakes += 1;
                    }
                    Some(RiverKind::Ocean { .. }) => {
                        oceans += 1;
                    }
                    None => {}
                }

                buf[j * W + i] = match river_kind {
                    Some(RiverKind::Ocean) => u32::from_le_bytes([
                        ((64.0 - water_depth * 64.0) * 1.0) as u8,
                        ((32.0 - water_depth * 32.0) * 1.0) as u8,
                        0,
                        255,
                    ]),
                    Some(RiverKind::Lake { .. }) => u32::from_le_bytes([
                        (((64.0 + water_alt * 191.0) + (- water_depth * 64.0)) * 1.0) as u8,
                        (((32.0 + water_alt * 95.0) + (- water_depth * 32.0)) * 1.0) as u8,
                        0,
                        255,
                    ]),
                    Some(RiverKind::River { .. }) => u32::from_le_bytes([
                        64 + (alt * 191.0) as u8,
                        32 + (alt * 95.0) as u8,
                        0,
                        255,
                    ]),
                    None => {
                        let (r, g, b) = (
                            (alt * if is_temperature { temperature as f64 } else if is_shaded { alt } else { 0.0 }).sqrt(),
                            if is_shaded { 0.2 + (alt * 0.8) } else { alt },
                            (alt * if is_humidity { humidity as f64 } else if is_shaded { alt } else { 0.0 }).sqrt(),
                        );
                        let light = if is_shaded {
                            light
                        } else {
                            1.0
                        };
                        u32::from_le_bytes([
                            (b * light * 255.0) as u8,
                            (g * light * 255.0) as u8,
                            (r * light * 255.0) as u8,
                            255,
                        ])
                        /* u32::from_le_bytes([
                            (/*alt * *//*(1.0 - humidity)*/(alt * humidity).sqrt()/*temperature*/ * 255.0) as u8,
                            (/*alt*//*alt*//* * humidity*//*alt * 255.0*//*humidity*/alt * 255.0) as u8,
                            (/*alt*//*alt * *//*(1.0 - humidity)*/(alt * temperature).sqrt() * 255.0) as u8,
                            255,
                        ]) */
                    }
                };
            }
        }

        let spd = 32;
        if win.is_key_down(minifb::Key::P) {
            println!(
                "\
                 Land(adjacent): (X = temp, Y = humidity): {:?}\n\
                 Rivers: {:?}\n\
                 Lakes: {:?}\n\
                 Oceans: {:?}\n\
                 Total water: {:?}\n\
                 Total land(adjacent): {:?}",
                quads,
                rivers,
                lakes,
                oceans,
                rivers + lakes + oceans,
                quads.iter().map(|x| x.iter().sum::<u32>()).sum::<u32>()
            );
        }
        if win.get_mouse_down(minifb::MouseButton::Left) {
            if let Some((mx, my)) = win.get_mouse_pos(minifb::MouseMode::Clamp) {
                let pos = focus + Vec2::new(mx as i32, my as i32) * scale;
                println!(
                    "Chunk position: {:?}",
                    pos.map2(TerrainChunkSize::RECT_SIZE, |e, f| e * f as i32)
                );
            }
        }
        if win.is_key_down(minifb::Key::B) {
            is_basement ^= true;
        }
        if win.is_key_down(minifb::Key::H) {
            is_humidity ^= true;
        }
        if win.is_key_down(minifb::Key::T) {
            is_temperature ^= true;
        }
        if win.is_key_down(minifb::Key::L) {
            is_shaded ^= true;
        }
        if win.is_key_down(minifb::Key::W) {
            focus.y -= spd * scale;
        }
        if win.is_key_down(minifb::Key::A) {
            focus.x -= spd * scale;
        }
        if win.is_key_down(minifb::Key::S) {
            focus.y += spd * scale;
        }
        if win.is_key_down(minifb::Key::D) {
            focus.x += spd * scale;
        }
        if win.is_key_down(minifb::Key::Q) {
            gain += 64.0;
        }
        if win.is_key_down(minifb::Key::E) {
            gain = (gain - 64.0).max(64.0);
        }
        if win.is_key_down(minifb::Key::R) {
            scale += 1;
        }
        if win.is_key_down(minifb::Key::F) {
            scale = (scale - 1).max(0);
        }

        win.update_with_buffer(&buf).unwrap();
    }
}
