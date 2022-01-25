use super::SceneData;
use crate::render::{DynamicModel, Renderer, TrailDrawer, TrailVertex};
use common::uid::Uid;
use common_base::span;
use std::collections::HashMap;

pub struct TrailMgr {
    /// GPU vertex buffers
    dynamic_models: HashMap<Uid, DynamicModel<TrailVertex>>,
}

impl TrailMgr {
    pub fn new(_renderer: &mut Renderer) -> Self {
        Self {
            dynamic_models: HashMap::new(),
        }
    }

    pub fn maintain(&mut self, _renderer: &mut Renderer, _scene_data: &SceneData) {
        span!(_guard, "maintain", "TrailMgr::maintain");
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
