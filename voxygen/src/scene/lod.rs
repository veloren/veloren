use crate::{
    render::{
        pipelines::lod_terrain::{Locals, Vertex},
        Consts, FilterMethod, Globals, LodTerrainPipeline, Mesh, Model, Quad, Renderer, Texture,
    },
    settings::Settings,
};
use client::Client;
use common::spiral::Spiral2d;
use vek::*;

pub struct Lod {
    model: Option<(u32, Model<LodTerrainPipeline>)>,
    locals: Consts<Locals>,
    map: Texture,
    tgt_detail: u32,
}

impl Lod {
    pub fn new(renderer: &mut Renderer, client: &Client, settings: &Settings) -> Self {
        Self {
            model: None,
            locals: renderer.create_consts(&[Locals::default()]).unwrap(),
            map: renderer
                .create_texture(&client.world_map.0, Some(FilterMethod::Trilinear), None)
                .expect("Failed to generate map texture"),
            tgt_detail: settings.graphics.lod_detail.max(100).min(2500),
        }
    }

    pub fn set_detail(&mut self, detail: u32) { self.tgt_detail = detail.max(100).min(2500); }

    pub fn maintain(&mut self, renderer: &mut Renderer) {
        if self
            .model
            .as_ref()
            .map(|(detail, _)| *detail != self.tgt_detail)
            .unwrap_or(true)
        {
            self.model = Some((
                self.tgt_detail,
                renderer
                    .create_model(&create_lod_terrain_mesh(self.tgt_detail))
                    .unwrap(),
            ));
        }
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        if let Some((_, model)) = self.model.as_ref() {
            renderer.render_lod_terrain(&model, globals, &self.locals, &self.map);
        }
    }
}

fn create_lod_terrain_mesh(detail: u32) -> Mesh<LodTerrainPipeline> {
    Spiral2d::new()
        .take((detail * detail) as usize)
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
