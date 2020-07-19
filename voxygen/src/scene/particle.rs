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
};
use dot_vox::DotVoxData;
use hashbrown::HashMap;
use rand::Rng;
use specs::{Join, WorldExt};
use std::time::{Duration, Instant};
use vek::Vec3;

struct Particles {
    alive_until: Instant, // created_at + lifespan
    instances: Instances<ParticleInstance>,
}

pub struct ParticleMgr {
    // keep track of lifespans
    particles: Vec<Particles>,
    model_cache: HashMap<&'static str, Model<ParticlePipeline>>,
}

const MODEL_KEY: &str = "voxygen.voxel.particle";

impl ParticleMgr {
    pub fn new(renderer: &mut Renderer) -> Self {
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

            let model = renderer
                .create_model(mesh)
                .expect("Failed to create particle model");

            model
        });

        Self {
            particles: Vec::new(),
            model_cache,
        }
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, scene_data: &SceneData) {
        let now = Instant::now();

        // remove dead particles
        self.particles.retain(|p| p.alive_until > now);

        self.maintain_body_particles(renderer, scene_data);

        self.maintain_boost_particles(renderer, scene_data);
    }

    fn maintain_body_particles(&mut self, renderer: &mut Renderer, scene_data: &SceneData) {
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
                    self.maintain_campfirelit_particles(renderer, scene_data, pos)
                },
                Body::Object(object::Body::BoltFire) => {
                    self.maintain_boltfire_particles(renderer, scene_data, pos)
                },
                Body::Object(object::Body::BoltFireBig) => {
                    self.maintain_boltfirebig_particles(renderer, scene_data, pos)
                },
                Body::Object(object::Body::Bomb) => {
                    self.maintain_bomb_particles(renderer, scene_data, pos)
                },
                // Body::Object(object::Body::Pouch) => {
                //     self.maintain_pouch_particles(renderer, scene_data, pos)
                // },
                _ => {},
            }
        }
    }

    fn maintain_campfirelit_particles(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        pos: &Pos,
    ) {
        let time = scene_data.state.get_time();
        let now = Instant::now();
        let mut rng = rand::thread_rng();

        let fire_cpu_insts = vec![ParticleInstance::new(
            time,
            rng.gen(),
            ParticleMode::CampfireFire,
            pos.0,
        )];

        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(250),
            instances: renderer
                .create_instances(&fire_cpu_insts)
                .expect("Failed to upload particle instances to the GPU!"),
        });

        let smoke_cpu_insts = vec![ParticleInstance::new(
            time,
            rng.gen(),
            ParticleMode::CampfireSmoke,
            pos.0,
        )];

        let smoke_cpu_insts = renderer
            .create_instances(&smoke_cpu_insts)
            .expect("Failed to upload particle instances to the GPU!");

        self.particles.push(Particles {
            alive_until: now + Duration::from_secs(10),
            instances: smoke_cpu_insts,
        });
    }

    fn maintain_boltfire_particles(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        pos: &Pos,
    ) {
        let time = scene_data.state.get_time();
        let now = Instant::now();
        let mut rng = rand::thread_rng();

        let fire_cpu_insts = vec![ParticleInstance::new(
            time,
            rng.gen(),
            ParticleMode::CampfireFire,
            pos.0,
        )];

        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(250),
            instances: renderer
                .create_instances(&fire_cpu_insts)
                .expect("Failed to upload particle instances to the GPU!"),
        });

        let smoke_cpu_insts = vec![ParticleInstance::new(
            time,
            rng.gen(),
            ParticleMode::CampfireSmoke,
            pos.0,
        )];

        let smoke_cpu_insts = renderer
            .create_instances(&smoke_cpu_insts)
            .expect("Failed to upload particle instances to the GPU!");

        self.particles.push(Particles {
            alive_until: now + Duration::from_secs(1),
            instances: smoke_cpu_insts,
        });
    }

    fn maintain_boltfirebig_particles(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        pos: &Pos,
    ) {
        let time = scene_data.state.get_time();
        let now = Instant::now();
        let mut rng = rand::thread_rng();

        let fire_cpu_insts = vec![
            ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireFire, pos.0),
            ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireFire, pos.0),
        ];

        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(250),
            instances: renderer
                .create_instances(&fire_cpu_insts)
                .expect("Failed to upload particle instances to the GPU!"),
        });

        let smoke_cpu_insts = vec![
            ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireSmoke, pos.0),
            ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireSmoke, pos.0),
            ParticleInstance::new(time, rng.gen(), ParticleMode::CampfireSmoke, pos.0),
        ];

        let smoke_cpu_insts = renderer
            .create_instances(&smoke_cpu_insts)
            .expect("Failed to upload particle instances to the GPU!");

        self.particles.push(Particles {
            alive_until: now + Duration::from_secs(2),
            instances: smoke_cpu_insts,
        });
    }

    fn maintain_bomb_particles(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        pos: &Pos,
    ) {
        let time = scene_data.state.get_time();
        let now = Instant::now();
        let mut rng = rand::thread_rng();

        let fire_cpu_insts = vec![
            ParticleInstance::new(time, rng.gen(), ParticleMode::GunPowderSpark, pos.0),
            ParticleInstance::new(time, rng.gen(), ParticleMode::GunPowderSpark, pos.0),
            ParticleInstance::new(time, rng.gen(), ParticleMode::GunPowderSpark, pos.0),
            ParticleInstance::new(time, rng.gen(), ParticleMode::GunPowderSpark, pos.0),
            ParticleInstance::new(time, rng.gen(), ParticleMode::GunPowderSpark, pos.0),
        ];

        self.particles.push(Particles {
            alive_until: now + Duration::from_millis(1500),
            instances: renderer
                .create_instances(&fire_cpu_insts)
                .expect("Failed to upload particle instances to the GPU!"),
        });

        let smoke_cpu_insts = vec![ParticleInstance::new(
            time,
            rng.gen(),
            ParticleMode::CampfireSmoke,
            pos.0,
        )];

        let smoke_cpu_insts = renderer
            .create_instances(&smoke_cpu_insts)
            .expect("Failed to upload particle instances to the GPU!");

        self.particles.push(Particles {
            alive_until: now + Duration::from_secs(2),
            instances: smoke_cpu_insts,
        });
    }

    // fn maintain_pouch_particles(
    //     &mut self,
    //     renderer: &mut Renderer,
    //     scene_data: &SceneData,
    //     pos: &Pos,
    // ) {
    //     let time = scene_data.state.get_time();
    //     let now = Instant::now();
    //     let mut rng = rand::thread_rng();

    //     let smoke_cpu_insts = vec![ParticleInstance::new(
    //         time,
    //         rng.gen(),
    //         ParticleMode::CampfireSmoke,
    //         pos.0,
    //     )];

    //     let smoke_cpu_insts = renderer
    //         .create_instances(&smoke_cpu_insts)
    //         .expect("Failed to upload particle instances to the GPU!");

    //     self.particles.push(Particles {
    //         alive_until: now + Duration::from_secs(1),
    //         instances: smoke_cpu_insts,
    //     });
    // }

    fn maintain_boost_particles(&mut self, renderer: &mut Renderer, scene_data: &SceneData) {
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
                let cpu_insts = vec![ParticleInstance::new(
                    time,
                    rng.gen(),
                    ParticleMode::CampfireSmoke,
                    pos.0,
                )];

                let gpu_insts = renderer
                    .create_instances(&cpu_insts)
                    .expect("Failed to upload particle instances to the GPU!");

                self.particles.push(Particles {
                    alive_until: now + Duration::from_secs(15),
                    instances: gpu_insts,
                });
            }
        }
    }

    pub fn render(
        &self,
        renderer: &mut Renderer,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
    ) {
        let model = &self
            .model_cache
            .get(MODEL_KEY)
            .expect("Expected particle model in cache");

        for particle in &self.particles {
            renderer.render_particles(model, globals, &particle.instances, lights, shadows);
        }
    }
}
