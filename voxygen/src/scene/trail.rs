use super::SceneData;
use crate::render::{DynamicModel, Mesh, Quad, Renderer, TrailDrawer, TrailVertex};
use common_base::span;
use specs::Entity as EcsEntity;
use std::collections::HashMap;
// use vek::*;

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
struct MeshKey {
    entity: EcsEntity,
    is_main_weapon: bool,
}

pub struct TrailMgr {
    /// Meshes for each entity
    entity_meshes: HashMap<MeshKey, Mesh<TrailVertex>>,

    /// Offset
    pub offset: usize,

    /// Dynamic model to upload to GPU
    dynamic_model: DynamicModel<TrailVertex>,

    /// Used to create sub model from dynamic model
    model_len: u32,
}

const TRAIL_DYNAMIC_MODEL_SIZE: usize = 15;
const TRAIL_SHRINKAGE: f32 = 0.8;

impl TrailMgr {
    pub fn new(renderer: &mut Renderer) -> Self {
        Self {
            entity_meshes: HashMap::new(),
            offset: 0,
            dynamic_model: renderer.create_dynamic_model(0),
            model_len: 0,
        }
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, scene_data: &SceneData) {
        span!(_guard, "maintain", "TrailMgr::maintain");

        if scene_data.weapon_trails_enabled {
            // Update offset
            self.offset = (self.offset + 1) % TRAIL_DYNAMIC_MODEL_SIZE;

            self.entity_meshes.values_mut().for_each(|mesh| {
                // Shrink size of each quad over time
                let vertices = mesh.vertices_mut_vec();
                for i in 0..TRAIL_DYNAMIC_MODEL_SIZE {
                    // Verts per quad are in b, c, a, d order
                    vertices[i * 4 + 2] = vertices[i * 4 + 2] * TRAIL_SHRINKAGE
                        + vertices[i * 4] * (1.0 - TRAIL_SHRINKAGE);
                    if i != (self.offset + TRAIL_DYNAMIC_MODEL_SIZE - 1) % TRAIL_DYNAMIC_MODEL_SIZE
                    {
                        // Avoid shrinking edge of most recent quad so that edges of quads align
                        vertices[i * 4 + 3] = vertices[i * 4 + 3] * TRAIL_SHRINKAGE
                            + vertices[i * 4 + 1] * (1.0 - TRAIL_SHRINKAGE);
                    }
                }

                // Reset quad for each entity mesh at new offset
                let zero = TrailVertex::zero();
                mesh.replace_quad(self.offset * 4, Quad::new(zero, zero, zero, zero));
            });

            // Clear meshes for entities that only have zero quads in mesh
            self.entity_meshes
                .retain(|_, mesh| mesh.iter().any(|vert| *vert != TrailVertex::zero()));

            // Create dynamic model from currently existing meshes
            let mut big_mesh = Mesh::new();
            self.entity_meshes
                .values()
                // If any of the vertices in a mesh are non-zero, upload the entire mesh for the entity
                .filter(|mesh| mesh.iter().any(|vert| *vert != TrailVertex::zero()))
                .for_each(|mesh| big_mesh.push_mesh(mesh));

            // To avoid empty mesh
            if big_mesh.is_empty() {
                let zero = TrailVertex::zero();
                big_mesh.push_quad(Quad::new(zero, zero, zero, zero));
            }

            // If dynamic model too small, resize
            if self.dynamic_model.len() < big_mesh.len() {
                self.dynamic_model = renderer.create_dynamic_model(big_mesh.len());
            };
            renderer.update_model(&self.dynamic_model, &big_mesh, 0);
            self.model_len = big_mesh.len() as u32;
        } else {
            self.entity_meshes.clear();
        }
    }

    pub fn render<'a>(&'a self, drawer: &mut TrailDrawer<'_, 'a>, scene_data: &SceneData) {
        span!(_guard, "render", "TrailMgr::render");
        if scene_data.weapon_trails_enabled {
            drawer.draw(&self.dynamic_model, self.model_len);
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
        self.entity_meshes
            .entry(key)
            .or_insert_with(Self::default_trail_mesh)
    }

    fn default_trail_mesh() -> Mesh<TrailVertex> {
        let mut mesh = Mesh::new();
        let zero = TrailVertex::zero();
        for _ in 0..TRAIL_DYNAMIC_MODEL_SIZE {
            mesh.push_quad(Quad::new(zero, zero, zero, zero));
        }
        mesh
    }
}
