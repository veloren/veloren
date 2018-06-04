use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};

use nalgebra::{Vector2, Matrix4};

use client::{ClientHandle, ClientMode};
use camera::Camera;
use window::{RenderWindow, Event};
use model_object::{ModelObject, Constants};
use mesh::{Mesh, Vertex};
use region::Chunk;

pub struct Game {
    client: Arc<Mutex<ClientHandle>>,
    window: Arc<Mutex<RenderWindow>>,
    data: Arc<Mutex<Data>>,
}

struct Data {
    camera: Camera,
    test_model: ModelObject,
}

impl Game {
    pub fn new<B: ToSocketAddrs, R: ToSocketAddrs>(mode: ClientMode, alias: &str, bind_addr: B, remote_addr: R) -> Game {
        let mut window = RenderWindow::new();

        let chunk = Chunk::test((200, 200, 30));
        let mut test_mesh = Mesh::from(&chunk);

        Game {
            data: Arc::new(Mutex::new(Data {
                camera: Camera::new(),
                test_model: ModelObject::new(
                    window.renderer_mut(),
                    &test_mesh,
                ),
            })),
            client: Arc::new(Mutex::new(ClientHandle::new(mode, alias, bind_addr, remote_addr)
                .expect("Could not start client"))),
            window: Arc::new(Mutex::new(window)),
        }
    }

    pub fn handle_window_events(&self) -> bool {
        let mut keep_running = true;
        let mut resized = false;

        self.window.lock().unwrap().handle_events(|event| {
            match event {
                Event::CloseRequest => keep_running = false,
                Event::CursorMoved { dx, dy } => {
                    self.data.lock().unwrap().camera.rotate_by(Vector2::<f32>::new(dx as f32 * 0.005, dy as f32 * 0.005))
                },
                Event::Resized { w, h } => {
                    self.data.lock().unwrap().camera.set_aspect_ratio(w as f32 / h as f32);
                },
                _ => {},
            }
        });

        keep_running
    }

    pub fn update_logic(&self) {
        // Nothing yet
    }

    pub fn render_frame(&self) {
        let mut window = self.window.lock().unwrap();

        window.renderer_mut().begin_frame();

        let camera_mats = self.data.lock().unwrap().camera.get_mats();

        // Render the test model
        window.renderer_mut().update_model_object(
            &self.data.lock().unwrap().test_model,
            Constants::new(&Matrix4::<f32>::identity(), &camera_mats.0, &camera_mats.1)
        );
        window.renderer_mut().render_model_object(&self.data.lock().unwrap().test_model);

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
