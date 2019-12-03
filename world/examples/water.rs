use common::{terrain::TerrainChunkSize, vol::RectVolSize};
// use self::Mode::*;
use std::{f32, f64};
use vek::*;
use veloren_world::{
    sim::{RiverKind, WorldOpts, WORLD_SIZE},
    util::Sampler,
    World, CONFIG,
};

const W: usize = 1024;
const H: usize = 1024;

/* enum Mode {
    /// Directional keys affect position of the camera.
    ///
    /// (W A S D move left and right, F B zoom in and out).
    Alt,
    /// Directional keys affect angle of the lens
    ///
    /// (W
    Lens,
    /// Directional keys affect light direction.
    ///
    /// (W A S D move left and right, F B move towards and awaay).
    Light,
}; */

fn main() {
    pretty_env_logger::init();

    let world = World::generate(1337, WorldOpts {
        seed_elements: false,
        ..WorldOpts::default()
    });

    let sampler = world.sim();

    let mut win =
        minifb::Window::new("World Viewer", W, H, minifb::WindowOptions::default()).unwrap();

    let mut focus = Vec3::new(0.0, 0.0, CONFIG.sea_level as f64);
    // Altitude is divided by gain and clamped to [0, 1]; thus, decreasing gain makes
    // smaller differences in altitude appear larger.
    let mut gain = CONFIG.mountain_scale;
    // The Z component during normal calculations is multiplied by gain; thus,
    let mut lgain = 1.0;
    let mut scale = (WORLD_SIZE.x / W) as f64;

    // Right-handed coordinate system: light is going left, down, and "backwards" (i.e. on the
    // map, where we translate the y coordinate on the world map to z in the coordinate system,
    // the light comes from -y on the map and points towards +y on the map).  In a right
    // handed coordinate system, the "camera" points towards -z, so positive z is backwards
    // "into" the camera.
    //
    // "In world space the x-axis will be pointing east, the y-axis up and the z-axis will be pointing south"
    let mut light_direction = Vec3::new(-0.8, -1.0, 0.3);
    let light_res = 3;

    let mut is_basement = false;
    let mut is_water = true;
    let mut is_shaded = true;
    let mut is_temperature = true;
    let mut is_humidity = true;

    while win.is_open() {
        let light = light_direction.normalized();
        let mut buf = vec![0; W * H];
        const QUADRANTS: usize = 4;
        let mut quads = [[0u32; QUADRANTS]; QUADRANTS];
        let mut rivers = 0u32;
        let mut lakes = 0u32;
        let mut oceans = 0u32;

        // let water_light = (light_direction.z + 1.0) / 2.0 * 0.8 + 0.2;
        let focus_rect = Vec2::from(focus);
        let true_sea_level = (CONFIG.sea_level as f64 - focus.z) / gain as f64;

        for i in 0..W {
            for j in 0..H {
                let pos = (focus_rect + Vec2::new(i as f64, j as f64) * scale).map(|e: f64| e as i32);
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
                let pos = pos * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);
                let downhill_pos = (downhill
                    .map(|downhill_pos| downhill_pos/*.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / sz as i32)*/)
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
                // Pointing downhill, forward
                // (index--note that (0,0,1) is backward right-handed)
                let forward_vec = Vec3::new(
                    (downhill_pos.x - pos.x) as f64,
                    (downhill_alt - alt) as f64 * lgain,
                    (downhill_pos.y - pos.y) as f64,
                );
                // Pointing 90 degrees left (in horizontal xy) of downhill, up
                // (middle--note that (1,0,0), 90 degrees CCW backward, is right right-handed)
                let up_vec = Vec3::new(
                    (cross_pos.x - pos.x) as f64,
                    (cross_alt - alt) as f64 * lgain,
                    (cross_pos.y - pos.y) as f64,
                );
                // Then cross points "to the right" (upwards) on a right-handed coordinate system.
                // (right-handed coordinate system means (0, 0, 1.0) is "forward" into the screen).
                let surface_normal = forward_vec.cross(up_vec).normalized();
                // f = (0, alt_bl - alt_tl, 1) [backward right-handed = (0,0,1)]
                // u = (1, alt_tr - alt_tl, 0) [right (90 degrees CCW backward) = (1,0,0)]
                // (f × u in right-handed coordinate system: pointing up)
                //
                // f × u =
                //   (a.y*b.z - a.z*b.y,
                //    a.z*b.x - a.x*b.z,
                //    a.x*b.y - a.y*b.x,
                //   )
                // =
                //   (-(alt_tr - alt_tl),
                //    1,
                //    -(alt_bl - alt_tl),
                //   )
                // =
                //   (alt_tl - alt_tr,
                //    1,
                //    alt_tl - alt_bl,
                //   )
                //
                // let surface_normal = Vec3::new((alt_tl - alt_tr) as f64, 1.0, (alt_tl - alt_bl) as f64).normalized();
                let light = (surface_normal.dot(light) + 1.0) / 2.0;
                let light = (light * 0.9) + 0.1;

                let true_water_alt = (alt.max(water_alt) as f64 - focus.z) / gain as f64;
                let true_alt = (alt as f64 - focus.z) / gain as f64;
                let water_depth = (true_water_alt - true_alt)
                    .min(1.0)
                    .max(0.0);
                let water_alt = true_water_alt
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

                buf[j * W + i] = match (river_kind, (is_water, true_alt >= true_sea_level)) {
                    (_, (false, _)) | ( None, (_, true)) => {
                        let (r, g, b) = (
                            (if is_shaded { alt } else { alt } * if is_temperature { temperature as f64 } else if is_shaded { alt } else { 0.0 }).sqrt(),
                            if is_shaded { 0.2 + (alt * 0.8) } else { alt },
                            (if is_shaded { alt } else { alt } * if is_humidity { humidity as f64 } else if is_shaded { alt } else { 0.0 }).sqrt(),
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
                    },
                    (Some(RiverKind::Ocean), _) => u32::from_le_bytes([
                        ((64.0 - water_depth * 64.0) * 1.0) as u8,
                        ((32.0 - water_depth * 32.0) * 1.0) as u8,
                        0,
                        255,
                    ]),
                    (Some(RiverKind::River { .. }), _) => u32::from_le_bytes([
                        64 + (alt * 191.0) as u8,
                        32 + (alt * 95.0) as u8,
                        0,
                        255,
                    ]),
                    (None, _) | (Some(RiverKind::Lake { .. }), _) => u32::from_le_bytes([
                        (((64.0 + water_alt * 191.0) + (- water_depth * 64.0)) * 1.0) as u8,
                        (((32.0 + water_alt * 95.0) + (- water_depth * 32.0)) * 1.0) as u8,
                        0,
                        255,
                    ]),
                };
            }
        }

        let spd = 32.0;
        let lspd = 0.1;
        if win.is_key_down(minifb::Key::P) {
            println!(
                "\
                 Gain / Shade gain: {:?} / {:?}\n\
                 Scale / Focus: {:?} / {:?}\n\
                 Light: {:?}
                 Land(adjacent): (X = temp, Y = humidity): {:?}\n\
                 Rivers: {:?}\n\
                 Lakes: {:?}\n\
                 Oceans: {:?}\n\
                 Total water: {:?}\n\
                 Total land(adjacent): {:?}",
                gain,
                lgain,
                scale,
                focus,
                light_direction,
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
                let pos = (focus_rect + (Vec2::new(mx as f64, my as f64) * scale)).map(|e| e as i32);
                println!(
                    "Chunk position: {:?}",
                    pos.map2(TerrainChunkSize::RECT_SIZE, |e, f| e * f as i32)
                );
            }
        }
        let is_camera = win.is_key_down(minifb::Key::C);
        if win.is_key_down(minifb::Key::B) {
            is_basement ^= true;
        }
        if win.is_key_down(minifb::Key::H) {
            is_humidity ^= true;
        }
        if win.is_key_down(minifb::Key::T) {
            is_temperature ^= true;
        }
        if win.is_key_down(minifb::Key::O) {
            is_water ^= true;
        }
        if win.is_key_down(minifb::Key::L) {
            is_shaded ^= true;
        }
        if win.is_key_down(minifb::Key::W) {
            if is_camera {
                light_direction.z -= lspd;
            } else {
                focus.y -= spd * scale;
            }
        }
        if win.is_key_down(minifb::Key::A) {
            if is_camera {
                light_direction.x -= lspd;
            } else {
                focus.x -= spd * scale;
            }
        }
        if win.is_key_down(minifb::Key::S) {
            if is_camera {
                light_direction.z += lspd;
            } else {
                focus.y += spd * scale;
            }
        }
        if win.is_key_down(minifb::Key::D) {
            if is_camera {
                light_direction.x += lspd;
            } else {
                focus.x += spd * scale;
            }
        }
        if win.is_key_down(minifb::Key::Q) {
            if is_camera {
                if (lgain * 2.0).is_normal() {
                    lgain *= 2.0;
                }
            } else {
                gain += 64.0;
            }
        }
        if win.is_key_down(minifb::Key::E) {
            if is_camera {
                if (lgain / 2.0).is_normal() {
                    lgain /= 2.0;
                }
            } else {
                gain = (gain - 64.0).max(64.0);
            }
        }
        if win.is_key_down(minifb::Key::R) {
            if is_camera {
                focus.z += spd * scale;
            } else {
                if (scale * 2.0).is_normal() {
                    scale *= 2.0;
                }
                // scale += 1;
            }
        }
        if win.is_key_down(minifb::Key::F) {
            if is_camera {
                focus.z -= spd * scale;
            } else {
                if (scale / 2.0).is_normal() {
                    scale /= 2.0;
                }
                // scale = (scale - 1).max(0);
            }
        }

        win.update_with_buffer(&buf).unwrap();
    }
}
