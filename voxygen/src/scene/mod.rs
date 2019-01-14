pub mod camera;
pub mod figure;

// Standard
use std::time::Duration;

// Library
use vek::*;
use dot_vox;

// Project
use client::{
    self,
    Client,
};
use common::figure::Segment;

// Crate
use crate::{
    Error,
    render::{
        Consts,
        Globals,
        Model,
        Renderer,
        SkyboxPipeline,
        SkyboxLocals,
        FigureLocals,
        create_skybox_mesh,
    },
    window::Event,
    mesh::Meshable,
    anim::{
        Animation,
        CharacterSkeleton,
        RunAnimation,
    },
};

// Local
use self::{
    camera::Camera,
    figure::Figure,
};

// TODO: Don't hard-code this
const CURSOR_PAN_SCALE: f32 = 0.005;

struct Skybox {
    model: Model<SkyboxPipeline>,
    locals: Consts<SkyboxLocals>,
}

pub struct Scene {
    camera: Camera,
    globals: Consts<Globals>,
    skybox: Skybox,

    test_figure: Figure<CharacterSkeleton>,

    client: Client,
}

// TODO: Make a proper asset loading system
fn load_segment(filename: &'static str) -> Segment {
    Segment::from(dot_vox::load(&(concat!(env!("CARGO_MANIFEST_DIR"), "/test_assets/").to_string() + filename)).unwrap())
}

impl Scene {
    /// Create a new `Scene` with default parameters.
    pub fn new(renderer: &mut Renderer) -> Self {
        Self {
            camera: Camera::new(),
            globals: renderer
                .create_consts(&[Globals::default()])
                .unwrap(),
            skybox: Skybox {
                model: renderer
                    .create_model(&create_skybox_mesh())
                    .unwrap(),
                locals: renderer
                    .create_consts(&[SkyboxLocals::default()])
                    .unwrap(),
            },

            test_figure: Figure::new(
                renderer,
                [
                    Some(load_segment("head.vox").generate_mesh_with_offset(Vec3::new(-7.0, -5.5, -1.0))),
                    Some(load_segment("chest.vox").generate_mesh_with_offset(Vec3::new(-6.0, -3.0, 0.0))),
                    Some(load_segment("belt.vox").generate_mesh_with_offset(Vec3::new(-5.0, -3.0, 0.0))),
                    Some(load_segment("pants.vox").generate_mesh_with_offset(Vec3::new(-5.0, -3.0, 0.0))),
                    Some(load_segment("hand.vox").generate_mesh_with_offset(Vec3::new(-2.0, -2.0, -1.0))),
                    Some(load_segment("hand.vox").generate_mesh_with_offset(Vec3::new(-2.0, -2.0, -1.0))),
                    Some(load_segment("foot.vox").generate_mesh_with_offset(Vec3::new(-2.5, -3.0, 0.0))),
                    Some(load_segment("foot.vox").generate_mesh_with_offset(Vec3::new(-2.5, -3.0, 0.0))),
                    Some(load_segment("sword.vox").generate_mesh_with_offset(Vec3::new(-6.5, -1.0, 0.0))),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ],
                CharacterSkeleton::new(),
            )
                .unwrap(),

            client: Client::new(),
        }
    }

    /// Tick the scene (and the client attached to it)
    pub fn tick(&mut self, dt: Duration) -> Result<(), Error> {
        self.client.tick(client::Input {}, dt)?;
        Ok(())
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
        renderer.update_consts(&mut self.globals, &[Globals::new(
            view_mat,
            proj_mat,
            cam_pos,
            self.camera.get_focus_pos(),
            10.0,
            self.client.state().get_time_of_day(),
            0.0,
        )])
            .expect("Failed to update global constants");

        // TODO: Don't do this here
        RunAnimation::update_skeleton(
            &mut self.test_figure.skeleton,
            self.client.state().get_tick(),
        );
        self.test_figure.update_locals(renderer, FigureLocals::default());
        self.test_figure.update_skeleton(renderer);
    }

    /// Render the scene using the provided `Renderer`
    pub fn render_to(&self, renderer: &mut Renderer) {
        // Render the skybox first (it appears over everything else so must be rendered first)
        renderer.render_skybox(
            &self.skybox.model,
            &self.globals,
            &self.skybox.locals,
        );

        // Render the test figure
        self.test_figure.render(renderer, &self.globals);
    }
}
