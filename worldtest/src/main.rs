extern crate sfml;
extern crate worldgen;

use sfml::system::Vector2f;
use sfml::window::{ContextSettings, VideoMode, Event, Style};
use sfml::graphics::{RectangleShape, Color, RenderTarget, RenderWindow, Shape, Transformable};

use worldgen::MacroWorld;

const WORLD_SEED: u32 = 1337;
const WORLD_SIZE: u32 = 800;

fn main() {
    let mw = MacroWorld::new(WORLD_SEED, WORLD_SIZE);

    let mut win = RenderWindow::new(
        VideoMode::new(mw.size(), mw.size(), 32),
        "Veloren World Test",
        Style::CLOSE,
        &ContextSettings::default()
    );

    loop {
        loop {
            match win.poll_event() {
                Some(Event::Closed) => return,
                None => break,
                _ => {},
            }
        }

        win.clear(&Color::rgb(0, 0, 0));

        let mut pixel = RectangleShape::with_size(Vector2f::new(1., 1.));
        for x in 0..mw.size() {
            for y in 0..mw.size() {
                let alt = mw.get(x, y).unwrap().altitude() as u8;
                pixel.set_fill_color(&Color::rgb(alt, alt, alt));
                pixel.set_position(Vector2f::new(x as f32, y as f32));
                win.draw(&pixel);
            }
        }

        win.display();
    }
}
