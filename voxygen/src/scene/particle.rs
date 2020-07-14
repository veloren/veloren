use super::SceneData;
use crate::{
    mesh::Meshable,
    render::{
        mesh::Quad, Consts, Globals, Instances, Light, Model, ParticleInstance, ParticlePipeline,
        Renderer, Shadow,
    },
};
use common::{
    assets,
    comp::{
        visual::ParticleEmitterMode, CharacterState, Ori, ParticleEmitter, ParticleEmitters, Pos,
        Vel,
    },
    figure::Segment,
};
use dot_vox::DotVoxData;
use hashbrown::HashMap;
use rand::Rng;
use specs::{Entity as EcsEntity, Join, WorldExt};
use std::time::{Duration, Instant};
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

const MODEL_KEY: &str = "voxygen.voxel.particle";

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
        let now = Instant::now();
        let state = scene_data.state;
        let ecs = state.ecs();

        // remove dead emitters
        self.emitters.retain(|k, _v| ecs.is_alive(*k));

        // remove dead particles
        self.particles.retain(|p| p.alive_until > now);

        // add ParticleEmitter particles
        self.maintain_particle_emitter(renderer, scene_data);

        self.maintain_ability_particles(renderer, scene_data);
    }

    fn maintain_particle_emitter(&mut self, renderer: &mut Renderer, scene_data: &SceneData) {
        let state = scene_data.state;
        let ecs = state.ecs();

        let time = state.get_time();

        let now = Instant::now();
        let beginning_of_time1 = self.beginning_of_time.clone();

        for (_i, (entity, particle_emitters, pos, ori, vel)) in (
            &ecs.entities(),
            &ecs.read_storage::<ParticleEmitters>(),
            &ecs.read_storage::<Pos>(),
            ecs.read_storage::<Ori>().maybe(),
            ecs.read_storage::<Vel>().maybe(),
        )
            .join()
            .enumerate()
        {
            for particle_emitter in &particle_emitters.0 {
                // TODO: track multiple particle_emitter last_emit
                let emitter = self.emitters.entry(entity).or_insert_with(|| Emitter {
                    last_emit: beginning_of_time1, // self.beginning_of_time.clone()
                });

                if emitter.last_emit + particle_emitter.frequency < now {
                    emitter.last_emit = Instant::now();

                    let cpu_insts =
                        into_particle_instances(&particle_emitter, renderer, time, pos, ori, vel);

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
    }

    fn maintain_ability_particles(&mut self, renderer: &mut Renderer, scene_data: &SceneData) {
        let state = scene_data.state;
        let ecs = state.ecs();

        let time = state.get_time();

        let now = Instant::now();
        let beginning_of_time1 = self.beginning_of_time.clone();

        for (_i, (entity, pos, character_state)) in (
            &ecs.entities(),
            //&ecs.read_storage::<ParticleEmitter>(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<CharacterState>(),
        )
            .join()
            .enumerate()
        {
            // let emitter = self.emitters.entry(entity).or_insert_with(|| Emitter {
            //     last_emit: beginning_of_time1, // self.beginning_of_time.clone()
            // });

            // if emitter.last_emit + particle_emitter.frequency < now {
            //     emitter.last_emit = Instant::now();
            // }

            if let CharacterState::BasicMelee(melee_data) = character_state {
                // TODO: configure the emitter on the ability instead.
                let particle_emitter = ParticleEmitter {
                    count: (30, 50),
                    frequency: Duration::from_millis(1000), // doesn't matter
                    initial_lifespan: Duration::from_millis(1000),
                    initial_offset: (
                        Vec3::new(1.0, -1.0, 0.0),
                        Vec3::new(1.01, 1.0, 2.0), /* TODO: cone // melee_data.max_angle */
                    ),
                    initial_orientation: (Vec3::zero(), Vec3::one()),
                    initial_scale: (1.0, 3.0),
                    mode: ParticleEmitterMode::Sprinkler,
                    initial_velocity: (
                        Vec3::new(1.0, 0.0, 0.0),
                        Vec3::new(10.0, 0.01, 0.01), /* TODO: cone // melee_data.max_angle */
                    ),
                    initial_col: (Rgb::zero(), Rgb::one()),
                };

                let cpu_insts =
                    into_particle_instances(&particle_emitter, renderer, time, pos, None, None);

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
    time: f64,
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
            Rgb::new(
                rng.gen_range(particle_emitter.initial_col.0.r, particle_emitter.initial_col.1.r),
                rng.gen_range(particle_emitter.initial_col.0.g, particle_emitter.initial_col.1.g),
                rng.gen_range(particle_emitter.initial_col.0.b, particle_emitter.initial_col.1.b),
            ), // instance color
            vel2 // relative
            + Vec3::new(
                rng.gen_range(particle_emitter.initial_velocity.0.x, particle_emitter.initial_velocity.1.x),
                rng.gen_range(particle_emitter.initial_velocity.0.y, particle_emitter.initial_velocity.1.y),
                rng.gen_range(particle_emitter.initial_velocity.0.z, particle_emitter.initial_velocity.1.z),
            ),
            time,
            rng.gen_range(0.0, 20.0),       // wind sway
            ParticleEmitterMode::Sprinkler, // particle_emitter.mode */
        ));
    }

    instances_vec
}
