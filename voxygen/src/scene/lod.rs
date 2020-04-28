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

pub struct LodData {
    pub map: Texture<LodColorFmt>,
    pub horizon: Texture<LodTextureFmt>,
    pub tgt_detail: u32,
}

pub struct Lod {
    model: Option<(u32, Model<LodTerrainPipeline>)>,
    locals: Consts<Locals>,
    data: LodData,
}

impl LodData {
    pub fn new(
        renderer: &mut Renderer,
        lod_base: &image::DynamicImage,
        lod_horizon: &image::DynamicImage,
        tgt_detail: u32,
        border_color: gfx::texture::PackedColor,
    ) -> Self {
        Self {
            map: renderer
                .create_texture(
                    lod_base,
                    Some(FilterMethod::Bilinear),
                    Some(WrapMode::Border),
                    Some(border_color),
                )
                .expect("Failed to generate map texture"),
            horizon: renderer
                .create_texture(
                    lod_horizon,
                    Some(FilterMethod::Trilinear),
                    Some(WrapMode::Border),
                    Some([0.0, 1.0, 0.0, 1.0].into()),
                )
                .expect("Failed to generate map texture"),
            tgt_detail,
        }
    }
}

impl Lod {
    pub fn new(renderer: &mut Renderer, client: &Client, settings: &Settings) -> Self {
        let water_color = /*Rgba::new(0.2, 0.5, 1.0, 0.0)*/srgba_to_linear(Rgba::new(0.0, 0.25, 0.5, 0.0)/* * 0.5*/);
        Self {
            model: None,
            locals: renderer.create_consts(&[Locals::default()]).unwrap(),
            data: LodData::new(
                renderer,
                &client.lod_base,
                &client.lod_horizon,
                settings.graphics.lod_detail.max(100).min(2500),
                [water_color.r, water_color.g, water_color.b, water_color.a].into(),
            ),
        }
    }

    pub fn get_data(&self) -> &LodData { &self.data }

    pub fn set_detail(&mut self, detail: u32) { self.data.tgt_detail = detail.max(100).min(2500); }

    pub fn maintain(&mut self, renderer: &mut Renderer, _time_of_day: f64) {
        if self
            .model
            .as_ref()
            .map(|(detail, _)| *detail != self.data.tgt_detail)
            .unwrap_or(true)
        {
            self.model = Some((
                self.data.tgt_detail,
                renderer
                    .create_model(&create_lod_terrain_mesh(self.data.tgt_detail))
                    .unwrap(),
            ));
        }
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        if let Some((_, model)) = self.model.as_ref() {
            renderer.render_lod_terrain(
                &model,
                globals,
                &self.locals,
                &self.data.map,
                &self.data.horizon,
            );
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
