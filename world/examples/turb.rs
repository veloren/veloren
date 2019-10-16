use noise::{Seedable, SuperSimplex};

use vek::*;

const W: usize = 640;
const H: usize = 640;

fn main() {
    let mut win = minifb::Window::new("Turb", W, H, minifb::WindowOptions::default()).unwrap();

    let _nz_x = SuperSimplex::new().set_seed(0);
    let _nz_y = SuperSimplex::new().set_seed(1);

    let mut time = 0.0f64;
    while win.is_open() {
        let mut buf = vec![0; W * H];

        for i in 0..W {
            for j in 0..H {
                let pos = Vec2::new(i as f64 / W as f64, j as f64 / H as f64) * 0.5 - 0.25;

                let pos = pos * 10.0;

                let pos = (0..10).fold(pos, |pos, _| pos.map(|e| e.powf(3.0) - 1.0));

                let val = if pos.map(|e| e.abs() < 0.5).reduce_and() {
                    1.0f32
                } else {
                    0.0
                };

                buf[j * W + i] = u32::from_le_bytes([(val.max(0.0).min(1.0) * 255.0) as u8; 4]);
            }
        }

        win.update_with_buffer(&buf).unwrap();

        time += 1.0 / 60.0;
    }
}
