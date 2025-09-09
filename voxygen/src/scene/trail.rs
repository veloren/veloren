use super::{SceneData, figure::cache::ToolKey};
use crate::render::{DynamicModel, Mesh, Quad, Renderer, TrailDrawer, TrailVertex};
use common::{
    assets::{AssetExt, BoxedError, FileAsset, load_ron},
    comp::{
        Body, Item, Pos, Vel,
        item::ItemKind,
        object,
        tool::{Tool, ToolKind},
    },
};
use common_base::span;
use serde::Deserialize;
use specs::{Entity as EcsEntity, Join, WorldExt};
use std::{borrow::Cow, collections::HashMap};
use vek::*;

lazy_static::lazy_static! {
    pub static ref TOOL_TRAIL_MANIFEST: ToolTrailManifest = ToolTrailManifest::load_expect_cloned("voxygen.voxel.tool_trail_manifest");
}

#[derive(Clone, Debug, Deserialize)]
pub struct ToolTrailManifest(pub HashMap<ToolKey, ([f32; 3], [f32; 3])>);

impl FileAsset for ToolTrailManifest {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> { load_ron(&bytes) }
}

impl ToolTrailManifest {
    pub fn get(&self, item: &Item) -> Option<(Vec3<f32>, Vec3<f32>)> {
        let tool_key = ToolKey::from(item);
        self.0
            .get(&tool_key)
            .map(|trail_points| (Vec3::from(trail_points.0), Vec3::from(trail_points.1)))
            .or_else(|| {
                if let ItemKind::Tool(Tool { kind, .. }) = *item.kind() {
                    let z_offsets = match kind {
                        ToolKind::Sword => (0.0, 29.25),
                        ToolKind::Axe => (10.0, 19.25),
                        ToolKind::Hammer => (10.0, 19.25),
                        ToolKind::Staff => (10.0, 19.25),
                        ToolKind::Sceptre => (10.0, 19.25),
                        _ => (0.0, 0.0),
                    };
                    Some((
                        Vec3::new(0.0, 0.0, z_offsets.0),
                        Vec3::new(0.0, 0.0, z_offsets.1),
                    ))
                } else {
                    None
                }
            })
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
struct MeshKey {
    entity: EcsEntity,
    is_main_weapon: bool,
}

#[derive(Default)]
pub struct TrailMgr {
    /// Meshes for each entity, usize is the last offset tick it was updated
    entity_meshes: HashMap<MeshKey, (Mesh<TrailVertex>, usize)>,

    /// Position cache for things like projectiles
    pos_cache: HashMap<EcsEntity, Pos>,

    /// Offset
    offset: usize,

    /// Dynamic model to upload to GPU
    dynamic_model: Option<DynamicModel<TrailVertex>>,

    /// Used to create sub model from dynamic model
    model_len: u32,
}

const TRAIL_DYNAMIC_MODEL_SIZE: usize = 15;
const TRAIL_SHRINKAGE: f32 = 0.8;

impl TrailMgr {
    pub fn maintain(&mut self, renderer: &mut Renderer, scene_data: &SceneData) {
        span!(_guard, "maintain", "TrailMgr::maintain");

        if scene_data.weapon_trails_enabled {
            // Hack to shove trails in for projectiles
            let ecs = scene_data.state.ecs();
            for (entity, body, vel, pos) in (
                &ecs.entities(),
                &ecs.read_storage::<Body>(),
                &ecs.read_storage::<Vel>(),
                &ecs.read_storage::<Pos>(),
            )
                .join()
            {
                const MIN_SPEED: f32 = 15.0;
                if vel.0.magnitude_squared() > MIN_SPEED.powi(2)
                    && matches!(
                        body,
                        Body::Object(
                            object::Body::Arrow
                                | object::Body::MultiArrow
                                | object::Body::ArrowSnake
                                | object::Body::ArrowTurret
                                | object::Body::ArrowClay
                                | object::Body::BoltBesieger,
                        )
                    )
                {
                    let last_pos = *self.pos_cache.entry(entity).or_insert(*pos);
                    let offset = self.offset;
                    let quad_mesh = self.entity_mesh_or_insert(entity, true);
                    const THICKNESS: f32 = 0.2;
                    let p1 = pos.0;
                    let p2 = p1 + Vec3::unit_z() * THICKNESS;
                    let p4 = last_pos.0;
                    let p3 = p4 + Vec3::unit_z() * THICKNESS;
                    let vertex = |p: Vec3<f32>| TrailVertex {
                        pos: p.into_array(),
                    };
                    let quad = Quad::new(vertex(p1), vertex(p2), vertex(p3), vertex(p4));
                    quad_mesh.replace_quad(offset * 4, quad);
                    self.pos_cache.insert(entity, *pos);
                }
            }

            // Update offset
            self.offset = (self.offset + 1) % TRAIL_DYNAMIC_MODEL_SIZE;

            self.entity_meshes.values_mut().for_each(|(mesh, _)| {
                // TODO: Figure out how to do this in shader files instead
                // Shrink size of each quad over time
                let vertices = mesh.vertices_mut_vec();
                let last_offset =
                    (self.offset + TRAIL_DYNAMIC_MODEL_SIZE - 1) % TRAIL_DYNAMIC_MODEL_SIZE;
                let next_offset = (self.offset + 1) % TRAIL_DYNAMIC_MODEL_SIZE;
                for i in 0..TRAIL_DYNAMIC_MODEL_SIZE {
                    // Verts per quad are in b, c, a, d order
                    let [b, c, a, d] = [0, 1, 2, 3].map(|offset| i * 4 + offset);
                    vertices[a] = if i == next_offset {
                        vertices[b]
                    } else {
                        vertices[a] * TRAIL_SHRINKAGE + vertices[b] * (1.0 - TRAIL_SHRINKAGE)
                    };
                    if i != last_offset {
                        // Avoid shrinking edge of most recent quad so that edges of quads align
                        vertices[d] =
                            vertices[d] * TRAIL_SHRINKAGE + vertices[c] * (1.0 - TRAIL_SHRINKAGE);
                    }
                }

                // Reset quad for each entity mesh at new offset
                let zero = TrailVertex::zero();
                mesh.replace_quad(self.offset * 4, Quad::new(zero, zero, zero, zero));
            });

            // Clear meshes for entities that only have zero quads in mesh
            self.entity_meshes
                .retain(|_, (_mesh, last_updated)| *last_updated != self.offset);

            // TODO: as an optimization we can keep this big mesh around between frames and
            // write directly to it for each entity.
            // Create big mesh from currently existing meshes that is used to update dynamic
            // model
            let mut big_mesh = Mesh::new();
            self.entity_meshes
                .values()
                // If any of the vertices in a mesh are non-zero, upload the entire mesh for the entity
                .filter(|(mesh, _)| mesh.iter().any(|vert| *vert != TrailVertex::zero()))
                .for_each(|(mesh, _)| big_mesh.push_mesh(mesh));

            // To avoid empty mesh
            if big_mesh.is_empty() {
                let zero = TrailVertex::zero();
                big_mesh.push_quad(Quad::new(zero, zero, zero, zero));
            }

            // If dynamic model too small, resize, with room for 10 additional entities to
            // avoid needing to resize frequently
            if self.dynamic_model.as_ref().map_or(0, |model| model.len()) < big_mesh.len() {
                self.dynamic_model = Some(
                    renderer
                        .create_dynamic_model(big_mesh.len() + TRAIL_DYNAMIC_MODEL_SIZE * 4 * 10),
                );
            }
            if let Some(dynamic_model) = &self.dynamic_model {
                renderer.update_model(dynamic_model, &big_mesh, 0);
            }
            self.model_len = big_mesh.len() as u32;
        } else {
            self.entity_meshes.clear();
            // Clear dynamic model to free memory
            self.dynamic_model = None;
        }
    }

    pub fn render<'a>(&'a self, drawer: &mut TrailDrawer<'_, 'a>, scene_data: &SceneData) {
        span!(_guard, "render", "TrailMgr::render");
        if scene_data.weapon_trails_enabled
            && let Some(dynamic_model) = &self.dynamic_model
        {
            drawer.draw(dynamic_model.submodel(0..self.model_len))
        }
    }

    pub fn entity_mesh_or_insert(
        &mut self,
        entity: EcsEntity,
        is_main_weapon: bool,
    ) -> &mut Mesh<TrailVertex> {
        let key = MeshKey {
            entity,
            is_main_weapon,
        };
        &mut self
            .entity_meshes
            .entry(key)
            .and_modify(|(_mesh, offset)| *offset = self.offset)
            .or_insert((Self::default_trail_mesh(), self.offset))
            .0
    }

    fn default_trail_mesh() -> Mesh<TrailVertex> {
        let mut mesh = Mesh::new();
        let zero = TrailVertex::zero();
        for _ in 0..TRAIL_DYNAMIC_MODEL_SIZE {
            mesh.push_quad(Quad::new(zero, zero, zero, zero));
        }
        mesh
    }

    pub fn offset(&self) -> usize { self.offset }
}
