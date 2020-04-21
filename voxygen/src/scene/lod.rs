use crate::{
    render::{
        pipelines::lod_terrain::{Locals, Vertex},
        Consts, FilterMethod, Globals, LodColorFmt, LodTerrainPipeline, LodTextureFmt, Mesh, Model,
        Quad, Renderer, Texture, WrapMode,
    },
    settings::Settings,
};
use client::Client;
use common::{spiral::Spiral2d, util::srgba_to_linear};
use vek::*;

pub struct Lod {
    model: Option<(u32, Model<LodTerrainPipeline>)>,
    locals: Consts<Locals>,
    pub map: Texture<LodColorFmt>,
    pub horizon: Texture<LodTextureFmt>,
    tgt_detail: u32,
}

impl Lod {
    pub fn new(renderer: &mut Renderer, client: &Client, settings: &Settings) -> Self {
        let water_color = /*Rgba::new(0.2, 0.5, 1.0, 0.0)*/srgba_to_linear(Rgba::new(0.0, 0.25, 0.5, 0.0)/* * 0.5*/);
        Self {
            model: None,
            locals: renderer.create_consts(&[Locals::default()]).unwrap(),
            map: renderer
                .create_texture(
                    &client.lod_base,
                    Some(FilterMethod::Bilinear),
                    Some(WrapMode::Border),
                    Some([water_color.r, water_color.g, water_color.b, water_color.a].into()),
                )
                .expect("Failed to generate map texture"),
            horizon: renderer
                .create_texture(
                    &client.lod_horizon,
                    Some(FilterMethod::Trilinear),
                    Some(WrapMode::Border),
                    Some([0.0, 1.0, 0.0, 1.0].into()),
                )
                .expect("Failed to generate map texture"),
            tgt_detail: settings.graphics.lod_detail.max(100).min(2500),
        }
    }

    pub fn set_detail(&mut self, detail: u32) { self.tgt_detail = detail.max(100).min(2500); }

    pub fn maintain(&mut self, renderer: &mut Renderer, _time_of_day: f64) {
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
            renderer.render_lod_terrain(&model, globals, &self.locals, &self.map, &self.horizon);
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
