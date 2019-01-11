pub mod camera;

// Crate
use crate::render::{
    Consts,
    Globals,
    Model,
    Renderer,
    SkyboxPipeline,
    SkyboxLocals,
    create_skybox_mesh,
};

// Local
use self::camera::Camera;

struct Skybox {
    model: Model<SkyboxPipeline>,
    locals: Consts<SkyboxLocals>,
}

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
                .create_consts_with(Globals::new())
                .unwrap(),
            skybox: Skybox {
                model: renderer
                    .create_model(&create_skybox_mesh())
                    .unwrap(),
                locals: renderer
                    .create_consts_with(SkyboxLocals::new())
                    .unwrap(),
            },
        }
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
