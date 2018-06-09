use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
//use std::f32::{sin, cos};

use nalgebra::{Vector2, Vector3, Matrix4, Translation3};
use glutin::ElementState;

use client::{Client, ClientMode};
use camera::Camera;
use window::{RenderWindow, Event};
use model_object::{ModelObject, Constants};
use mesh::{Mesh, Vertex};
use region::Chunk;
use key_state::KeyState;
use std::sync::RwLock;
use std::sync::RwLockWriteGuard;

pub struct Game {
    running: AtomicBool,
    client: Arc<RwLock<Client>>,
    window: Arc<Mutex<RenderWindow>>,
    data: Mutex<Data>,
    camera: Mutex<Camera>,
    key_state: Mutex<KeyState>,
}

// "Data" includes mutable state
struct Data {
    player_model: ModelObject,
    test_model: ModelObject,
    cursor_trapped: bool,
}

impl Game {
    pub fn new<B: ToSocketAddrs, R: ToSocketAddrs>(mode: ClientMode, alias: &str, bind_addr: B, remote_addr: R) -> Game {
        let mut window = RenderWindow::new();

        let chunk = Chunk::test((100, 100, 100));
        let test_mesh = Mesh::from(&chunk);

        let mut player_mesh = Mesh::new();
        player_mesh.add(&[
            Vertex { pos: [0., 1., 0.], norm: [0., 0., 1.], col: [1., 0., 0., 1.] },
            Vertex { pos: [-1., -1., 0.], norm: [0., 0., 1.], col: [0., 1., 0., 1.] },
            Vertex { pos: [1., -1., 0.], norm: [0., 0., 1.], col: [0., 0., 1., 1.] },

            Vertex { pos: [0., 1., 0.], norm: [0., 0., 1.], col: [1., 0., 0., 1.] },
            Vertex { pos: [1., -1., 0.], norm: [0., 0., 1.], col: [0., 0., 1., 1.] },
            Vertex { pos: [-1., -1., 0.], norm: [0., 0., 1.], col: [0., 1., 0., 1.] },
        ]);

        Game {
            data: Mutex::new(Data {
                player_model: ModelObject::new(
                    window.renderer_mut(),
                    &player_mesh,
                ),
                test_model: ModelObject::new(
                    window.renderer_mut(),
                    &test_mesh,
                ),
                cursor_trapped: true,
            }),
            running: AtomicBool::new(true),
            client: Client::new(mode, alias.to_string(), remote_addr)
				.expect("Could not create new client"),
            window: Arc::new(Mutex::new(window)),
            camera: Mutex::new(Camera::new()),
            key_state: Mutex::new(KeyState::new()),
        }
    }

    pub fn get_client(&self) -> RwLockWriteGuard<Client> { self.client.write().unwrap() }

    pub fn handle_window_events(&self) -> bool {
        self.window.lock().unwrap().handle_events(|event| {
            match event {
                Event::CloseRequest => self.running.store(false, Ordering::Relaxed),
                Event::CursorMoved { dx, dy } => {
                    let mut data = self.data.lock().unwrap();

                    if data.cursor_trapped {
                        self.camera.lock().unwrap().rotate_by(Vector2::<f32>::new(dx as f32 * 0.002, dy as f32 * 0.002))
                    }
                },
                Event::MouseWheel { dy, .. } => {
                    self.camera.lock().unwrap().zoom_by(-dy as f32);
                },
                Event::KeyboardInput { i, .. } => {
                    match i.scancode {
                        1 => self.data.lock().unwrap().cursor_trapped = false,
                        17 => self.key_state.lock().unwrap().up = match i.state { // W (up)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        },
                        30 => self.key_state.lock().unwrap().left = match i.state { // A (left)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        },
                        31 => self.key_state.lock().unwrap().down = match i.state { // S (down)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        },
                        32 => self.key_state.lock().unwrap().right = match i.state { // D (right)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        },
                        57 => self.key_state.lock().unwrap().fly = match i.state { // Space (fly)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        },
                        42 => self.key_state.lock().unwrap().fall = match i.state { // Shift (fall)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        },
                        _ => (),
                    }
                },
                Event::Resized { w, h } => {
                    self.camera.lock().unwrap().set_aspect_ratio(w as f32 / h as f32);
                },
                _ => {},
            }
        });

        // Calculate movement player movement vector
        let ori = self.camera.lock().unwrap().ori();
        let unit_vecs = (
            Vector2::new(f32::cos(-ori.x), f32::sin(-ori.x)),
            Vector2::new(f32::sin(ori.x), f32::cos(ori.x))
        );
        let dir_vec = self.key_state.lock().unwrap().dir_vec();
        let mov_vec = unit_vecs.0 * dir_vec.x + unit_vecs.1 * dir_vec.y;
        let fly_vec = self.key_state.lock().unwrap().fly_vec();

        self.get_client().player_mut().dir_vec = Vector3::<f32>::new(mov_vec.x, mov_vec.y, fly_vec);

        self.running.load(Ordering::Relaxed)
    }

    pub fn render_frame(&self) {
        let mut window = self.window.lock().unwrap();

        window.renderer_mut().begin_frame();

        if let Some(uid) = self.get_client().player().entity_uid {
            if let Some(e) = self.get_client().entities().get(&uid) {
                self.camera.lock().unwrap().set_focus(*e.pos());
            }
        }

        let camera_mats = self.camera.lock().unwrap().get_mats();

        // Render the test model
        window.renderer_mut().update_model_object(
            &self.data.lock().unwrap().test_model,
            Constants::new(&Matrix4::<f32>::identity(), &camera_mats.0, &camera_mats.1)
        );
        window.renderer_mut().render_model_object(&self.data.lock().unwrap().test_model);

        for (uid, entity) in self.get_client().entities().iter() {
            window.renderer_mut().update_model_object(
                &self.data.lock().unwrap().player_model,
                Constants::new(
                    &Translation3::<f32>::from_vector(*entity.pos()).to_homogeneous(),
                    &camera_mats.0,
                    &camera_mats.1
                )
            );
            window.renderer_mut().render_model_object(&self.data.lock().unwrap().player_model);
        }

        window.swap_buffers();
        window.renderer_mut().end_frame();
    }

    pub fn run(&self) {
        while self.handle_window_events() {
            self.render_frame();
        }

		self.get_client().shutdown();
    }
}
