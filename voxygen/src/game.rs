// Standard
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::f32::consts::PI;
//use std::f32::{sin, cos};

// Library
use nalgebra::{Vector2, Vector3, Translation3, Rotation3, convert, dot};
use coord::prelude::*;
use glutin::{ElementState, VirtualKeyCode};
use dot_vox;

// Project
use client::{Client, ClientMode};

// Local
use map::Map;
use camera::Camera;
use window::{RenderWindow, Event};
use model_object::{ModelObject, Constants};
use mesh::{Mesh};
use region::Chunk;
use key_state::KeyState;
use vox::vox_to_model;

pub struct Game {
    running: AtomicBool,
    client: Arc<Client>,
    window: RenderWindow,
    data: Mutex<Data>,
    camera: Mutex<Camera>,
    key_state: Mutex<KeyState>,
}

// "Data" includes mutable state
struct Data {
    player_model: ModelObject,
    other_player_model: ModelObject,
    map: Map,
}

impl Game {
    pub fn new<R: ToSocketAddrs>(mode: ClientMode, alias: &str, remote_addr: R) -> Game {
        let window = RenderWindow::new();

        let vox = dot_vox::load("vox/3.vox").unwrap();
        let voxmodel = vox_to_model(vox);

        let player_mesh = Mesh::from(&voxmodel);

        let player_model = ModelObject::new(
            &mut window.renderer_mut(),
            &player_mesh,
        );

        let vox = dot_vox::load("vox/5.vox").unwrap();
        let voxmodel = vox_to_model(vox);

        let other_player_mesh = Mesh::from(&voxmodel);

        let other_player_model = ModelObject::new(
            &mut window.renderer_mut(),
            &other_player_mesh,
        );

        let chunk = Chunk::test(vec3!(0, 0, 0), vec3!(100,100,100));
        let test_mesh = Mesh::from(&chunk);

        let test_model = ModelObject::new(
            &mut window.renderer_mut(),
            &test_mesh,
        );

        let mut map = Map::new();
        map.chunks().insert(Vector3::new(0,0,0), test_model);

        let chunk = Chunk::test(Vec3::from((100,0,0)),Vec3::from((100,100,100)));
        let test_mesh = Mesh::from(&chunk);

        let test_model = ModelObject::new(
            &mut window.renderer_mut(),
            &test_mesh,
        );

        map.chunks().insert(Vector3::new(100,0,0), test_model);

        Game {
            data: Mutex::new(Data {
                player_model,
                other_player_model,
                map,
            }),
            running: AtomicBool::new(true),
            client: Client::new(mode, alias.to_string(), remote_addr)
				.expect("Could not create new client"),
            window,
            camera: Mutex::new(Camera::new()),
            key_state: Mutex::new(KeyState::new()),
        }
    }

    pub fn handle_window_events(&self) -> bool {
        self.window.handle_events(|event| {
            match event {
                Event::CloseRequest => self.running.store(false, Ordering::Relaxed),
                Event::CursorMoved { dx, dy } => {
                    let data = self.data.lock().unwrap();

                    if self.window.cursor_trapped().load(Ordering::Relaxed) {
                        //debug!("dx: {}, dy: {}", dx, dy);
                        self.camera.lock().unwrap().rotate_by(Vector2::<f32>::new(dx as f32 * 0.002, dy as f32 * 0.002));
                    }
                },
                Event::MouseWheel { dy, .. } => {
                    self.camera.lock().unwrap().zoom_by((-dy / 4.0) as f32);
                },
                Event::KeyboardInput { i, .. } => {
                    match i.virtual_keycode {
                        Some(VirtualKeyCode::Escape) => self.window.cursor_trapped().store(false, Ordering::Relaxed),
                        Some(VirtualKeyCode::W) => self.key_state.lock().unwrap().up = match i.state { // W (up)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        },
                        Some(VirtualKeyCode::A) => self.key_state.lock().unwrap().left = match i.state { // A (left)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        },
                        Some(VirtualKeyCode::S) => self.key_state.lock().unwrap().down = match i.state { // S (down)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        },
                        Some(VirtualKeyCode::D) => self.key_state.lock().unwrap().right = match i.state { // D (right)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        },
                        Some(VirtualKeyCode::Space) => self.key_state.lock().unwrap().fly = match i.state { // Space (fly)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        },
                        Some(VirtualKeyCode::LShift) => self.key_state.lock().unwrap().fall = match i.state { // Shift (fall)
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        },
                        _ => (),
                    }
                },
                Event::Resized { w, h } => {
                    self.camera.lock().unwrap().set_aspect_ratio(w as f32 / h as f32);
                },
                //_ => {},
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

        //self.client.player_mut().dir_vec = vec3!(mov_vec.x, mov_vec.y, fly_vec);

        let mut entries = self.client.entities_mut();
        if let Some(eid) = self.client.player().entity_uid {
            if let Some(player_entry) = entries.get_mut(&eid) {
                *player_entry.move_dir_mut() = vec3!(mov_vec.x, mov_vec.y, fly_vec);
                let ori = self.camera.lock().unwrap().ori();
                *player_entry.look_dir_mut() = vec2!(ori.x, ori.y);
            }
        }

        self.running.load(Ordering::Relaxed)
    }

    pub fn render_frame(&self) {
        let mut renderer = self.window.renderer_mut();
        renderer.begin_frame();

        if let Some(uid) = self.client.player().entity_uid {
            if let Some(e) = self.client.entities().get(&uid) {
                self.camera.lock().unwrap().set_focus(Vector3::<f32>::new(e.pos().x, e.pos().y, e.pos().z)); // TODO: Improve this
            }
        }

        let camera_mats = self.camera.lock().unwrap().get_mats();
        let camera_ori = self.camera.lock().unwrap().ori();

        // Render the test model

        for (pos, model) in self.data.lock().unwrap().map.chunks() {
            renderer.update_model_object(
                &model,
                Constants::new(//&Matrix4::<f32>::identity(),
                    &Translation3::<f32>::from_vector(convert(*pos)).to_homogeneous(),
                    &camera_mats.0,
                    &camera_mats.1)
            );
            renderer.render_model_object(&model);
        }

        for (eid, entity) in self.client.entities().iter() {
            let model_mat = &Translation3::<f32>::from_vector(Vector3::<f32>::new(entity.pos().x, entity.pos().y, entity.pos().z)).to_homogeneous();
            let rot = Rotation3::<f32>::new(Vector3::<f32>::new(0.0, 0.0, PI - entity.look_dir().x)).to_homogeneous();
            let model_mat = model_mat * rot;
            let mut data = self.data.lock().unwrap();
            let ref mut model;
            match self.client.player().entity_uid {
                Some(uid) if uid == *eid => model = &mut data.player_model,
                _ => model = &mut data.other_player_model,
            }
            renderer.update_model_object(
                &model,
                Constants::new(
                    &model_mat, // TODO: Improve this
                    &camera_mats.0,
                    &camera_mats.1,
                )
            );
            renderer.render_model_object(&model);
        }

        self.window.swap_buffers();
        renderer.end_frame();
    }

    pub fn run(&self) {
        while self.handle_window_events() {
            self.render_frame();
        }

		self.client.shutdown();
    }
}
