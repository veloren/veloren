use super::{SceneData, Terrain};
use crate::{
    render::{
        Instances, TrailDrawer,
        TrailInstance, Renderer,
    },
};
use common::{
    terrain::TerrainChunk,
};
use common_base::span;
use std::time::Duration;
use vek::*;

pub struct TrailMgr {
    /// keep track of lifespans
    trails: Vec<Trail>,

    /// GPU Instance Buffer
    instances: Instances<TrailInstance>,
}

impl TrailMgr {
    pub fn new(renderer: &mut Renderer) -> Self {
        Self {
            trails: Vec::new(),
            instances: default_instances(renderer),
        }
    }

    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        terrain: &Terrain<TerrainChunk>,
    ) {
        span!(_guard, "maintain", "TrailMgr::maintain");
        if scene_data.trails_enabled {
            // remove dead Trails
            self.trails
                .retain(|p| p.alive_until > scene_data.state.get_time());

            // add new Trails

            self.upload_trails(renderer);
        } else {
            // remove all trail lifespans
            if !self.trails.is_empty() {
                self.trails.clear();
                self.upload_trails(renderer);
            }
        }
    }

    fn upload_trails(&mut self, renderer: &mut Renderer) {
        span!(_guard, "upload_trails", "TrailMgr::upload_trails");
        let all_cpu_instances = self
            .trails
            .iter()
            .map(|p| p.instance)
            .collect::<Vec<TrailInstance>>();

        // TODO: optimise buffer writes
        let gpu_instances = renderer
            .create_instances(&all_cpu_instances)
            .expect("Failed to upload trail instances to the GPU!");

        self.instances = gpu_instances;
    }

    pub fn render<'a>(&'a self, drawer: &mut TrailDrawer<'_, 'a>, scene_data: &SceneData) {
        span!(_guard, "render", "TrailMgr::render");
        if scene_data.trails_enabled {
            drawer.draw(&self.instances);
        }
    }

    pub fn trail_count(&self) -> usize { self.instances.count() }

    pub fn trail_count_visible(&self) -> usize { self.instances.count() }
}

fn default_instances(renderer: &mut Renderer) -> Instances<TrailInstance> {
    let empty_vec = Vec::new();

    renderer
        .create_instances(&empty_vec)
        .expect("Failed to upload trail instances to the GPU!")
}

#[derive(Clone, Copy)]
struct Trail {
    alive_until: f64, // created_at + lifespan
    instance: TrailInstance,
}

impl Trail {
    fn new(lifespan: Duration, time: f64, inner_pos: Vec3<f32>, outer_pos: Vec3<f32>)-> Self {
        Trail {
            alive_until: time + lifespan.as_secs_f64(),
            instance: TrailInstance::new(time, lifespan.as_secs_f32(), inner_pos, outer_pos),
        }
    }
}
