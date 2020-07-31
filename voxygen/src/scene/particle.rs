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

struct Particles {
    alive_until: Instant, // created_at + lifespan
    instance: ParticleInstance,
}

pub struct ParticleMgr {
    // keep track of lifespans
    particles: Vec<Particles>,
    instances: Instances<ParticleInstance>,
    model_cache: HashMap<&'static str, Model<ParticlePipeline>>,
}

const MODEL_KEY: &str = "voxygen.voxel.particle";

impl ParticleMgr {
    pub fn new(renderer: &mut Renderer) -> Self {
        Self {
            particles: Vec::new(),
            instances: default_instances(renderer),
            model_cache: default_cache(renderer),
        }
    }

    pub fn particle_count(&self) -> usize { self.instances.count() }

    pub fn particle_count_visible(&self) -> usize { self.instances.count() }

    pub fn handle_outcome(&mut self, outcome: &Outcome, scene_data: &SceneData) {
        let time = scene_data.state.get_time();
        let now = Instant::now();
        let mut rng = rand::thread_rng();

        match outcome {
            Outcome::Explosion { pos, power } => {
                for _ in 0..64 {
                    self.particles.push(Particles {
                        alive_until: now + Duration::from_secs(4),
                        instance: ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireSmoke, *pos + Vec2::<f32>::zero().map(|_| rng.gen_range(-1.0, 1.0) * power)),
                    });
                }
            },
            _ => {},
        }
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, scene_data: &SceneData) {
        if scene_data.particles_enabled {
            let now = Instant::now();

            // remove dead particles
            self.particles.retain(|p| p.alive_until > now);

            self.maintain_body_particles(scene_data);
            self.maintain_boost_particles(scene_data);

            self.upload_particles(renderer);
        } else {
            self.particles.clear();
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
        let now = Instant::now();
        let mut rng = rand::thread_rng();

        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(250),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireFire, pos.0),
        });

        self.particles.push(Particles {
            alive_until: now + Duration::from_secs(10),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireSmoke, pos.0),
        });
    }

    fn maintain_boltfire_particles(&mut self, scene_data: &SceneData, pos: &Pos) {
        let time = scene_data.state.get_time();
        let now = Instant::now();
        let mut rng = rand::thread_rng();

        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(250),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireFire, pos.0),
        });

        self.particles.push(Particles {
            alive_until: now + Duration::from_secs(1),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireSmoke, pos.0),
        });
    }

    fn maintain_boltfirebig_particles(&mut self, scene_data: &SceneData, pos: &Pos) {
        let time = scene_data.state.get_time();
        let now = Instant::now();
        let mut rng = rand::thread_rng();

        // fire
        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(250),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireFire, pos.0),
        });
        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(250),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireFire, pos.0),
        });

        // smoke
        self.particles.push(Particles {
            alive_until: now + Duration::from_secs(2),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireSmoke, pos.0),
        });
        self.particles.push(Particles {
            alive_until: now + Duration::from_secs(2),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireSmoke, pos.0),
        });
        self.particles.push(Particles {
            alive_until: now + Duration::from_secs(2),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireSmoke, pos.0),
        });
    }

    fn maintain_bomb_particles(&mut self, scene_data: &SceneData, pos: &Pos) {
        let time = scene_data.state.get_time();
        let now = Instant::now();
        let mut rng = rand::thread_rng();

        // sparks
        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(1500),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::GunPowderSpark, pos.0),
        });
        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(1500),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::GunPowderSpark, pos.0),
        });
        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(1500),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::GunPowderSpark, pos.0),
        });
        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(1500),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::GunPowderSpark, pos.0),
        });
        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(1500),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::GunPowderSpark, pos.0),
        });

        // smoke
        self.particles.push(Particles {
            alive_until: now + Duration::from_secs(2),
            instance: ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireSmoke, pos.0),
        });
    }

    fn maintain_boost_particles(&mut self, scene_data: &SceneData) {
        let state = scene_data.state;
        let ecs = state.ecs();
        let time = state.get_time();
        let now = Instant::now();
        let mut rng = rand::thread_rng();

        for (_i, (_entity, pos, character_state)) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<CharacterState>(),
        )
            .join()
            .enumerate()
        {
            if let CharacterState::Boost(_) = character_state {
                self.particles.push(Particles {
                    alive_until: now + Duration::from_secs(15),
                    instance: ParticleInstance::new(
                        time,
                        rng.gen(),
                        ParticleMode::CampfireSmoke,
                        pos.0,
                    ),
                });
            }
        }
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
                .get(MODEL_KEY)
                .expect("Expected particle model in cache");

            renderer.render_particles(model, globals, &self.instances, lights, shadows);
        }
    }
}

fn default_instances(renderer: &mut Renderer) -> Instances<ParticleInstance> {
    let empty_vec = Vec::new();

    renderer
        .create_instances(&empty_vec)
        .expect("Failed to upload particle instances to the GPU!")
}

fn default_cache(renderer: &mut Renderer) -> HashMap<&'static str, Model<ParticlePipeline>> {
    let mut model_cache = HashMap::new();

    model_cache.entry(MODEL_KEY).or_insert_with(|| {
        let offset = Vec3::zero();
        let lod_scale = Vec3::one();

        let vox = assets::load_expect::<DotVoxData>(MODEL_KEY);

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
