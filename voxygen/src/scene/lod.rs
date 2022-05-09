use crate::{
    render::{
        pipelines::lod_terrain::{LodData, Vertex},
        FirstPassDrawer, LodTerrainVertex, LodObjectVertex, Mesh, Model, Quad, Renderer, Instances, LodObjectInstance, Tri,
    },
    scene::GlobalModel,
    settings::Settings,
};
use hashbrown::HashMap;
use client::Client;
use common::{
    assets::{ObjAsset, AssetExt},
    spiral::Spiral2d,
    util::srgba_to_linear,
    lod,
};
use vek::*;

pub struct Lod {
    model: Option<(u32, Model<LodTerrainVertex>)>,
    data: LodData,

    zone_objects: HashMap<Vec2<i32>, HashMap<lod::ObjectKind, Instances<LodObjectInstance>>>,
    object_data: HashMap<lod::ObjectKind, Model<LodObjectVertex>>,
}

// TODO: Make constant when possible.
pub fn water_color() -> Rgba<f32> {
    /* Rgba::new(0.2, 0.5, 1.0, 0.0) */
    srgba_to_linear(Rgba::new(0.0, 0.25, 0.5, 0.0))
}

impl Lod {
    pub fn new(
        renderer: &mut Renderer,
        client: &Client,
        settings: &Settings,
    ) -> Self {
        let data = LodData::new(
            renderer,
            client.world_data().chunk_size().as_(),
            client.world_data().lod_base.raw(),
            client.world_data().lod_alt.raw(),
            client.world_data().lod_horizon.raw(),
            settings.graphics.lod_detail.max(100).min(2500),
            /* TODO: figure out how we want to do this without color borders?
              * water_color().into_array().into(), */
        );
        Self {
            zone_objects: HashMap::new(),
            object_data: [
                (lod::ObjectKind::Oak, make_lod_object("oak", renderer, &data)),
                (lod::ObjectKind::Pine, make_lod_object("pine", renderer, &data)),
            ]
                .into_iter()
                .collect(),
            model: None,
            data,
        }
    }

    pub fn get_data(&self) -> &LodData { &self.data }

    pub fn set_detail(&mut self, detail: u32) {
        // Make sure the recorded detail is even.
        self.data.tgt_detail = (detail - detail % 2).max(100).min(2500);
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, client: &Client) {
        // Update LoD terrain mesh according to detail
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

        // Maintain LoD object instances
        for (p, zone) in client.lod_zones() {
            self.zone_objects.entry(*p).or_insert_with(|| {
                let mut objects = HashMap::<_, Vec<_>>::new();
                for object in zone.objects.iter() {
                    let pos = p.map(|e| lod::to_wpos(e) as f32).with_z(0.0)
                        + object.pos.map(|e| e as f32)
                        + Vec2::broadcast(0.5).with_z(0.0);
                    objects
                        .entry(object.kind)
                        .or_default()
                        .push(LodObjectInstance::new(pos, object.flags));
                }
                objects
                    .into_iter()
                    .map(|(kind, instances)| {
                        (kind, renderer.create_instances(&instances).expect("Renderer error?!"))
                    })
                    .collect()
            });
        }

        self.zone_objects.retain(|p, _| client.lod_zones().contains_key(p));
    }

    pub fn render<'a>(&'a self, drawer: &mut FirstPassDrawer<'a>) {
        if let Some((_, model)) = self.model.as_ref() {
            drawer.draw_lod_terrain(model);
        }

        // Draw LoD objects
        let mut drawer = drawer.draw_lod_objects();
        for objects in self.zone_objects.values() {
            for (kind, instances) in objects {
                if let Some(model) = self.object_data.get(kind) {
                    drawer.draw(model, instances);
                }
            }
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

fn make_lod_object(
    name: &str,
    renderer: &mut Renderer,
    lod_data: &LodData,
) -> Model<LodObjectVertex> {
    let model = ObjAsset::load_expect(&format!("voxygen.lod.{}", name));
    let mesh = model
        .read().0
        .triangles()
        .map(|vs| {
            let [a, b, c] = vs.map(|v| LodObjectVertex::new(
                v.position().into(),
                v.normal().unwrap_or([0.0, 0.0, 1.0]).into(),
                Vec3::broadcast(1.0),
            ));
            Tri::new(a, b, c)
        })
        .collect();
    renderer
        .create_model(&mesh)
        .expect("Mesh was empty!")
}
