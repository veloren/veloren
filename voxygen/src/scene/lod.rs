use crate::{
    render::{
        pipelines::lod_terrain::{Locals, Vertex},
        Consts, Globals, LodAltFmt, LodColorFmt, LodTerrainPipeline, LodTextureFmt, Mesh, Model,
        Quad, Renderer, Texture,
    },
    settings::Settings,
};
use client::Client;
use common::{spiral::Spiral2d, util::srgba_to_linear};
use gfx::texture::SamplerInfo;
use vek::*;

pub struct LodData {
    pub map: Texture<LodColorFmt>,
    pub alt: Texture<LodAltFmt>,
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
        map_size: Vec2<u16>,
        lod_base: &[u32],
        lod_alt: &[u32],
        lod_horizon: &[u32],
        tgt_detail: u32,
        border_color: gfx::texture::PackedColor,
    ) -> Self {
        let kind = gfx::texture::Kind::D2(map_size.x, map_size.y, gfx::texture::AaMode::Single);
        let info = gfx::texture::SamplerInfo::new(
            gfx::texture::FilterMethod::Bilinear,
            gfx::texture::WrapMode::Border,
        );
        Self {
            map: renderer
                .create_texture_immutable_raw(
                    kind,
                    gfx::texture::Mipmap::Provided,
                    &[gfx::memory::cast_slice(lod_base)],
                    SamplerInfo {
                        border: border_color,
                        ..info
                    },
                )
                .expect("Failed to generate map texture"),
            alt: renderer
                .create_texture_immutable_raw(
                    kind,
                    gfx::texture::Mipmap::Provided,
                    &[gfx::memory::cast_slice(lod_alt)],
                    SamplerInfo {
                        border: [0.0, 0.0, 0.0, 0.0].into(),
                        ..info
                    },
                )
                .expect("Failed to generate alt texture"),
            horizon: renderer
                .create_texture_immutable_raw(
                    kind,
                    gfx::texture::Mipmap::Provided,
                    &[gfx::memory::cast_slice(lod_horizon)],
                    SamplerInfo {
                        // filter: gfx::texture::FilterMethod::Nearest,
                        // filter: gfx::texture::FilterMethod::TriLinear,
                        border: [1.0, 0.0, 1.0, 0.0].into(),
                        ..info
                    },
                )
                .expect("Failed to generate horizon texture"),
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
                client.world_map.1,
                &client.lod_base,
                &client.lod_alt,
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
                &self.data.alt,
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
