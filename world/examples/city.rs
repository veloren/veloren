use rand::thread_rng;

use vek::*;
use veloren_world::sim::Settlement;

const W: usize = 640;
const H: usize = 480;

fn main() {
    let mut win =
        minifb::Window::new("City Viewer", W, H, minifb::WindowOptions::default()).unwrap();

    let settlement = Settlement::generate(&mut thread_rng());

    while win.is_open() {
        let mut buf = vec![0; W * H];

        for i in 0..W {
            for j in 0..H {
                let pos = Vec2::new(i as f32, j as f32) * 0.002;

                let seed = settlement.get_at(pos).map(|b| b.seed).unwrap_or(0);

                buf[j * W + i] = u32::from_le_bytes([
                    (seed >> 0) as u8,
                    (seed >> 8) as u8,
                    (seed >> 16) as u8,
                    (seed >> 24) as u8,
                ]);
            }
        }

        win.update_with_buffer(&buf).unwrap();
    }
}
