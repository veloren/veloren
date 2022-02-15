use super::SceneData;
use crate::render::{DynamicModel, Mesh, Quad, Renderer, TrailDrawer, TrailVertex};
use common::comp::CharacterState;
use common_base::span;
use specs::{Entity as EcsEntity, Join, WorldExt};
use std::collections::HashMap;
// use vek::*;

pub struct TrailMgr {
    /// Meshes for each entity
    pub entity_meshes: HashMap<EcsEntity, Mesh<TrailVertex>>,

    /// Offset
    pub offset: usize,

    /// Dynamic model to upload to GPU
    dynamic_model: DynamicModel<TrailVertex>,
}

const TRAIL_DYNAMIC_MODEL_SIZE: usize = 15;

impl TrailMgr {
    pub fn new(renderer: &mut Renderer) -> Self {
        Self {
            entity_meshes: HashMap::new(),
            offset: 0,
            dynamic_model: renderer.create_dynamic_model(0),
        }
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, scene_data: &SceneData) {
        span!(_guard, "maintain", "TrailMgr::maintain");

        if scene_data.weapon_trails_enabled {
            // Update offset
            self.offset = (self.offset + 1) % TRAIL_DYNAMIC_MODEL_SIZE;

            // TODO: Automatically make mesh 0 at this offset for all entities

            // Create a mesh for each entity that doesn't already have one
            let ecs = scene_data.state.ecs();
            for (entity, _char_state) in
                (&ecs.entities(), &ecs.read_storage::<CharacterState>()).join()
            {
                // Result returned doesn't matter, it just needs to only insert if entry didn't
                // already exist
                if let Ok(mesh) = self.entity_meshes.try_insert(entity, Mesh::new()) {
                    // Allocate up to necessary length so repalce_quad works as expected elsewhere
                    let zero = TrailVertex::zero();
                    for _ in 0..TRAIL_DYNAMIC_MODEL_SIZE {
                        mesh.push_quad(Quad::new(zero, zero, zero, zero));
                    }
                }
            }

            // Clear meshes for entities that no longer exist (is this even
            // necessary? not sure if this growing too big is a concern)
            self.entity_meshes
                .retain(|entity, _| ecs.entities().is_alive(*entity));

            // Create dynamic model from currently existing meshes
            self.dynamic_model = {
                let mut big_mesh = Mesh::new();
                self.entity_meshes
                    .values()
                    .for_each(|mesh| big_mesh.push_mesh(mesh));
                let dynamic_model = renderer.create_dynamic_model(big_mesh.len());
                renderer.update_model(&dynamic_model, &big_mesh, 0);
                dynamic_model
            };
        } else {
            self.entity_meshes.clear();
        }
    }

    pub fn render<'a>(&'a self, drawer: &mut TrailDrawer<'_, 'a>, scene_data: &SceneData) {
        span!(_guard, "render", "TrailMgr::render");
        if scene_data.weapon_trails_enabled {
            drawer.draw(&self.dynamic_model);
        }
    }
}
