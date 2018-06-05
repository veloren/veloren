use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};

use nalgebra::{Vector2, Matrix4};

use client::{Client, ClientMode};
use camera::Camera;
use window::{RenderWindow, Event};
use model_object::{ModelObject, Constants};
use mesh::Mesh;
use region::Chunk;

pub struct Game {
    client: Arc<Client>,
    window: Arc<Mutex<RenderWindow>>,
    data: Arc<Mutex<Data>>,
}

// "Data" includes mutable state
struct Data {
    camera: Camera,
    test_model: ModelObject,
    cursor_trapped: bool,
}

impl Game {
    pub fn new<B: ToSocketAddrs, R: ToSocketAddrs>(mode: ClientMode, alias: &str, bind_addr: B, remote_addr: R) -> Game {
        let mut window = RenderWindow::new();

        let chunk = Chunk::test((200, 200, 100));
        let test_mesh = Mesh::from(&chunk);

        let game = Game {
            data: Arc::new(Mutex::new(Data {
                camera: Camera::new(),
                test_model: ModelObject::new(
                    window.renderer_mut(),
                    &test_mesh,
                ),
                cursor_trapped: true,
            })),
            client: Client::new(mode, alias.to_string(), bind_addr, remote_addr)
                .expect("Could not create new client"),
            window: Arc::new(Mutex::new(window)),
        };

        Client::start(game.client.clone());

        game
    }

    pub fn handle_window_events(&self) -> bool {
        let mut keep_running = true;

        self.window.lock().unwrap().handle_events(|event| {
            match event {
                Event::CloseRequest => keep_running = false,
                Event::CursorMoved { dx, dy } => {
                    let mut data = self.data.lock().unwrap();

                    if data.cursor_trapped {
                        data.camera.rotate_by(Vector2::<f32>::new(dx as f32 * 0.002, dy as f32 * 0.002))
                    }
                },
                Event::MouseWheel { dy, .. } => {
                    self.data.lock().unwrap().camera.zoom_by(-dy as f32);
                },
                Event::KeyboardInput { i, .. } => {
                    println!("pressed: {}", i.scancode);
                    match i.scancode {
                        1 => self.data.lock().unwrap().cursor_trapped = false,
                        //W 17 => {},
                        //A 30 => {},
                        //S 31 => {},
                        //D 32 => {},
                        _ => (),
                    }
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
