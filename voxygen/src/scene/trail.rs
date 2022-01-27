use super::SceneData;
use crate::render::{
    pipelines::trail, DynamicModel, Mesh, Quad, Renderer, TrailDrawer, TrailVertex,
};
use common::comp::CharacterState;
use common_base::span;
use specs::{Entity as EcsEntity, Join, WorldExt};
use std::collections::HashMap;
// use vek::*;

pub struct TrailMgr {
    /// Keep track of lifetimes
    // trails: Vec<Trail>,

    /// GPU vertex buffers
    pub dynamic_models: HashMap<EcsEntity, DynamicModel<TrailVertex>>,

    /// Offset
    pub offset: usize,
}

const TRAIL_DYNAMIC_MODEL_SIZE: usize = 30;

impl TrailMgr {
    pub fn new(_renderer: &mut Renderer) -> Self {
        Self {
            // trails: Vec::new(),
            dynamic_models: HashMap::new(),
            offset: 0,
        }
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, scene_data: &SceneData) {
        span!(_guard, "maintain", "TrailMgr::maintain");

        if scene_data.particles_enabled {
            // remove dead Trails
            // self.trails
            //     .retain(|t| t.alive_until > scene_data.state.get_time());

            // Update offset
            self.offset = (self.offset + 1) % TRAIL_DYNAMIC_MODEL_SIZE;

            // Update dynamic models
            let ecs = scene_data.state.ecs();
            for (entity, _char_state) in
                (&ecs.entities(), &ecs.read_storage::<CharacterState>()).join()
            {
                if let Ok(model) = self.dynamic_models.try_insert(
                    entity,
                    renderer.create_dynamic_model(TRAIL_DYNAMIC_MODEL_SIZE * 4),
                ) {
                    let mut mesh = Mesh::new();
                    let zero = trail::Vertex { pos: [0.0; 3] };
                    for _ in 0..TRAIL_DYNAMIC_MODEL_SIZE {
                        mesh.push_quad(Quad::new(zero, zero, zero, zero));
                    }
                    renderer.update_model(model, &mesh, 0);
                }
            }

            // Clear dynamic models for entities that no longer exist (is this even
            // necessary? not sure if this growing too big is a concern)
            self.dynamic_models
                .retain(|entity, _| ecs.entities().is_alive(*entity))
        } else {
            // if !self.trails.is_empty() {
            //     self.trails.clear();
            // }
        }
    }

    pub fn render<'a>(&'a self, drawer: &mut TrailDrawer<'_, 'a>, scene_data: &SceneData) {
        span!(_guard, "render", "TrailMgr::render");
        if scene_data.trails_enabled {
            for dynamic_model in self.dynamic_models.values() {
                drawer.draw(dynamic_model);
            }
        }
    }
}

// struct Trail {
//     entity: EcsEntity,
//     pos1: Vec3<f32>,
//     pos2: Vec3<f32>,
//     alive_until: f64,
// }

// impl Trail {
//     pub fn new(entity: EcsEntity, pos1: Vec3<f32>, pos2: Vec3<f32>, time:
// f64) -> Self {         const LIFETIME: f64 = 1.0;
//         Self {
//             entity,
//             pos1,
//             pos2,
//             alive_until: time + LIFETIME,
//         }
//     }
// }
