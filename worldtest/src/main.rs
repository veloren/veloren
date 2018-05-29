extern crate sfml;
extern crate worldgen;

use sfml::system::Vector2f;
use sfml::window::{ContextSettings, VideoMode, Event, Style};
use sfml::graphics::{RectangleShape, Color, RenderTarget, RenderWindow, Shape, Transformable};

use worldgen::{MacroWorld, Biome};

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
                let chunk = mw.get(x, y).unwrap();
                let alt = chunk.altitude() as u8;
                match chunk.biome() {
                    Biome::Ocean => pixel.set_fill_color(&Color::rgba(0, 50, 255, alt)),
                    Biome::Grassland => pixel.set_fill_color(&Color::rgba(50, 255, 0, alt)),
                    Biome::River => pixel.set_fill_color(&Color::rgba(0, 50, 255, alt)),
                    Biome::Sand => pixel.set_fill_color(&Color::rgba(255, 255, 0, alt)),
                    Biome::Mountain => pixel.set_fill_color(&Color::rgba(150, 150, 150, alt)),
                }

                pixel.set_position(Vector2f::new(x as f32, y as f32));
                win.draw(&pixel);
            }
        }

        win.display();
    }
}
