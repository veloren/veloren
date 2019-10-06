
use vek::*;
use veloren_world::{
    sim::{RiverKind, WORLD_SIZE},
    util::Sampler,
    World, CONFIG,
};

const W: usize = /*WORLD_SIZE.x*/1024;
const H: usize = /*WORLD_SIZE.y*/1024;

fn main() {
    let world = World::generate(1337);

    let sampler = world.sim(); //world.sample_columns();

    let mut win =
        minifb::Window::new("World Viewer", W, H, minifb::WindowOptions::default()).unwrap();

    let mut focus = Vec2::zero();
    let mut gain = 1.0;
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
                /* u32::from_le_bytes([loc_color.0, loc_color.1, alt, alt]);
                        let water_kind = match sample.river.river_kind {
                            None => 0.0,
                        };
                        (
                            sample.alt.sub(64.0).add(gain).mul(0.7).max(0.0).min(255.0) as u8,
                            sample.location,
                        )
                    })
                    .unwrap_or((0, None));

                let loc_color = location
                    .map(|l| (l.loc_idx as u8 * 17, l.loc_idx as u8 * 13))
                    .unwrap_or((0, 0));

                buf[j * W + i] = u32::from_le_bytes([loc_color.0, loc_color.1, alt, alt]); */
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
            gain += 10.0;
        }
        if win.is_key_down(minifb::Key::E) {
            gain -= 10.0;
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
