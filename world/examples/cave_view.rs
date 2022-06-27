use rand::thread_rng;
use vek::*;
use veloren_world::{index::Index, site::Settlement, IndexRef};

const W: usize = 640;
const H: usize = 480;

fn main() {
    let seed = 1337;
    let index = &Index::new(seed);

    let mut win =
        minifb::Window::new("Cave Viewer", W, H, minifb::WindowOptions::default()).unwrap();

    let settlement = Settlement::generate(Vec2::zero(), None, &mut thread_rng());

    let mut focus = Vec2::<f32>::zero();
    let mut zoom = 1.0;
    let mut is_t = false;
    let colors = &*index.colors();
    let features = &*index.features();
    let index = IndexRef {
        colors,
        features,
        index,
    };

    while win.is_open() {
        let mut buf = vec![0; W * H];

        let win_to_pos =
            |wp: Vec2<usize>| (wp.map(|e| e as f32) - Vec2::new(W as f32, H as f32) * 0.5) * zoom;

        for i in 0..W {
            for j in 0..H {
                use common::terrain::{quadratic_nearest_point, river_spline_coeffs};

                let pos = focus + win_to_pos(Vec2::new(i, j)) * zoom;

                let a = Vec2::new(1000.0, 0.0);
                let b = Vec2::new(1100.0, 30.0);
                let d = Vec2::new(0.0, 0.0);
                let closest = quadratic_nearest_point(
                    &river_spline_coeffs(a, d, b),
                    pos.map(|e| e as f64),
                    Vec2::new(a, b),
                )
                .unwrap();
                let color = Lerp::lerp(
                    Rgb::new(1.0, 0.0, 0.0),
                    Rgb::new(0.0, 1.0, 0.0),
                    1.0 / (1.0 + if is_t { closest.0 } else { closest.2 }),
                );

                let color = Rgba::new(color.r, color.g, color.b, 1.0);
                buf[j * W + i] = u32::from_le_bytes(color.map(|e| (e * 255.0) as u8).into_array());
            }
        }

        let spd = 4.0;
        if win.is_key_down(minifb::Key::W) {
            focus.y -= spd * zoom;
        }
        if win.is_key_down(minifb::Key::A) {
            focus.x -= spd * zoom;
        }
        if win.is_key_down(minifb::Key::S) {
            focus.y += spd * zoom;
        }
        if win.is_key_down(minifb::Key::D) {
            focus.x += spd * zoom;
        }
        if win.is_key_down(minifb::Key::Q) {
            zoom *= 1.015;
        }
        if win.is_key_down(minifb::Key::E) {
            zoom /= 1.015;
        }
        if win.is_key_down(minifb::Key::Tab) {
            is_t ^= true;
        }

        win.update_with_buffer(&buf, W, H).unwrap();
    }
}
