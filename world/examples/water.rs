use vek::*;
use veloren_world::{
    sim::{RiverKind, WORLD_SIZE},
    World, CONFIG,
};

const W: usize = 1024;
const H: usize = 1024;

fn main() {
    let world = World::generate(1337);

    let sampler = world.sim();

    let mut win =
        minifb::Window::new("World Viewer", W, H, minifb::WindowOptions::default()).unwrap();

    let mut focus = Vec2::zero();
    let mut _gain = 1.0;
    let mut scale = (WORLD_SIZE.x / W) as i32;

    while win.is_open() {
        let mut buf = vec![0; W * H];
        const QUADRANTS : usize = 4;
        let mut quads = [[0u32; QUADRANTS]; QUADRANTS];
        let mut rivers = 0u32;
        let mut lakes = 0u32;
        let mut oceans = 0u32;

        for i in 0..W {
            for j in 0..H {
                let pos = focus + Vec2::new(i as i32, j as i32) * scale;

                let (alt, water_alt, humidity, temperature, river_kind) = sampler
                    .get(pos)
                    .map(|sample| (sample.alt, sample.water_alt, sample.humidity, sample.temp, sample.river.river_kind))
                    .unwrap_or((CONFIG.sea_level, CONFIG.sea_level, 0.0, 0.0, None));
                let humidity = humidity
                    .min(1.0)
                    .max(0.0);
                let temperature = temperature
                    .min(1.0)
                    .max(-1.0)
                    * 0.5 + 0.5;
                let water_alt = ((alt.max(water_alt) - CONFIG.sea_level) / CONFIG.mountain_scale)
                    .min(1.0)
                    .max(0.0);
                let alt = ((alt - CONFIG.sea_level) / CONFIG.mountain_scale)
                    .min(1.0)
                    .max(0.0);
                let quad = |x: f32| ((x as f64 * QUADRANTS as f64).floor() as usize).min(QUADRANTS - 1);
                if river_kind.is_none() || humidity != 0.0 {
                    quads[quad(humidity)][quad(temperature)] += 1;
                }
                match river_kind {
                    Some(RiverKind::River { .. }) => {
                        rivers += 1;
                    },
                    Some(RiverKind::Lake { .. }) => {
                        lakes += 1;
                    },
                    Some(RiverKind::Ocean { .. }) => {
                        oceans += 1;
                    },
                    None => {},
                }

                buf[j * W + i] = match river_kind {
                    Some(RiverKind::Ocean) => u32::from_le_bytes([64, 32, 0, 255]),
                    Some(RiverKind::Lake { .. }) => u32::from_le_bytes([
                        64 + (water_alt * 191.0) as u8,
                        32 + (water_alt * 95.0) as u8,
                        0,
                        255,
                    ]),
                    Some(RiverKind::River { .. }) => u32::from_le_bytes([
                        64 + (alt * 191.0) as u8,
                        32 + (alt * 95.0) as u8,
                        0,
                        255,
                    ]),
                    None => u32::from_le_bytes([
                        (/*alt * *//*(1.0 - humidity)*/(alt * humidity).sqrt()/*temperature*/ * 255.0) as u8,
                        (/*alt*//*alt*//* * humidity*//*alt * 255.0*//*humidity*/alt * 255.0) as u8,
                        (/*alt*//*alt * *//*(1.0 - humidity)*/(alt * temperature).sqrt() * 255.0) as u8,
                        255,
                    ]),
                };
            }
        }

        let spd = 32;
        if win.is_key_down(minifb::Key::P) {
            println!("\
                Land(adjacent): (X = temp, Y = humidity): {:?}\n\
                Rivers: {:?}\n\
                Lakes: {:?}\n\
                Oceans: {:?}\n\
                Total water: {:?}\n\
                Total land(adjacent): {:?}",
                quads, rivers, lakes, oceans,
                rivers + lakes + oceans,
                quads.iter().map( |x| x.iter().sum::<u32>() ).sum::<u32>()
            );
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
            _gain += 10.0;
        }
        if win.is_key_down(minifb::Key::E) {
            _gain -= 10.0;
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
