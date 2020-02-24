use crate::render::{
    pipelines::lod_terrain::{Locals, Vertex},
    Consts, FilterMethod, Globals, LodTerrainPipeline, Mesh, Model, Quad, Renderer, Texture,
};
use client::Client;
use common::spiral::Spiral2d;
use vek::*;

pub struct Lod {
    model: Model<LodTerrainPipeline>,
    locals: Consts<Locals>,
    map: Texture,
}

impl Lod {
    pub fn new(renderer: &mut Renderer, client: &Client) -> Self {
        Self {
            model: renderer
                .create_model(&create_lod_terrain_mesh(500))
                .unwrap(),
            locals: renderer.create_consts(&[Locals::default()]).unwrap(),
            map: renderer
                .create_texture(&client.world_map.0, Some(FilterMethod::Trilinear), None)
                .expect("Failed to generate map texture"),
        }
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        renderer.render_lod_terrain(&self.model, globals, &self.locals, &self.map);
    }
}

fn create_lod_terrain_mesh(detail: usize) -> Mesh<LodTerrainPipeline> {
    Spiral2d::new()
        .take(detail * detail)
        .map(|pos| {
            let x = pos.x + detail as i32 / 2;
            let y = pos.y + detail as i32 / 2;

            let transform = |x| (2.0 * x as f32) / detail as f32 - 1.0;

            Quad::new(
                Vertex::new(Vec2::new(x + 0, y + 0).map(transform)),
                Vertex::new(Vec2::new(x + 1, y + 0).map(transform)),
                Vertex::new(Vec2::new(x + 1, y + 1).map(transform)),
                Vertex::new(Vec2::new(x + 0, y + 1).map(transform)),
            )
            .rotated_by(if (x > detail as i32 / 2) ^ (y > detail as i32 / 2) {
                0
            } else {
                1
            })
        })
        .collect()
}
