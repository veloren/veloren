pub mod camera;

// Library
use vek::*;

// Crate
use crate::{
    render::{
        Consts,
        Globals,
        Model,
        Renderer,
        SkyboxPipeline,
        SkyboxLocals,
        create_skybox_mesh,
    },
    window::Event,
};

// Local
use self::camera::Camera;

struct Skybox {
    model: Model<SkyboxPipeline>,
    locals: Consts<SkyboxLocals>,
}

// TODO: Don't hard-code this
const CURSOR_PAN_SCALE: f32 = 0.005;

pub struct Scene {
    camera: Camera,
    globals: Consts<Globals>,
    skybox: Skybox,
}

impl Scene {
    /// Create a new `Scene` with default parameters.
    pub fn new(renderer: &mut Renderer) -> Self {
        Self {
            camera: Camera::new(),
            globals: renderer
                .create_consts_with(Globals::default())
                .unwrap(),
            skybox: Skybox {
                model: renderer
                    .create_model(&create_skybox_mesh())
                    .unwrap(),
                locals: renderer
                    .create_consts_with(SkyboxLocals::default())
                    .unwrap(),
            },
        }
    }

    /// Handle an incoming user input event (i.e: cursor moved, key pressed, window closed, etc.).
    pub fn handle_input_event(&mut self, event: Event) -> bool {
        match event {
            // Panning the cursor makes the camera rotate
            Event::CursorPan(delta) => {
                self.camera.rotate_by(Vec3::from(delta) * CURSOR_PAN_SCALE);
                true
            },
            // All other events are unhandled
            _ => false,
        }
    }

    /// Maintain and update GPU data such as constant buffers, models, etc.
    pub fn maintain_gpu_data(&mut self, renderer: &mut Renderer) {
        // Compute camera matrices
        let (view_mat, proj_mat, cam_pos) = self.camera.compute_dependents();

        // Update global constants
        renderer.update_consts(&mut self.globals, Globals::new(
            view_mat,
            proj_mat,
            cam_pos,
            self.camera.get_focus_pos(),
            10.0,
            0.0,
            0.0,
        ))
            .expect("Failed to update global constants");
    }

    /// Render the scene using the provided `Renderer`
    pub fn render_to(&self, renderer: &mut Renderer) {
        // Render the skybox first (it appears over everything else so must be rendered first)
        renderer.render_skybox(
            &self.skybox.model,
            &self.skybox.locals,
            &self.globals,
        );
    }
}
