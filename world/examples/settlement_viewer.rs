use rand::thread_rng;
use vek::*;
use veloren_world::generator::settlement::Settlement;

const W: usize = 640;
const H: usize = 480;

fn main() {
    let mut win =
        minifb::Window::new("Settlement Viewer", W, H, minifb::WindowOptions::default()).unwrap();

    let settlement = Settlement::generate(Vec2::zero(), None, &mut thread_rng());

    let mut focus = Vec2::<f32>::zero();
    let mut zoom = 1.0;

    while win.is_open() {
        let mut buf = vec![0; W * H];

        let win_to_pos =
            |wp: Vec2<usize>| (wp.map(|e| e as f32) - Vec2::new(W as f32, H as f32) * 0.5) * zoom;

        for i in 0..W {
            for j in 0..H {
                let pos = focus + win_to_pos(Vec2::new(i, j)) * zoom;

                let color = settlement
                    .get_color(pos.map(|e| e.floor() as i32))
                    .unwrap_or(Rgb::new(35, 50, 20));

                buf[j * W + i] = u32::from_le_bytes([color.b, color.g, color.r, 255]);
            }
        }

        let spd = 20.0;
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
            zoom *= 1.05;
        }
        if win.is_key_down(minifb::Key::E) {
            zoom /= 1.05;
        }

        win.update_with_buffer(&buf).unwrap();
    }
}
