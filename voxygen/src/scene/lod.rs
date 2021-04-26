use crate::{
    render::{
        pipelines::lod_terrain::{LodData, Vertex},
        FirstPassDrawer, LodTerrainVertex, Mesh, Model, Quad, Renderer,
    },
    settings::Settings,
};
use client::Client;
use common::{spiral::Spiral2d, util::srgba_to_linear};
use vek::*;

pub struct Lod {
    model: Option<(u32, Model<LodTerrainVertex>)>,
    data: LodData,
}

// TODO: Make constant when possible.
pub fn water_color() -> Rgba<f32> {
    /* Rgba::new(0.2, 0.5, 1.0, 0.0) */
    srgba_to_linear(Rgba::new(0.0, 0.25, 0.5, 0.0))
}

impl Lod {
    pub fn new(renderer: &mut Renderer, client: &Client, settings: &Settings) -> Self {
        Self {
            model: None,
            data: LodData::new(
                renderer,
                client.world_data().chunk_size().as_(),
                client.world_data().lod_base.raw(),
                client.world_data().lod_alt.raw(),
                client.world_data().lod_horizon.raw(),
                settings.graphics.lod_detail.max(100).min(2500),
                // water_color().into_array().into(),
            ),
        }
    }

    pub fn get_data(&self) -> &LodData { &self.data }

    pub fn set_detail(&mut self, detail: u32) {
        // Make sure the recorded detail is even.
        self.data.tgt_detail = (detail - detail % 2).max(100).min(2500);
    }

    pub fn maintain(&mut self, renderer: &mut Renderer) {
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

    pub fn render<'a>(&'a self, drawer: &mut FirstPassDrawer<'a>) {
        if let Some((_, model)) = self.model.as_ref() {
            drawer.draw_lod_terrain(&model);
        }
    }
}

fn create_lod_terrain_mesh(detail: u32) -> Mesh<LodTerrainVertex> {
    // detail is even, so we choose odd detail (detail + 1) to create two even
    // halves with an empty hole.
    let detail = detail + 1;
    Spiral2d::new()
        .take((detail * detail) as usize)
        .skip(1)
        .map(|pos| {
            let x = pos.x + detail as i32 / 2;
            let y = pos.y + detail as i32 / 2;

            let transform = |x| (2.0 * x as f32) / detail as f32 - 1.0;

            Quad::new(
                Vertex::new(Vec2::new(x, y).map(transform)),
                Vertex::new(Vec2::new(x + 1, y).map(transform)),
                Vertex::new(Vec2::new(x + 1, y + 1).map(transform)),
                Vertex::new(Vec2::new(x, y + 1).map(transform)),
            )
            .rotated_by(if (x > detail as i32 / 2) ^ (y > detail as i32 / 2) {
                0
            } else {
                1
            })
        })
        .collect()
}
