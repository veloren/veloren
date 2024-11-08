use crate::{
    render::{
        pipelines::lod_terrain::{LodData, Vertex},
        CullingMode, FirstPassDrawer, Instances, LodObjectInstance, LodObjectVertex,
        LodTerrainVertex, Mesh, Model, Quad, Renderer, Tri,
    },
    scene::{camera, Camera},
    settings::Settings,
};
use client::Client;
use common::{
    assets::{AssetExt, ObjAsset},
    lod,
    spiral::Spiral2d,
    util::{srgb_to_linear, srgba_to_linear},
    weather,
};
use hashbrown::HashMap;
use std::ops::Range;
use treeculler::{BVol, Frustum, AABB};
use vek::*;

// For culling
const MAX_OBJECT_RADIUS: i32 = 64;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct VertexFlags: u8 {
        /// Use instance not vertex colour
        const INST_COLOR    = 0b00000001;
        /// Glow!
        const GLOW          = 0b00000010;
    }
}

struct ObjectGroup {
    instances: Instances<LodObjectInstance>,
    // None implies no instances
    z_range: Option<Range<i32>>,
    frustum_last_plane_index: u8,
    visible: bool,
}

pub struct Lod {
    model: Option<(u32, Model<LodTerrainVertex>)>,
    data: LodData,

    zone_objects: HashMap<Vec2<i32>, HashMap<lod::ObjectKind, ObjectGroup>>,
    object_data: HashMap<lod::ObjectKind, Model<LodObjectVertex>>,
}

// TODO: Make constant when possible.
pub fn water_color() -> Rgba<f32> {
    /* Rgba::new(0.2, 0.5, 1.0, 0.0) */
    srgba_to_linear(Rgba::new(0.0, 0.25, 0.5, 0.0))
}

impl Lod {
    pub fn new(renderer: &mut Renderer, client: &Client, settings: &Settings) -> Self {
        let data = LodData::new(
            renderer,
            client.world_data().chunk_size().as_(),
            client.world_data().lod_base.raw(),
            client.world_data().lod_alt.raw(),
            client.world_data().lod_horizon.raw(),
            (client.world_data().chunk_size().as_() / weather::CHUNKS_PER_CELL).map(|e| e.max(1)),
            settings.graphics.lod_detail.clamp(100, 2500),
            /* TODO: figure out how we want to do this without color borders?
             * water_color().into_array().into(), */
        );
        Self {
            model: None,
            data,
            zone_objects: HashMap::new(),
            object_data: [
                (
                    lod::ObjectKind::GenericTree,
                    make_lod_object("oak", renderer),
                ),
                (lod::ObjectKind::Pine, make_lod_object("pine", renderer)),
                (lod::ObjectKind::Dead, make_lod_object("dead", renderer)),
                (lod::ObjectKind::House, make_lod_object("house", renderer)),
                (
                    lod::ObjectKind::GiantTree,
                    make_lod_object("giant_tree", renderer),
                ),
                (
                    lod::ObjectKind::Mangrove,
                    make_lod_object("mangrove", renderer),
                ),
                (lod::ObjectKind::Acacia, make_lod_object("acacia", renderer)),
                (lod::ObjectKind::Birch, make_lod_object("birch", renderer)),
                (
                    lod::ObjectKind::Redwood,
                    make_lod_object("redwood", renderer),
                ),
                (lod::ObjectKind::Baobab, make_lod_object("baobab", renderer)),
                (
                    lod::ObjectKind::Frostpine,
                    make_lod_object("frostpine", renderer),
                ),
                (lod::ObjectKind::Haniwa, make_lod_object("haniwa", renderer)),
                (
                    lod::ObjectKind::Desert,
                    make_lod_object("desert_houses", renderer),
                ),
                (lod::ObjectKind::Palm, make_lod_object("palm", renderer)),
                (lod::ObjectKind::Arena, make_lod_object("arena", renderer)),
                (
                    lod::ObjectKind::SavannahHut,
                    make_lod_object("savannah_hut", renderer),
                ),
                (
                    lod::ObjectKind::SavannahAirshipDock,
                    make_lod_object("savannah_airship_dock", renderer),
                ),
                (
                    lod::ObjectKind::TerracottaPalace,
                    make_lod_object("terracotta_palace", renderer),
                ),
                (
                    lod::ObjectKind::TerracottaHouse,
                    make_lod_object("terracotta_house", renderer),
                ),
                (
                    lod::ObjectKind::TerracottaYard,
                    make_lod_object("terracotta_yard", renderer),
                ),
                (
                    lod::ObjectKind::AirshipDock,
                    make_lod_object("airship_dock", renderer),
                ),
                (
                    lod::ObjectKind::CoastalHouse,
                    make_lod_object("coastal_house", renderer),
                ),
                (
                    lod::ObjectKind::CoastalWorkshop,
                    make_lod_object("coastal_workshop", renderer),
                ),
            ]
            .into(),
        }
    }

    pub fn get_data(&self) -> &LodData { &self.data }

    pub fn set_detail(&mut self, detail: u32) {
        // Make sure the recorded detail is even.
        self.data.tgt_detail = (detail - detail % 2).clamp(100, 2500);
    }

    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        client: &Client,
        focus_pos: Vec3<f32>,
        camera: &Camera,
    ) {
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

        // Create new LoD groups when a new zone has loaded
        for (p, zone) in client.lod_zones() {
            self.zone_objects.entry(*p).or_insert_with(|| {
                let mut objects = HashMap::<_, Vec<_>>::new();
                let mut z_range = None;
                for object in zone.objects.iter() {
                    let pos = p.map(|e| lod::to_wpos(e) as f32).with_z(0.0)
                        + object.pos.map(|e| e as f32)
                        + Vec2::broadcast(0.5).with_z(0.0);
                    z_range = Some(z_range.map_or(
                        pos.z as i32..pos.z as i32,
                        |z_range: Range<i32>| {
                            z_range.start.min(pos.z as i32)..z_range.end.max(pos.z as i32)
                        },
                    ));
                    objects
                        .entry(object.kind)
                        .or_default()
                        .push(LodObjectInstance::new(pos, object.color, object.flags));
                }
                objects
                    .into_iter()
                    .map(|(kind, instances)| {
                        (kind, ObjectGroup {
                            instances: renderer.create_instances(&instances),
                            z_range: z_range.clone(),
                            frustum_last_plane_index: 0,
                            visible: false,
                        })
                    })
                    .collect()
            });
        }

        // Remove zones that are unloaded
        self.zone_objects
            .retain(|p, _| client.lod_zones().contains_key(p));

        // Determine visibility of zones based on view frustum
        let camera::Dependents {
            view_mat,
            proj_mat_treeculler,
            ..
        } = camera.dependents();
        let focus_off = focus_pos.map(|e| e.trunc());
        let frustum = Frustum::from_modelview_projection(
            (proj_mat_treeculler * view_mat * Mat4::translation_3d(-focus_off)).into_col_arrays(),
        );
        for (pos, groups) in &mut self.zone_objects {
            for group in groups.values_mut() {
                if let Some(z_range) = &group.z_range {
                    let group_min = (pos.map(lod::to_wpos).with_z(z_range.start)
                        - MAX_OBJECT_RADIUS)
                        .map(|e| e as f32);
                    let group_max = ((pos + 1).map(lod::to_wpos).with_z(z_range.end)
                        + MAX_OBJECT_RADIUS)
                        .map(|e| e as f32);
                    let (in_frustum, last_plane_index) =
                        AABB::new(group_min.into_array(), group_max.into_array())
                            .coherent_test_against_frustum(
                                &frustum,
                                group.frustum_last_plane_index,
                            );
                    group.visible = in_frustum;
                    group.frustum_last_plane_index = last_plane_index;
                }
            }
        }
        // Update weather texture
        // NOTE: consider moving the lerping to a shader if the overhead of uploading to
        // the gpu each frame becomes an issue.
        let weather = client.state().weather_grid();
        let size = weather.size().as_::<u32>();
        renderer.update_texture(
            &self.data.weather,
            [0, 0],
            [size.x, size.y],
            &weather
                .iter()
                .map(|(_, w)| [(w.cloud * 255.0) as u8, (w.rain * 255.0) as u8, 0, 0])
                .collect::<Vec<_>>(),
        );
    }

    pub fn render<'a>(&'a self, drawer: &mut FirstPassDrawer<'a>, culling_mode: CullingMode) {
        if let Some((_, model)) = self.model.as_ref() {
            drawer.draw_lod_terrain(model);
        }

        if !matches!(culling_mode, CullingMode::Underground) {
            // Draw LoD objects
            let mut drawer = drawer.draw_lod_objects();
            for groups in self.zone_objects.values() {
                for (kind, group) in groups.iter().filter(|(_, g)| g.visible) {
                    if let Some(model) = self.object_data.get(kind) {
                        drawer.draw(model, &group.instances);
                    }
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
            .rotated_by(usize::from(
                !((x > detail as i32 / 2) ^ (y > detail as i32 / 2)),
            ))
        })
        .collect()
}

fn make_lod_object(name: &str, renderer: &mut Renderer) -> Model<LodObjectVertex> {
    let model = ObjAsset::load_expect(&format!("voxygen.lod.{}", name));
    let mesh = model
        .read()
        .0
        .objects()
        .flat_map(|(objname, obj)| {
            let mut color = objname.split('_').filter_map(|x| x.parse::<u8>().ok());
            let color = color
                .next()
                .and_then(|r| Some(Rgb::new(r, color.next()?, color.next()?)))
                .unwrap_or(Rgb::broadcast(127));
            let color = srgb_to_linear(color.map(|c| (c as f32 / 255.0)));
            let flags = match objname {
                "InstCol" => VertexFlags::INST_COLOR,
                "Glow" => VertexFlags::GLOW,
                _ => VertexFlags::empty(),
            };
            obj.triangles().map(move |vs| {
                let [a, b, c] = vs.map(|v| {
                    LodObjectVertex::new(
                        v.position().into(),
                        v.normal().unwrap_or([0.0, 0.0, 1.0]).into(),
                        color,
                        flags,
                    )
                });
                Tri::new(a, b, c)
            })
        })
        .collect();
    renderer.create_model(&mesh).expect("Mesh was empty!")
}
