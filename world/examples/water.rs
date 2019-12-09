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

        for i in 0..W {
            for j in 0..H {
                let pos = focus + Vec2::new(i as i32, j as i32) * scale;

                let (alt, water_alt, river_kind) = sampler
                    .get(pos)
                    .map(|sample| (sample.alt, sample.water_alt, sample.river.river_kind))
                    .unwrap_or((CONFIG.sea_level, CONFIG.sea_level, None));
                let alt = ((alt - CONFIG.sea_level) / CONFIG.mountain_scale)
                    .min(1.0)
                    .max(0.0);
                let water_alt = ((alt.max(water_alt) - CONFIG.sea_level) / CONFIG.mountain_scale)
                    .min(1.0)
                    .max(0.0);
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
                    None => u32::from_le_bytes([0, (alt * 255.0) as u8, 0, 255]),
                };
            }
        }

        let spd = 32;
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
