use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};

use client::{ClientHandle, ClientMode};
use camera::Camera;
use window::{RenderWindow, Event};

pub struct Game {
    pub client: Arc<Mutex<ClientHandle>>,
    pub window: Arc<Mutex<RenderWindow>>,
    pub camera: Arc<Mutex<Camera>>,
}

impl Game {
    pub fn new<B: ToSocketAddrs, R: ToSocketAddrs>(mode: ClientMode, alias: &str, bind_addr: B, remote_addr: R) -> Game {
        Game {
            client: Arc::new(Mutex::new(ClientHandle::new(mode, alias, bind_addr, remote_addr)
                .expect("Could not start client"))),
            window: Arc::new(Mutex::new(RenderWindow::new())),
            camera: Arc::new(Mutex::new(Camera::new())),
        }
    }

    pub fn handle_window_events(&self) -> bool {
        let mut keep_running = true;

        let mut cam_rot = (0.0, 0.0);
        self.window.lock().unwrap().handle_events(|event| {
            match event {
                Event::CloseRequest => keep_running = false,
                Event::CursorMoved { dx, dy } => cam_rot = (dx as f32, dy as f32),
                _ => {},
            }
        });

        self.camera.lock().unwrap().rotate_by(cam_rot);

        keep_running
    }

    pub fn update_logic(&self) {
        // Nothing yet
    }

    pub fn render_frame(&self) {
        let mut window = self.window.lock().unwrap();
        window.renderer_mut().begin_frame();
        window.swap_buffers();
        window.renderer_mut().end_frame();
    }

    pub fn run(&self) {
        while self.handle_window_events() {
            self.update_logic();
            self.render_frame();
        }
    }
}
