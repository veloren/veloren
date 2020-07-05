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
use tracing::{debug, error, warn};
use vek::{Mat4, Rgb, Vec3};
struct Particles {
    // this is probably nieve,
    // could cache and re-use between particles,
    // should be a cache key?
    // model: Model<ParticlePipeline>,
    instances: Instances<ParticleInstance>,
}

pub struct ParticleMgr {
    entity_particles: HashMap<EcsEntity, Vec<Particles>>,
    model_cache: Model<ParticlePipeline>,
}

impl ParticleMgr {
    pub fn new(renderer: &mut Renderer) -> Self {
        let offset = Vec3::zero();
        let lod_scale = Vec3::one();

        // TODO: from cache
        let vox = assets::load_expect::<DotVoxData>("voxygen.voxel.not_found");

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

        Self {
            entity_particles: HashMap::new(),
            model_cache: model,
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

        // remove dead particles

        // remove dead entities, with dead particles
        self.entity_particles.retain(|k, v| ecs.is_alive(*k));

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
            let entry = self
                .entity_particles
                .entry(entity)
                .or_insert_with(|| into_particles(renderer, tick, particle_emitter, pos, ori, vel));
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
        for particles in self.entity_particles.values() {
            for particle in particles {
                renderer.render_particles(
                    &self.model_cache,
                    globals,
                    &particle.instances,
                    lights,
                    shadows,
                );
            }
        }
    }
}

fn into_particles(
    renderer: &mut Renderer,
    tick: u64,
    particle_emitter: &ParticleEmitter,
    pos: &Pos,
    ori: Option<&Ori>,
    vel: Option<&Vel>,
) -> Vec<Particles> {
    let mut rng = rand::thread_rng();

    let desired_instance_count = 100;

    // let ori_default = Ori::default();
    let vel_default = Vel::default();

    // let ori2 = ori.unwrap_or_else(|| &ori_default);
    let vel2 = vel.unwrap_or_else(|| &vel_default).0;
    let mut instances_vec = Vec::new();

    for x in 0..desired_instance_count {
        // how does ParticleEmitterMode fit in here?
        // can we have a ParticleInstance type per ParticleEmitterMode?
        // can we mix and match instance types in the same instances_vec?
        instances_vec.push(ParticleInstance::new(
            Mat4::identity()
                // initial rotation
                .rotated_x(rng.gen_range(0.0, 3.14 * 2.0))
                .rotated_y(rng.gen_range(0.0, 3.14 * 2.0))
                .rotated_z(rng.gen_range(0.0, 3.14 * 2.0))
                // inition position
                .translated_3d(
                    pos.0
                        + Vec3::new(
                            rng.gen_range(-5.0, 5.0),
                            rng.gen_range(-5.0, 5.0),
                            rng.gen_range(0.0, 10.0),
                        ),
                ),
            Rgb::broadcast(1.0), // instance color
            vel2 + Vec3::broadcast(rng.gen_range(-5.0, 5.0)),
            tick,
            rng.gen_range(0.0, 20.0),       // wind sway
            ParticleEmitterMode::Sprinkler, // particle_emitter.mode */
        ));
    }

    let instances = renderer
        .create_instances(&instances_vec)
        .expect("Failed to upload particle instances to the GPU!");

    vec![Particles { instances }]
}
