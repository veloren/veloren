use super::{terrain::BlocksOfInterest, SceneData, Terrain};
use crate::{
    mesh::{greedy::GreedyMesh, Meshable},
    render::{
        pipelines::particle::ParticleMode, GlobalModel, Instances, Light, LodData, Model,
        ParticleInstance, ParticlePipeline, Renderer,
    },
};
use common::{
    assets::{AssetExt, DotVoxAsset},
    combat::CombatEffect,
    comp::{item::Reagent, object, BeamSegment, Body, CharacterState, Ori, Pos, Shockwave},
    figure::Segment,
    outcome::Outcome,
    resources::DeltaTime,
    span,
    spiral::Spiral2d,
    terrain::TerrainChunk,
    vol::{RectRasterableVol, SizedVol},
};
use hashbrown::HashMap;
use rand::prelude::*;
use specs::{Join, WorldExt};
use std::{f32::consts::PI, time::Duration};
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
        span!(_guard, "handle_outcome", "ParticleMgr::handle_outcome");
        let time = scene_data.state.get_time();
        let mut rng = rand::thread_rng();

        match outcome {
            Outcome::Explosion {
                pos,
                power,
                radius,
                is_attack,
                reagent,
            } => {
                if *is_attack {
                    if *power < 0.0 {
                        self.particles.resize_with(
                            self.particles.len() + (60.0 * power.abs()) as usize,
                            || {
                                Particle::new_directed(
                                    Duration::from_secs_f32(rng.gen_range(0.2..3.0)),
                                    time,
                                    ParticleMode::EnergyNature,
                                    *pos,
                                    *pos + Vec3::<f32>::zero()
                                        .map(|_| rng.gen_range(-1.0..1.0))
                                        .normalized()
                                        * rng.gen_range(1.0..*radius),
                                )
                            },
                        );
                    } else {
                        self.particles.resize_with(
                            self.particles.len() + (75.0 * power.abs()) as usize,
                            || {
                                Particle::new_directed(
                                    Duration::from_millis(500),
                                    time,
                                    ParticleMode::Explosion,
                                    *pos,
                                    *pos + Vec3::<f32>::zero()
                                        .map(|_| rng.gen_range(-1.0..1.0))
                                        .normalized()
                                        * *radius,
                                )
                            },
                        );
                    }
                } else {
                    self.particles.resize_with(
                        self.particles.len() + if reagent.is_some() { 300 } else { 150 },
                        || {
                            Particle::new(
                                Duration::from_millis(if reagent.is_some() { 1000 } else { 250 }),
                                time,
                                match reagent {
                                    Some(Reagent::Blue) => ParticleMode::FireworkBlue,
                                    Some(Reagent::Green) => ParticleMode::FireworkGreen,
                                    Some(Reagent::Purple) => ParticleMode::FireworkPurple,
                                    Some(Reagent::Red) => ParticleMode::FireworkRed,
                                    Some(Reagent::Yellow) => ParticleMode::FireworkYellow,
                                    None => ParticleMode::Shrapnel,
                                },
                                *pos,
                            )
                        },
                    );

                    self.particles.resize_with(
                        self.particles.len() + if reagent.is_some() { 100 } else { 200 },
                        || {
                            Particle::new(
                                Duration::from_secs(4),
                                time,
                                ParticleMode::CampfireSmoke,
                                *pos + Vec3::<f32>::zero()
                                    .map(|_| rng.gen_range(-1.0..1.0))
                                    .normalized()
                                    * *radius,
                            )
                        },
                    );
                }
            },
            Outcome::ProjectileShot { .. } => {},
            _ => {},
        }
    }

    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        terrain: &Terrain<TerrainChunk>,
        lights: &mut Vec<Light>,
    ) {
        span!(_guard, "maintain", "ParticleMgr::maintain");
        if scene_data.particles_enabled {
            // update timings
            self.scheduler.maintain(scene_data.state.get_time());

            // remove dead Particle
            self.particles
                .retain(|p| p.alive_until > scene_data.state.get_time());

            // add new Particle
            self.maintain_body_particles(scene_data);
            self.maintain_boost_particles(scene_data);
            self.maintain_beam_particles(scene_data, lights);
            self.maintain_block_particles(scene_data, terrain);
            self.maintain_shockwave_particles(scene_data);
        } else {
            // remove all particle lifespans
            self.particles.clear();

            // remove all timings
            self.scheduler.clear();
        }

        self.upload_particles(renderer);
    }

    fn maintain_body_particles(&mut self, scene_data: &SceneData) {
        span!(
            _guard,
            "body_particles",
            "ParticleMgr::maintain_body_particles"
        );
        let ecs = scene_data.state.ecs();
        for (body, pos) in (&ecs.read_storage::<Body>(), &ecs.read_storage::<Pos>()).join() {
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
                Body::Object(object::Body::BoltNature) => {
                    self.maintain_boltnature_particles(scene_data, pos)
                },
                Body::Object(
                    object::Body::Bomb
                    | object::Body::FireworkBlue
                    | object::Body::FireworkGreen
                    | object::Body::FireworkPurple
                    | object::Body::FireworkRed
                    | object::Body::FireworkYellow,
                ) => self.maintain_bomb_particles(scene_data, pos),
                _ => {},
            }
        }
    }

    fn maintain_campfirelit_particles(&mut self, scene_data: &SceneData, pos: &Pos) {
        span!(
            _guard,
            "campfirelit_particles",
            "ParticleMgr::maintain_campfirelit_particles"
        );
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
                pos.0.map(|e| e + thread_rng().gen_range(-0.25..0.25)),
            ));
        }
    }

    fn maintain_boltfire_particles(&mut self, scene_data: &SceneData, pos: &Pos) {
        span!(
            _guard,
            "boltfire_particles",
            "ParticleMgr::maintain_boltfire_particles"
        );
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
        span!(
            _guard,
            "boltfirebig_particles",
            "ParticleMgr::maintain_boltfirebig_particles"
        );
        let time = scene_data.state.get_time();

        // fire
        self.particles.resize_with(
            self.particles.len() + usize::from(self.scheduler.heartbeats(Duration::from_millis(3))),
            || {
                Particle::new(
                    Duration::from_millis(250),
                    time,
                    ParticleMode::CampfireFire,
                    pos.0,
                )
            },
        );

        // smoke
        self.particles.resize_with(
            self.particles.len() + usize::from(self.scheduler.heartbeats(Duration::from_millis(5))),
            || {
                Particle::new(
                    Duration::from_secs(2),
                    time,
                    ParticleMode::CampfireSmoke,
                    pos.0,
                )
            },
        );
    }

    fn maintain_boltnature_particles(&mut self, scene_data: &SceneData, pos: &Pos) {
        let time = scene_data.state.get_time();

        // nature
        self.particles.resize_with(
            self.particles.len() + usize::from(self.scheduler.heartbeats(Duration::from_millis(3))),
            || {
                Particle::new(
                    Duration::from_millis(250),
                    time,
                    ParticleMode::CampfireSmoke,
                    pos.0,
                )
            },
        );
    }

    fn maintain_bomb_particles(&mut self, scene_data: &SceneData, pos: &Pos) {
        span!(
            _guard,
            "bomb_particles",
            "ParticleMgr::maintain_bomb_particles"
        );
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
        span!(
            _guard,
            "boost_particles",
            "ParticleMgr::maintain_boost_particles"
        );
        let state = scene_data.state;
        let ecs = state.ecs();
        let time = state.get_time();

        for (pos, character_state) in (
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<CharacterState>(),
        )
            .join()
        {
            if let CharacterState::Boost(_) = character_state {
                self.particles.resize_with(
                    self.particles.len()
                        + usize::from(self.scheduler.heartbeats(Duration::from_millis(10))),
                    || {
                        Particle::new(
                            Duration::from_secs(15),
                            time,
                            ParticleMode::CampfireSmoke,
                            pos.0,
                        )
                    },
                );
            }
        }
    }

    fn maintain_beam_particles(&mut self, scene_data: &SceneData, lights: &mut Vec<Light>) {
        let state = scene_data.state;
        let ecs = state.ecs();
        let time = state.get_time();
        let dt = scene_data.state.ecs().fetch::<DeltaTime>().0;

        for (pos, ori, beam) in (
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Ori>(),
            &ecs.read_storage::<BeamSegment>(),
        )
            .join()
            .filter(|(_, _, b)| b.creation.map_or(true, |c| (c + dt as f64) >= time))
        {
//
            let range = beam.properties.speed * beam.properties.duration.as_secs_f32();
            if beam
                .properties
                .attack
                .effects()
                .any(|e| matches!(e.effect(), CombatEffect::Heal(h) if *h > 0.0))
            {
                // Emit a light when using healing
                lights.push(Light::new(pos.0, Rgb::new(0.1, 1.0, 0.15), 1.0));
                for i in 0..self.scheduler.heartbeats(Duration::from_millis(1)) {
                    self.particles.push(Particle::new_beam(
                        beam.properties.duration,
                        time + i as f64 / 1000.0,
                        ParticleMode::HealingBeam,
                        pos.0,
                        pos.0 + *ori.look_dir() * range,
                    ));
//
            if let CharacterState::BasicBeam(b) = character_state {
                let particle_ori = b.particle_ori.unwrap_or_else(|| ori.look_vec());
                if b.stage_section == StageSection::Cast {
                    if b.static_data.base_hps > 0.0 {
                        // Emit a light when using healing
                        lights.push(Light::new(pos.0 + b.offset, Rgb::new(0.1, 1.0, 0.15), 1.0));
                        for i in 0..self.scheduler.heartbeats(Duration::from_millis(1)) {
                            self.particles.push(Particle::new_directed(
                                b.static_data.beam_duration,
                                time + i as f64 / 1000.0,
                                ParticleMode::HealingBeam,
                                pos.0 + particle_ori * 0.5 + b.offset,
                                pos.0 + particle_ori * b.static_data.range + b.offset,
                            ));
                        }
                    } else {
                        let mut rng = thread_rng();
                        let (from, to) = (Vec3::<f32>::unit_z(), particle_ori);
                        let m = Mat3::<f32>::rotation_from_to_3d(from, to);
                        // Emit a light when using flames
                        lights.push(Light::new(
                            pos.0 + b.offset,
                            Rgb::new(1.0, 0.25, 0.05).map(|e| e * rng.gen_range(0.8..1.2)),
                            2.0,
                        ));
                        self.particles.resize_with(
                            self.particles.len()
                                + 2 * usize::from(
                                    self.scheduler.heartbeats(Duration::from_millis(1)),
                                ),
                            || {
                                let phi: f32 =
                                    rng.gen_range(0.0..b.static_data.max_angle.to_radians());
                                let theta: f32 = rng.gen_range(0.0..2.0 * PI);
                                let offset_z = Vec3::new(
                                    phi.sin() * theta.cos(),
                                    phi.sin() * theta.sin(),
                                    phi.cos(),
                                );
                                let random_ori = offset_z * m * Vec3::new(-1.0, -1.0, 1.0);
                                Particle::new_directed(
                                    b.static_data.beam_duration,
                                    time,
                                    ParticleMode::FlameThrower,
                                    pos.0 + random_ori * 0.5 + b.offset,
                                    pos.0 + random_ori * b.static_data.range + b.offset,
                                )
                            },
                        );
                    }
//
                }
            } else {
                let mut rng = thread_rng();
                let (from, to) = (Vec3::<f32>::unit_z(), *ori.look_dir());
                let m = Mat3::<f32>::rotation_from_to_3d(from, to);
                // Emit a light when using flames
                lights.push(Light::new(
                    pos.0,
                    Rgb::new(1.0, 0.25, 0.05).map(|e| e * rng.gen_range(0.8..1.2)),
                    2.0,
                ));
                self.particles.resize_with(
                    self.particles.len()
                        + 2 * usize::from(self.scheduler.heartbeats(Duration::from_millis(1))),
                    || {
                        let phi: f32 = rng.gen_range(0.0..beam.properties.angle.to_radians());
                        let theta: f32 = rng.gen_range(0.0..2.0 * PI);
                        let offset_z =
                            Vec3::new(phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos());
                        let random_ori = offset_z * m * Vec3::new(-1.0, -1.0, 1.0);
                        Particle::new_beam(
                            beam.properties.duration,
                            time,
                            ParticleMode::FlameThrower,
                            pos.0 + random_ori,
                            pos.0 + random_ori * range,
                        )
                    },
                );
            }
        }
    }

    #[allow(clippy::same_item_push)] // TODO: Pending review in #587
    fn maintain_block_particles(
        &mut self,
        scene_data: &SceneData,
        terrain: &Terrain<TerrainChunk>,
    ) {
        span!(
            _guard,
            "block_particles",
            "ParticleMgr::maintain_block_particles"
        );
        let dt = scene_data.state.ecs().fetch::<DeltaTime>().0;
        let time = scene_data.state.get_time();
        let player_pos = scene_data
            .state
            .read_component_copied::<Pos>(scene_data.player_entity)
            .unwrap_or_default();
        let player_chunk = player_pos.0.xy().map2(TerrainChunk::RECT_SIZE, |e, sz| {
            (e.floor() as i32).div_euclid(sz as i32)
        });

        struct BlockParticles<'a> {
            // The function to select the blocks of interest that we should emit from
            blocks: fn(&'a BlocksOfInterest) -> &'a [Vec3<i32>],
            // The range, in chunks, that the particles should be generated in from the player
            range: usize,
            // The emission rate, per block per second, of the generated particles
            rate: f32,
            // The number of seconds that each particle should live for
            lifetime: f32,
            // The visual mode of the generated particle
            mode: ParticleMode,
            // Condition that must be true
            cond: fn(&SceneData) -> bool,
        }

        let particles: &[BlockParticles] = &[
            BlockParticles {
                blocks: |boi| &boi.leaves,
                range: 4,
                rate: 0.001,
                lifetime: 30.0,
                mode: ParticleMode::Leaf,
                cond: |_| true,
            },
            BlockParticles {
                blocks: |boi| &boi.fires,
                range: 2,
                rate: 20.0,
                lifetime: 0.25,
                mode: ParticleMode::CampfireFire,
                cond: |_| true,
            },
            BlockParticles {
                blocks: |boi| &boi.fire_bowls,
                range: 2,
                rate: 20.0,
                lifetime: 0.25,
                mode: ParticleMode::FireBowl,
                cond: |_| true,
            },
            BlockParticles {
                blocks: |boi| &boi.smokers,
                range: 8,
                rate: 3.0,
                lifetime: 40.0,
                mode: ParticleMode::CampfireSmoke,
                cond: |_| true,
            },
            BlockParticles {
                blocks: |boi| &boi.reeds,
                range: 6,
                rate: 0.004,
                lifetime: 40.0,
                mode: ParticleMode::Firefly,
                cond: |sd| sd.state.get_day_period().is_dark(),
            },
            BlockParticles {
                blocks: |boi| &boi.flowers,
                range: 5,
                rate: 0.002,
                lifetime: 40.0,
                mode: ParticleMode::Firefly,
                cond: |sd| sd.state.get_day_period().is_dark(),
            },
            BlockParticles {
                blocks: |boi| &boi.beehives,
                range: 3,
                rate: 0.5,
                lifetime: 30.0,
                mode: ParticleMode::Bee,
                cond: |sd| sd.state.get_day_period().is_light(),
            },
            BlockParticles {
                blocks: |boi| &boi.snow,
                range: 4,
                rate: 0.025,
                lifetime: 15.0,
                mode: ParticleMode::Snow,
                cond: |_| true,
            },
        ];

        let mut rng = thread_rng();
        for particles in particles.iter() {
            if !(particles.cond)(scene_data) {
                continue;
            }

            for offset in Spiral2d::new().take((particles.range * 2 + 1).pow(2)) {
                let chunk_pos = player_chunk + offset;

                terrain.get(chunk_pos).map(|chunk_data| {
                    let blocks = (particles.blocks)(&chunk_data.blocks_of_interest);

                    let avg_particles = dt * blocks.len() as f32 * particles.rate;
                    let particle_count = avg_particles.trunc() as usize
                        + (rng.gen::<f32>() < avg_particles.fract()) as usize;

                    self.particles
                        .resize_with(self.particles.len() + particle_count, || {
                            let block_pos =
                                Vec3::from(chunk_pos * TerrainChunk::RECT_SIZE.map(|e| e as i32))
                                    + blocks.choose(&mut rng).copied().unwrap(); // Can't fail

                            Particle::new(
                                Duration::from_secs_f32(particles.lifetime),
                                time,
                                particles.mode,
                                block_pos.map(|e: i32| e as f32 + rng.gen::<f32>()),
                            )
                        })
                });
            }
        }
    }

    fn maintain_shockwave_particles(&mut self, scene_data: &SceneData) {
        let state = scene_data.state;
        let ecs = state.ecs();
        let time = state.get_time();

        for (_i, (_entity, pos, ori, shockwave)) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Ori>(),
            &ecs.read_storage::<Shockwave>(),
        )
            .join()
            .enumerate()
        {
            let elapsed = time - shockwave.creation.unwrap_or_default();

            let distance = shockwave.properties.speed * elapsed as f32;

            let radians = shockwave.properties.angle.to_radians();

            let ori_vec = ori.look_vec();
            let theta = ori_vec.y.atan2(ori_vec.x);
            let dtheta = radians / distance;

            let heartbeats = self.scheduler.heartbeats(Duration::from_millis(2));

            for heartbeat in 0..heartbeats {
                if shockwave.properties.requires_ground {
                    // 1 / 3 the size of terrain voxel
                    let scale = 1.0 / 3.0;

                    let scaled_speed = shockwave.properties.speed * scale;

                    let sub_tick_interpolation = scaled_speed * 1000.0 * heartbeat as f32;

                    let distance =
                        shockwave.properties.speed * (elapsed as f32 - sub_tick_interpolation);

                    let new_particle_count = distance / scale as f32;
                    self.particles.reserve(new_particle_count as usize);

                    for d in 0..((distance / scale) as i32) {
                        let arc_position = theta - radians / 2.0 + dtheta * d as f32 * scale;

                        let position = pos.0
                            + distance * Vec3::new(arc_position.cos(), arc_position.sin(), 0.0);

                        let position_snapped = ((position / scale).floor() + 0.5) * scale;

                        self.particles.push(Particle::new(
                            Duration::from_millis(250),
                            time,
                            ParticleMode::GroundShockwave,
                            position_snapped,
                        ));
                    }
                } else {
                    for d in 0..3 * distance as i32 {
                        let arc_position = theta - radians / 2.0 + dtheta * d as f32 / 3.0;

                        let position = pos.0
                            + distance * Vec3::new(arc_position.cos(), arc_position.sin(), 0.0);

                        self.particles.push(Particle::new(
                            Duration::from_secs_f32((distance + 10.0) / 50.0),
                            time,
                            ParticleMode::FireShockwave,
                            position,
                        ));
                    }
                }
            }
        }
    }

    fn upload_particles(&mut self, renderer: &mut Renderer) {
        span!(_guard, "upload_particles", "ParticleMgr::upload_particles");
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
        global: &GlobalModel,
        lod: &LodData,
    ) {
        span!(_guard, "render", "ParticleMgr::render");
        if scene_data.particles_enabled {
            let model = &self
                .model_cache
                .get(DEFAULT_MODEL_KEY)
                .expect("Expected particle model in cache");

            renderer.render_particles(model, global, &self.instances, lod);
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
        let vox = DotVoxAsset::load_expect(DEFAULT_MODEL_KEY);

        // NOTE: If we add texturing we may eventually try to share it among all
        // particles in a single atlas.
        let max_texture_size = renderer.max_texture_size();
        let max_size =
            guillotiere::Size::new(i32::from(max_texture_size), i32::from(max_texture_size));
        let mut greedy = GreedyMesh::new(max_size);

        let segment = Segment::from(&vox.read().0);
        let segment_size = segment.size();
        let mut mesh =
            Meshable::<ParticlePipeline, &mut GreedyMesh>::generate_mesh(segment, &mut greedy).0;
        // Center particle vertices around origin
        for vert in mesh.vertices_mut() {
            vert.pos[0] -= segment_size.x as f32 / 2.0;
            vert.pos[1] -= segment_size.y as f32 / 2.0;
            vert.pos[2] -= segment_size.z as f32 / 2.0;
        }

        // NOTE: Ignoring coloring / lighting for now.
        drop(greedy);

        renderer
            .create_model(&mesh)
            .expect("Failed to create particle model")
    });

    model_cache
}

/// Accumulates heartbeats to be consumed on the next tick.
struct HeartbeatScheduler {
    /// Duration = Heartbeat Frequency/Intervals
    /// f64 = Last update time
    /// u8 = number of heartbeats since last update
    /// - if it's more frequent then tick rate, it could be 1 or more.
    /// - if it's less frequent then tick rate, it could be 1 or 0.
    /// - if it's equal to the tick rate, it could be between 2 and 0, due to
    /// delta time variance etc.
    timers: HashMap<Duration, (f64, u8)>,

    last_known_time: f64,
}

impl HeartbeatScheduler {
    pub fn new() -> Self {
        HeartbeatScheduler {
            timers: HashMap::new(),
            last_known_time: 0.0,
        }
    }

    /// updates the last elapsed times and elapsed counts
    /// this should be called once, and only once per tick.
    pub fn maintain(&mut self, now: f64) {
        span!(_guard, "maintain", "HeartbeatScheduler::maintain");
        self.last_known_time = now;

        for (frequency, (last_update, heartbeats)) in self.timers.iter_mut() {
            // the number of frequency cycles that have occurred.
            let total_heartbeats = (now - *last_update) / frequency.as_secs_f64();

            // exclude partial frequency cycles
            let full_heartbeats = total_heartbeats.floor();

            *heartbeats = full_heartbeats as u8;

            // the remaining partial frequency cycle, as a decimal.
            let partial_heartbeat = total_heartbeats - full_heartbeats;

            // the remaining partial frequency cycle, as a unit of time(f64).
            let partial_heartbeat_as_time = frequency.mul_f64(partial_heartbeat).as_secs_f64();

            // now minus the left over heart beat count precision as seconds,
            // Note: we want to preserve incomplete heartbeats, and roll them
            // over into the next update.
            *last_update = now - partial_heartbeat_as_time;
        }
    }

    /// returns the number of times this duration has elapsed since the last
    /// tick:
    /// - if it's more frequent then tick rate, it could be 1 or more.
    /// - if it's less frequent then tick rate, it could be 1 or 0.
    /// - if it's equal to the tick rate, it could be between 2 and 0, due to
    /// delta time variance.
    pub fn heartbeats(&mut self, frequency: Duration) -> u8 {
        span!(_guard, "HeartbeatScheduler::heartbeats");
        let last_known_time = self.last_known_time;

        self.timers
            .entry(frequency)
            .or_insert_with(|| (last_known_time, 0))
            .1
    }

    pub fn clear(&mut self) { self.timers.clear() }
}

#[derive(Clone, Copy)]
struct Particle {
    alive_until: f64, // created_at + lifespan
    instance: ParticleInstance,
}

impl Particle {
    fn new(lifespan: Duration, time: f64, mode: ParticleMode, pos: Vec3<f32>) -> Self {
        Particle {
            alive_until: time + lifespan.as_secs_f64(),
            instance: ParticleInstance::new(time, lifespan.as_secs_f32(), mode, pos),
        }
    }

    fn new_directed(
        lifespan: Duration,
        time: f64,
        mode: ParticleMode,
        pos1: Vec3<f32>,
        pos2: Vec3<f32>,
    ) -> Self {
        Particle {
            alive_until: time + lifespan.as_secs_f64(),
            instance: ParticleInstance::new_directed(
                time,
                lifespan.as_secs_f32(),
                mode,
                pos1,
                pos2,
            ),
        }
    }
}
