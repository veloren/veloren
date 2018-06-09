#![feature(nll)]

extern crate worldgen;
extern crate sfml;

use sfml::system::Vector2f;
use sfml::window::{ContextSettings, VideoMode, Event, Style};
use sfml::graphics::{RectangleShape, Vertex, PrimitiveType, Color, RenderTarget, RenderStates, RenderWindow, Shape, Transformable};

use worldgen::{World, Biome};

const WORLD_SEED: u32 = 1337;
const WORLD_SIZE: u32 = 256;

fn main() {
    let mut world = World::new(WORLD_SEED, WORLD_SIZE);

    let mut win = RenderWindow::new(
        VideoMode::new(world.map().size(), world.map().size(), 32),
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

        world.tick(8.0);

        win.clear(&Color::rgb(0, 0, 0));

        let mut pixel = RectangleShape::with_size(Vector2f::new(1., 1.));
        for x in 0..world.map().size() {
            for y in 0..world.map().size() {
                let chunk = world.map().get(x, y).unwrap();
                let alt = chunk.altitude() as u8;
                let (hue, sat) = match chunk.biome() {
                    Biome::Ocean => (250.0, 1.0),
                    Biome::Grassland => (120.0, 1.0), //pixel.set_fill_color(&Color::rgba(50, 255, 0, alt)),
                    Biome::River => (120.0, 1.0), //pixel.set_fill_color(&Color::rgba(0, 50, 255, alt)),
                    Biome::Sand => (45.0, 1.0), //pixel.set_fill_color(&Color::rgba(255, 255, 0, alt)),
                    Biome::Mountain => (120.0, 0.3), //pixel.set_fill_color(&Color::rgba(150, 150, 150, alt)),
                };

                let color = hsv2rgb((hue, sat, alt as f64 / 256.));
                pixel.set_fill_color(&Color::rgba((color.0 * 255.) as u8, (color.1 * 255.) as u8, (color.2 * 255.) as u8, 255));

                pixel.set_position(Vector2f::new(x as f32, y as f32));
                win.draw(&pixel);

                if x % 6 == 0 && y % 6 == 0 {
                    let verts = [
                        Vertex::new(Vector2f::new(x as f32, y as f32), Color::RED, Vector2f::new(0.0, 0.0)),
                        Vertex::new(Vector2f::new(
                            x as f32 + chunk.wind().x * 30.0,
                            y as f32 + chunk.wind().y * 30.0
                        ), Color::RED, Vector2f::new(0.0, 0.0)),
                    ];
                    win.draw_primitives(&verts, PrimitiveType::Lines, RenderStates::default());
                }
            }
        }

        win.display();
    }
}


fn hsv2rgb((h, s, v): (f64, f64, f64)) -> (f64, f64, f64) {
    if s <= 0.0 {
        return (v,v,v);
    }
    let mut hh = h;
    if hh >= 360.0 {
        hh = 0.0;
    }
    hh = hh / 60.0;
    let i = hh.floor() as u64;
    let ff = hh - i as f64;
    let p = v * (1.0 - s);
    let q = v * (1.0 - (s * ff));
    let t = v * (1.0 - (s * (1.0 - ff)));
    //println!("hsv: i {} {} {} {} {}", i, p,q,t,v);
    match i {
        0 => (v,t,p),
        1 => (q,v,p),
        2 => (p,v,t),
        3 => (p,q,v),
        4 => (t,p,v),
        5 => (v,p,q),
        _ => panic!("Unexpected value in hsv2rgb: i: {} h: {}", i, h),
    }
}
