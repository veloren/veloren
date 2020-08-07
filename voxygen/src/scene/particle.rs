use super::SceneData;
use crate::{
    mesh::Meshable,
    render::{
        pipelines::particle::ParticleMode, Consts, Globals, Instances, Light, Model,
        ParticleInstance, ParticlePipeline, Renderer, Shadow,
    },
};
use common::{
    assets,
    comp::{object, Body, CharacterState, Pos},
    figure::Segment,
    outcome::Outcome,
};
use dot_vox::DotVoxData;
use hashbrown::HashMap;
use rand::Rng;
use specs::{Join, WorldExt};
use std::time::{Duration, Instant};
use vek::*;

pub struct ParticleMgr {
    /// keep track of lifespans
    particles: Vec<Particle>,

    /// keep track of timings
    scheduler: HeartbeatScheduler,

    /// GPU Instance Buffer
    instances: Instances<ParticleInstance>,

    /// GPU Vertex Buffers
    model_cache: HashMap<&'static str, Model<ParticlePipeline>>,
}

impl ParticleMgr {
    pub fn new(renderer: &mut Renderer) -> Self {
        Self {
            particles: Vec::new(),
            scheduler: HeartbeatScheduler::new(),
            instances: default_instances(renderer),
            model_cache: default_cache(renderer),
        }
    }

    pub fn handle_outcome(&mut self, outcome: &Outcome, scene_data: &SceneData) {
        let time = scene_data.state.get_time();
        let mut rng = rand::thread_rng();

        match outcome {
            Outcome::Explosion { pos, power } => {
                for _ in 0..150 {
                    self.particles.push(Particle::new(
                        Duration::from_millis(250),
                        time,
                        ParticleMode::Shrapnel,
                        *pos,
                    ));
                }
                for _ in 0..200 {
                    self.particles.push(Particle::new(
                        Duration::from_secs(4),
                        time,
                        ParticleMode::CampfireSmoke,
                        *pos + Vec2::<f32>::zero().map(|_| rng.gen_range(-1.0, 1.0) * power),
                    ));
                }
            },
            Outcome::ProjectileShot { .. } => {},
        }
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, scene_data: &SceneData) {
        if scene_data.particles_enabled {
            let now = Instant::now();

            // remove dead Particle
            self.particles.retain(|p| p.alive_until > now);

            // add new Particle
            self.maintain_body_particles(scene_data);
            self.maintain_boost_particles(scene_data);

            // update timings
            self.scheduler.maintain();
        } else {
            // remove all particle lifespans
            self.particles.clear();

            // remove all timings
            self.scheduler.clear();
        }

        self.upload_particles(renderer);
    }

    fn maintain_body_particles(&mut self, scene_data: &SceneData) {
        let ecs = scene_data.state.ecs();
        for (_i, (_entity, body, pos)) in (
            &ecs.entities(),
            &ecs.read_storage::<Body>(),
            &ecs.read_storage::<Pos>(),
        )
            .join()
            .enumerate()
        {
            match body {
                Body::Object(object::Body::CampfireLit) => {
                    self.maintain_campfirelit_particles(scene_data, pos)
                },
                Body::Object(object::Body::BoltFire) => {
                    self.maintain_boltfire_particles(scene_data, pos)
                },
                Body::Object(object::Body::BoltFireBig) => {
                    self.maintain_boltfirebig_particles(scene_data, pos)
                },
                Body::Object(object::Body::Bomb) => self.maintain_bomb_particles(scene_data, pos),
                _ => {},
            }
        }
    }

    fn maintain_campfirelit_particles(&mut self, scene_data: &SceneData, pos: &Pos) {
        let time = scene_data.state.get_time();

        for _ in 0..self.scheduler.heartbeats(Duration::from_millis(10)) {
            self.particles.push(Particle::new(
                Duration::from_millis(250),
                time,
                ParticleMode::CampfireFire,
                pos.0,
            ));

            self.particles.push(Particle::new(
                Duration::from_secs(10),
                time,
                ParticleMode::CampfireSmoke,
                pos.0,
            ));
        }
    }

    fn maintain_boltfire_particles(&mut self, scene_data: &SceneData, pos: &Pos) {
        let time = scene_data.state.get_time();

        for _ in 0..self.scheduler.heartbeats(Duration::from_millis(10)) {
            self.particles.push(Particle::new(
                Duration::from_millis(250),
                time,
                ParticleMode::CampfireFire,
                pos.0,
            ));
            self.particles.push(Particle::new(
                Duration::from_secs(1),
                time,
                ParticleMode::CampfireSmoke,
                pos.0,
            ));
        }
    }

    fn maintain_boltfirebig_particles(&mut self, scene_data: &SceneData, pos: &Pos) {
        let time = scene_data.state.get_time();

        // fire
        for _ in 0..self.scheduler.heartbeats(Duration::from_millis(3)) {
            self.particles.push(Particle::new(
                Duration::from_millis(250),
                time,
                ParticleMode::CampfireFire,
                pos.0,
            ));
        }

        // smoke
        for _ in 0..self.scheduler.heartbeats(Duration::from_millis(5)) {
            self.particles.push(Particle::new(
                Duration::from_secs(2),
                time,
                ParticleMode::CampfireSmoke,
                pos.0,
            ));
        }
    }

    fn maintain_bomb_particles(&mut self, scene_data: &SceneData, pos: &Pos) {
        let time = scene_data.state.get_time();

        for _ in 0..self.scheduler.heartbeats(Duration::from_millis(10)) {
            // sparks
            self.particles.push(Particle::new(
                Duration::from_millis(1500),
                time,
                ParticleMode::GunPowderSpark,
                pos.0,
            ));

            // smoke
            self.particles.push(Particle::new(
                Duration::from_secs(2),
                time,
                ParticleMode::CampfireSmoke,
                pos.0,
            ));
        }
    }

    fn maintain_boost_particles(&mut self, scene_data: &SceneData) {
        let state = scene_data.state;
        let ecs = state.ecs();
        let time = state.get_time();

        for (_i, (_entity, pos, character_state)) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<CharacterState>(),
        )
            .join()
            .enumerate()
        {
            if let CharacterState::Boost(_) = character_state {
                for _ in 0..self.scheduler.heartbeats(Duration::from_millis(10)) {
                    self.particles.push(Particle::new(
                        Duration::from_secs(15),
                        time,
                        ParticleMode::CampfireSmoke,
                        pos.0,
                    ));
                }
            }
        }
    }

    fn upload_particles(&mut self, renderer: &mut Renderer) {
        let all_cpu_instances = self
            .particles
            .iter()
            .map(|p| p.instance)
            .collect::<Vec<ParticleInstance>>();

        // TODO: optimise buffer writes
        let gpu_instances = renderer
            .create_instances(&all_cpu_instances)
            .expect("Failed to upload particle instances to the GPU!");

        self.instances = gpu_instances;
    }

    pub fn render(
        &self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
    ) {
        if scene_data.particles_enabled {
            let model = &self
                .model_cache
                .get(DEFAULT_MODEL_KEY)
                .expect("Expected particle model in cache");

            renderer.render_particles(model, globals, &self.instances, lights, shadows);
        }
    }

    pub fn particle_count(&self) -> usize { self.instances.count() }

    pub fn particle_count_visible(&self) -> usize { self.instances.count() }
}

fn default_instances(renderer: &mut Renderer) -> Instances<ParticleInstance> {
    let empty_vec = Vec::new();

    renderer
        .create_instances(&empty_vec)
        .expect("Failed to upload particle instances to the GPU!")
}

const DEFAULT_MODEL_KEY: &str = "voxygen.voxel.particle";

fn default_cache(renderer: &mut Renderer) -> HashMap<&'static str, Model<ParticlePipeline>> {
    let mut model_cache = HashMap::new();

    model_cache.entry(DEFAULT_MODEL_KEY).or_insert_with(|| {
        let offset = Vec3::zero();
        let lod_scale = Vec3::one();

        let vox = assets::load_expect::<DotVoxData>(DEFAULT_MODEL_KEY);

        let mesh = &Meshable::<ParticlePipeline, ParticlePipeline>::generate_mesh(
            &Segment::from(vox.as_ref()),
            (offset * lod_scale, Vec3::one() / lod_scale),
        )
        .0;

        renderer
            .create_model(mesh)
            .expect("Failed to create particle model")
    });

    model_cache
}

/// Accumulates heartbeats to be consumed on the next tick.
struct HeartbeatScheduler {
    /// Duration = Heartbeat Frequency/Intervals
    /// Instant = Last update time
    /// u8 = number of heartbeats since last update
    /// - if it's more frequent then tick rate, it could be 1 or more.
    /// - if it's less frequent then tick rate, it could be 1 or 0.
    /// - if it's equal to the tick rate, it could be between 2 and 0, due to
    /// delta time variance etc.
    timers: HashMap<Duration, (Instant, u8)>,
}

impl HeartbeatScheduler {
    pub fn new() -> Self {
        HeartbeatScheduler {
            timers: HashMap::new(),
        }
    }

    /// updates the last elapsed times and elasped counts
    /// this should be called once, and only once per tick.
    pub fn maintain(&mut self) {
        for (frequency, (last_update, heartbeats)) in self.timers.iter_mut() {
            // the number of iterations since last update
            *heartbeats =
                // TODO: use nightly api once stable; https://github.com/rust-lang/rust/issues/63139
                (last_update.elapsed().as_secs_f32() / frequency.as_secs_f32()).floor() as u8;

            // Instant::now() minus the heart beat count precision,
            // or alternatively as expressed below.
            *last_update += frequency.mul_f32(*heartbeats as f32);
            // Note: we want to preserve incomplete heartbeats, and include them
            // in the next update.
        }
    }

    /// returns the number of times this duration has elasped since the last
    /// tick:
    /// - if it's more frequent then tick rate, it could be 1 or more.
    /// - if it's less frequent then tick rate, it could be 1 or 0.
    /// - if it's equal to the tick rate, it could be between 2 and 0, due to
    /// delta time variance.
    pub fn heartbeats(&mut self, frequency: Duration) -> u8 {
        self.timers
            .entry(frequency)
            .or_insert_with(|| (Instant::now(), 0))
            .1
    }

    pub fn clear(&mut self) { self.timers.clear() }
}

struct Particle {
    alive_until: Instant, // created_at + lifespan
    instance: ParticleInstance,
}

impl Particle {
    fn new(lifespan: Duration, time: f64, mode: ParticleMode, pos: Vec3<f32>) -> Self {
        Particle {
            alive_until: Instant::now() + lifespan,
            instance: ParticleInstance::new(time, mode, pos),
        }
    }
}
