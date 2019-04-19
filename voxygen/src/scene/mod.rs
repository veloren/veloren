pub mod camera;
pub mod figure;
pub mod terrain;

use vek::*;
use dot_vox;
use common::{
    comp,
    figure::Segment,
};
use client::Client;
use crate::{
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
    mesh::Meshable, anim::{
        Animation,
        character::{CharacterSkeleton, RunAnimation},
    },
};
use self::{
    camera::Camera,
    figure::FigureCache,
    terrain::Terrain,
};

// TODO: Don't hard-code this
const CURSOR_PAN_SCALE: f32 = 0.005;

struct Skybox {
    model: Model<SkyboxPipeline>,
    locals: Consts<SkyboxLocals>,
}

pub struct Scene {
    globals: Consts<Globals>,
    camera: Camera,

    skybox: Skybox,
    terrain: Terrain,

    figure_cache: FigureCache,
}

impl Scene {
    /// Create a new `Scene` with default parameters.
    pub fn new(renderer: &mut Renderer, client: &Client) -> Self {
        let resolution = renderer.get_resolution().map(|e| e as f32);

        Self {
            globals: renderer
                .create_consts(&[Globals::default()])
                .unwrap(),
            camera: Camera::new(resolution.x / resolution.y),

            skybox: Skybox {
                model: renderer
                    .create_model(&create_skybox_mesh())
                    .unwrap(),
                locals: renderer
                    .create_consts(&[SkyboxLocals::default()])
                    .unwrap(),
            },
            terrain: Terrain::new(),
            figure_cache: FigureCache::new(),
        }
    }

    /// Get a reference to the scene's camera.
    pub fn camera(&self) -> &Camera { &self.camera }

    /// Get a mutable reference to the scene's camera.
    pub fn camera_mut(&mut self) -> &mut Camera { &mut self.camera }

    /// Handle an incoming user input event (i.e: cursor moved, key pressed, window closed, etc.).
    ///
    /// If the event is handled, return true
    pub fn handle_input_event(&mut self, event: Event) -> bool {
        match event {
            // When the window is resized, change the camera's aspect ratio
            Event::Resize(dims) => {
                self.camera.set_aspect_ratio(dims.x as f32 / dims.y as f32);
                true
            },
            // Panning the cursor makes the camera rotate
            Event::CursorPan(delta) => {
                self.camera.rotate_by(Vec3::from(delta) * CURSOR_PAN_SCALE);
                true
            },
            // Zoom the camera when a zoom event occurs
            Event::Zoom(delta) => {
                self.camera.zoom_by(delta * 0.3);
                true
            },
            // All other events are unhandled
            _ => false,
        }
    }

    /// Maintain data such as GPU constant buffers, models, etc. To be called once per tick.
    pub fn maintain(&mut self, renderer: &mut Renderer, client: &mut Client) {
        // Get player position
        let player_pos = client
            .state()
            .ecs()
            .read_storage::<comp::phys::Pos>()
            .get(client.entity())
            .map(|pos| pos.0)
            .unwrap_or(Vec3::zero());

        // Alter camera position to match player
        self.camera.set_focus_pos(player_pos + Vec3::unit_z() * 1.5);

        // Compute camera matrices
        let (view_mat, proj_mat, cam_pos) = self.camera.compute_dependents();

        // Update global constants
        renderer.update_consts(&mut self.globals, &[Globals::new(
            view_mat,
            proj_mat,
            cam_pos,
            self.camera.get_focus_pos(),
            10.0,
            client.state().get_time_of_day(),
            client.state().get_time(),
        )])
            .expect("Failed to update global constants");

        // Maintain the terrain
        self.terrain.maintain(renderer, client);

        // Maintain the figures
        self.figure_cache.maintain(renderer, client);
    }

    /// Render the scene using the provided `Renderer`
    pub fn render(&mut self, renderer: &mut Renderer, client: &Client) {
        // Render the skybox first (it appears over everything else so must be rendered first)
        renderer.render_skybox(
            &self.skybox.model,
            &self.globals,
            &self.skybox.locals,
        );

        // Render terrain and figures
        self.terrain.render(renderer, &self.globals);
        self.figure_cache.render(renderer, client, &self.globals);
    }
}
