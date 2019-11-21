use crate::render::{
    pipelines::lod_terrain::{Locals, Vertex},
    Consts, Globals, LodTerrainPipeline, Mesh, Model, Quad, Renderer,
};
use vek::*;

pub struct Lod {
    model: Model<LodTerrainPipeline>,
    locals: Consts<Locals>,
}

impl Lod {
    pub fn new(renderer: &mut Renderer) -> Self {
        Self {
            model: renderer
                .create_model(&create_lod_terrain_mesh(100))
                .unwrap(),
            locals: renderer.create_consts(&[Locals::default()]).unwrap(),
        }
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        renderer.render_lod_terrain(&self.model, globals, &self.locals);
    }
}

fn create_lod_terrain_mesh(detail: usize) -> Mesh<LodTerrainPipeline> {
    let transform = |x| (2.0 * x as f32) / detail as f32 - 1.0;

    let mut mesh = Mesh::new();

    for x in 0..detail {
        for y in 0..detail {
            mesh.push_quad(Quad::new(
                Vertex::new(Vec2::new(x + 0, y + 0).map(transform)),
                Vertex::new(Vec2::new(x + 1, y + 0).map(transform)),
                Vertex::new(Vec2::new(x + 1, y + 1).map(transform)),
                Vertex::new(Vec2::new(x + 0, y + 1).map(transform)),
            ));
        }
    }

    mesh
}
