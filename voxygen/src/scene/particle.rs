use super::SceneData;
use crate::{
    mesh::Meshable,
    render::{
        mesh::Quad, Consts, Globals, Instances, Light, Mesh, Model, ParticleInstance,
        ParticlePipeline, Renderer, Shadow,
    },
};
use common::{
    assets,
    comp::{visual::ParticleEmitterMode, Ori, ParticleEmitter, Pos, Vel},
    figure::Segment,
    vol::BaseVol,
};
use dot_vox::DotVoxData;
use hashbrown::HashMap;
use rand::Rng;
use specs::{Entity as EcsEntity, Join, WorldExt};
use std::time::{Duration, Instant};
use tracing::{debug, error, warn};
use vek::{Mat4, Rgb, Vec3};

struct Particles {
    // this is probably nieve,
    // could cache and re-use between particles,
    // should be a cache key?
    // model: Model<ParticlePipeline>,
    // created_at: Instant,
    // lifespan: Duration,
    alive_until: Instant, // created_at + lifespan

    instances: Instances<ParticleInstance>,
}

struct Emitter {
    last_emit: Instant,
}

pub struct ParticleMgr {
    // to keep track of spawn intervals
    emitters: HashMap<EcsEntity, Emitter>,

    // to keep track of lifespans
    particles: Vec<Particles>,

    model_cache: HashMap<&'static str, Model<ParticlePipeline>>,

    beginning_of_time: Instant,
}

const MODEL_KEY: &str = "voxygen.voxel.not_found";

impl ParticleMgr {
    pub fn new(renderer: &mut Renderer) -> Self {
        let mut model_cache = HashMap::new();

        let model = model_cache.entry(MODEL_KEY).or_insert_with(|| {
            let offset = Vec3::zero();
            let lod_scale = Vec3::one();

            // TODO: from cache
            let vox = assets::load_expect::<DotVoxData>(MODEL_KEY);

            // TODO: from cache
            let mesh = &Meshable::<ParticlePipeline, ParticlePipeline>::generate_mesh(
                &Segment::from(vox.as_ref()),
                (offset * lod_scale, Vec3::one() / lod_scale),
            )
            .0;

            // TODO: from cache
            let model = renderer
                .create_model(mesh)
                .expect("Failed to create particle model");

            model
        });

        Self {
            emitters: HashMap::new(),
            particles: Vec::new(),
            model_cache,
            beginning_of_time: Instant::now(),
        }
    }

    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        focus_pos: Vec3<f32>,
        loaded_distance: f32,
        view_mat: Mat4<f32>,
        proj_mat: Mat4<f32>,
    ) {
        let state = scene_data.state;
        let ecs = state.ecs();

        let tick = scene_data.tick;

        let now = Instant::now();
        let beginning_of_time1 = self.beginning_of_time.clone();

        // remove dead emitters
        self.emitters.retain(|k, _v| ecs.is_alive(*k));

        // remove dead particles
        self.particles.retain(|p| p.alive_until > now);

        // add living entities particles
        for (_i, (entity, particle_emitter, pos, ori, vel)) in (
            &ecs.entities(),
            &ecs.read_storage::<ParticleEmitter>(),
            &ecs.read_storage::<Pos>(),
            ecs.read_storage::<Ori>().maybe(),
            ecs.read_storage::<Vel>().maybe(),
        )
            .join()
            .enumerate()
        {
            let emitter = self.emitters.entry(entity).or_insert_with(|| Emitter {
                last_emit: beginning_of_time1, // self.beginning_of_time.clone()
            });

            if emitter.last_emit + particle_emitter.frequency < now {
                emitter.last_emit = Instant::now();

                let cpu_insts =
                    into_particle_instances(particle_emitter, renderer, tick, pos, ori, vel);

                let gpu_insts = renderer
                    .create_instances(&cpu_insts)
                    .expect("Failed to upload particle instances to the GPU!");

                let entry = self.particles.push(Particles {
                    alive_until: now + particle_emitter.initial_lifespan,
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
        focus_pos: Vec3<f32>,
    ) {
        for particle in &self.particles {
            renderer.render_particles(
                &self
                    .model_cache
                    .get(MODEL_KEY)
                    .expect("Expected particle model in cache"),
                globals,
                &particle.instances,
                lights,
                shadows,
            );
        }
    }
}

fn into_particle_instances(
    particle_emitter: &ParticleEmitter,
    renderer: &mut Renderer,
    tick: u64,
    pos: &Pos,
    ori: Option<&Ori>,
    vel: Option<&Vel>,
) -> Vec<ParticleInstance> {
    let mut rng = rand::thread_rng();
    let vel_default = Vel::default();
    let vel2 = vel.unwrap_or_else(|| &vel_default).0;

    let mut instances_vec = Vec::new();

    for x in 0..rng.gen_range(particle_emitter.count.0, particle_emitter.count.1) {
        // how does ParticleEmitterMode fit in here?
        // can we have a ParticleInstance type per ParticleEmitterMode?
        // can we mix and match instance types in the same instances_vec?
        instances_vec.push(ParticleInstance::new(
            Mat4::identity()
                // initial rotation
                .rotated_x(rng.gen_range(particle_emitter.initial_orientation.0.x * std::f32::consts::PI * 2.0, particle_emitter.initial_orientation.1.x * std::f32::consts::PI * 2.0))
                .rotated_y(rng.gen_range(particle_emitter.initial_orientation.0.y * std::f32::consts::PI * 2.0, particle_emitter.initial_orientation.1.y * std::f32::consts::PI * 2.0))
                .rotated_z(rng.gen_range(particle_emitter.initial_orientation.0.z * std::f32::consts::PI * 2.0, particle_emitter.initial_orientation.1.z * std::f32::consts::PI * 2.0))
                // initial scale
                .scaled_3d(rng.gen_range(particle_emitter.initial_scale.0, particle_emitter.initial_scale.1))
                // inition position
                .translated_3d(
                    pos.0 // relative
                        + Vec3::new(
                            rng.gen_range(particle_emitter.initial_offset.0.x, particle_emitter.initial_offset.1.x),
                            rng.gen_range(particle_emitter.initial_offset.0.y, particle_emitter.initial_offset.1.y),
                            rng.gen_range(particle_emitter.initial_offset.0.z, particle_emitter.initial_offset.1.z),
                        ),
                ),
            Rgb::broadcast(1.0), // instance color
            vel2 + Vec3::broadcast(rng.gen_range(-5.0, 5.0)),
            tick,
            rng.gen_range(0.0, 20.0),       // wind sway
            ParticleEmitterMode::Sprinkler, // particle_emitter.mode */
        ));
    }

    instances_vec
}
