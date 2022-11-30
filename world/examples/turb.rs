use noise::{NoiseFn, Seedable, SuperSimplex, Turbulence};

use vek::*;

const W: usize = 640;
const H: usize = 640;

fn main() {
    let mut win = minifb::Window::new("Turb", W, H, minifb::WindowOptions::default()).unwrap();

    let nz = Turbulence::new(
        Turbulence::new(SuperSimplex::new())
            .set_frequency(0.2)
            .set_power(1.5),
    )
    .set_frequency(2.0)
    .set_power(0.2);

    let _nz_x = SuperSimplex::new().set_seed(0);
    let _nz_y = SuperSimplex::new().set_seed(1);

    let mut _time = 0.0f64;

    let mut scale = 50.0;

    while win.is_open() {
        let mut buf = vec![0; W * H];

        for i in 0..W {
            for j in 0..H {
                let pos = Vec2::new(i as f64, j as f64) / scale;

                let val = nz.get(pos.into_array());

                buf[j * W + i] = u32::from_le_bytes([(val.clamp(0.0, 1.0) * 255.0) as u8; 4]);
            }
        }

        if win.is_key_pressed(minifb::Key::Right, minifb::KeyRepeat::No) {
            scale *= 1.5;
        } else if win.is_key_pressed(minifb::Key::Left, minifb::KeyRepeat::No) {
            scale /= 1.5;
        }

        win.update_with_buffer(&buf, W, H).unwrap();

        _time += 1.0 / 60.0;
    }
}
