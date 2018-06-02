use std::io;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use get_if_addrs;

use client::{ClientHandle, ClientMode};

use window::{RenderWindow, Event};
use camera::Camera;

pub struct Game {
    pub client: ClientHandle,
    pub window: RenderWindow,
    pub camera: Camera,
}

pub struct GameHandle {
    game: Arc<Mutex<Game>>,
}

impl GameHandle {
    pub fn new(alias: &str) -> GameHandle {
        // TODO: Seriously? This needs to go. Make it auto-detect this stuff
        // <rubbish>
        let ip = get_if_addrs::get_if_addrs().unwrap()[0].ip();

        let mut port = String::new();
        println!("Local port [59001]:");
        io::stdin().read_line(&mut port).unwrap();
        let port = u16::from_str_radix(&port.trim(), 10).unwrap();

        println!("Binding to {}:{}...", ip.to_string(), port);

        let mut remote_addr = String::new();
        println!("Remote server address:");
        io::stdin().read_line(&mut remote_addr).unwrap();
        // </rubbish>

        GameHandle {
            game: Arc::new(Mutex::new(Game {
                client: ClientHandle::new(ClientMode::Game, &alias, SocketAddr::new(ip, port), remote_addr.trim())
                    .expect("Could not start client"),
                window: RenderWindow::new(),
                camera: Camera::new(),
            })),
        }
    }

    pub fn next_frame(&self) -> bool {
        // Handle window events
        let mut running = true;

        self.game.lock().unwrap().window.handle_events(|event| {
            match event {
                Event::CloseRequest => running = false,
                Event::CursorMoved { dx, dy } => {}, //game.camera.rotate_by((dx as f32, dy as f32)),
                _ => {},
            }
        });

        // Renderer the game
        self.game.lock().unwrap().window.renderer_mut().begin_frame();

        // Swap buffers, clean things up
        self.game.lock().unwrap().window.swap_buffers();
        self.game.lock().unwrap().window.renderer_mut().end_frame();

        running
    }
}
