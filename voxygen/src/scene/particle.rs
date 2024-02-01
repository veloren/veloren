use super::{terrain::BlocksOfInterest, FigureMgr, SceneData, Terrain};
use crate::{
    ecs::comp::Interpolated,
    mesh::{greedy::GreedyMesh, segment::generate_mesh_base_vol_particle},
    render::{
        pipelines::particle::ParticleMode, Instances, Light, Model, ParticleDrawer,
        ParticleInstance, ParticleVertex, Renderer,
    },
    scene::terrain::FireplaceType,
};
use common::{
    assets::{AssetExt, DotVoxAsset},
    comp::{
        self, aura, beam, body, buff,
        item::Reagent,
        object,
        shockwave::{self, ShockwaveDodgeable},
        Beam, Body, CharacterActivity, CharacterState, Fluid, Ori, PhysicsState, Pos, Scale,
        Shockwave, Vel,
    },
    figure::Segment,
    outcome::Outcome,
    resources::{DeltaTime, Time},
    spiral::Spiral2d,
    states::{self, utils::StageSection},
    terrain::{Block, SpriteKind, TerrainChunk, TerrainGrid},
    uid::IdMaps,
    vol::{ReadVol, RectRasterableVol, SizedVol},
};
use common_base::span;
use hashbrown::HashMap;
use rand::prelude::*;
use specs::{Join, LendJoin, WorldExt};
use std::{
    f32::consts::{PI, TAU},
    time::Duration,
};
use vek::*;

pub struct ParticleMgr {
    /// keep track of lifespans
    particles: Vec<Particle>,

    /// keep track of timings
    scheduler: HeartbeatScheduler,

    /// GPU Instance Buffer
    instances: Instances<ParticleInstance>,

    /// GPU Vertex Buffers
    model_cache: HashMap<&'static str, Model<ParticleVertex>>,
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
        let mut rng = thread_rng();

        match outcome {
            Outcome::Lightning { pos } => {
                self.particles.resize_with(self.particles.len() + 800, || {
                    Particle::new_directed(
                        Duration::from_secs_f32(rng.gen_range(0.5..1.0)),
                        time,
                        ParticleMode::Lightning,
                        *pos + Vec3::new(0.0, 0.0, rng.gen_range(0.0..600.0)),
                        *pos,
                    )
                });
            },
            Outcome::SpriteDelete { pos, sprite } => match sprite {
                SpriteKind::SeaUrchin => {
                    let pos = pos.map(|e| e as f32 + 0.5);
                    self.particles.resize_with(self.particles.len() + 10, || {
                        Particle::new_directed(
                            Duration::from_secs_f32(rng.gen_range(0.1..0.5)),
                            time,
                            ParticleMode::Steam,
                            pos + Vec3::new(0.0, 0.0, rng.gen_range(0.0..1.5)),
                            pos,
                        )
                    });
                },
                SpriteKind::EnsnaringVines => {},
                _ => {},
            },
            Outcome::Explosion {
                pos,
                power,
                radius,
                is_attack,
                reagent,
            } => {
                if *is_attack {
                    match reagent {
                        Some(Reagent::Green) => {
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
                        },
                        Some(Reagent::Red) => {
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
                        },
                        Some(Reagent::White) => {
                            self.particles.resize_with(
                                self.particles.len() + (75.0 * power.abs()) as usize,
                                || {
                                    Particle::new_directed(
                                        Duration::from_millis(500),
                                        time,
                                        ParticleMode::Ice,
                                        *pos,
                                        *pos + Vec3::<f32>::zero()
                                            .map(|_| rng.gen_range(-1.0..1.0))
                                            .normalized()
                                            * *radius,
                                    )
                                },
                            );
                        },
                        Some(Reagent::Purple) => {
                            self.particles.resize_with(
                                self.particles.len() + (75.0 * power.abs()) as usize,
                                || {
                                    Particle::new_directed(
                                        Duration::from_millis(500),
                                        time,
                                        ParticleMode::CultistFlame,
                                        *pos,
                                        *pos + Vec3::<f32>::zero()
                                            .map(|_| rng.gen_range(-1.0..1.0))
                                            .normalized()
                                            * *radius,
                                    )
                                },
                            );
                        },
                        Some(Reagent::Phoenix) => {
                            self.particles.resize_with(
                                self.particles.len() + (5.0 * power.abs()) as usize,
                                || {
                                    Particle::new_directed(
                                        Duration::from_millis(300),
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
                        },
                        _ => {},
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
                                    Some(Reagent::White) => ParticleMode::FireworkWhite,
                                    Some(Reagent::Yellow) => ParticleMode::FireworkYellow,
                                    Some(Reagent::Phoenix) => ParticleMode::FireworkYellow,
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
            Outcome::BreakBlock { pos, .. } => {
                // TODO: Use color field when particle colors are a thing
                self.particles.resize_with(self.particles.len() + 30, || {
                    Particle::new(
                        Duration::from_millis(100),
                        time,
                        ParticleMode::Shrapnel,
                        pos.map(|e| e as f32 + 0.5),
                    )
                });
            },
            Outcome::SpriteUnlocked { .. } => {},
            Outcome::FailedSpriteUnlock { pos } => {
                // TODO: Use color field when particle colors are a thing
                self.particles.resize_with(self.particles.len() + 10, || {
                    Particle::new(
                        Duration::from_millis(50),
                        time,
                        ParticleMode::Shrapnel,
                        pos.map(|e| e as f32 + 0.5),
                    )
                });
            },
            Outcome::SummonedCreature { pos, body } => match body {
                Body::BipedSmall(b) if matches!(b.species, body::biped_small::Species::Husk) => {
                    self.particles.resize_with(
                        self.particles.len()
                            + 2 * usize::from(self.scheduler.heartbeats(Duration::from_millis(1))),
                        || {
                            let start_pos = pos + Vec3::unit_z() * body.height() / 2.0;
                            let end_pos = pos
                                + Vec3::new(
                                    2.0 * rng.gen::<f32>() - 1.0,
                                    2.0 * rng.gen::<f32>() - 1.0,
                                    0.0,
                                )
                                .normalized()
                                    * (body.max_radius() + 4.0)
                                + Vec3::unit_z() * (body.height() + 2.0) * rng.gen::<f32>();

                            Particle::new_directed(
                                Duration::from_secs_f32(0.5),
                                time,
                                ParticleMode::CultistFlame,
                                start_pos,
                                end_pos,
                            )
                        },
                    );
                },
                Body::BipedSmall(b) if matches!(b.species, body::biped_small::Species::Boreal) => {
                    self.particles.resize_with(
                        self.particles.len()
                            + 2 * usize::from(self.scheduler.heartbeats(Duration::from_millis(1))),
                        || {
                            let start_pos = pos + Vec3::unit_z() * body.height() / 2.0;
                            let end_pos = pos
                                + Vec3::new(
                                    2.0 * rng.gen::<f32>() - 1.0,
                                    2.0 * rng.gen::<f32>() - 1.0,
                                    0.0,
                                )
                                .normalized()
                                    * (body.max_radius() + 4.0)
                                + Vec3::unit_z() * (body.height() + 20.0) * rng.gen::<f32>();

                            Particle::new_directed(
                                Duration::from_secs_f32(0.5),
                                time,
                                ParticleMode::GigaSnow,
                                start_pos,
                                end_pos,
                            )
                        },
                    );
                },
                _ => {},
            },
            Outcome::ProjectileHit { pos, target, .. } => {
                if target.is_some() {
                    let ecs = scene_data.state.ecs();
                    if target
                        .and_then(|target| ecs.read_resource::<IdMaps>().uid_entity(target))
                        .and_then(|entity| {
                            ecs.read_storage::<Body>()
                                .get(entity)
                                .map(|body| body.bleeds())
                        })
                        .unwrap_or(false)
                    {
                        self.particles.resize_with(self.particles.len() + 30, || {
                            Particle::new(
                                Duration::from_millis(250),
                                time,
                                ParticleMode::Blood,
                                *pos,
                            )
                        })
                    };
                };
            },
            Outcome::Block { pos, parry, .. } => {
                if *parry {
                    self.particles.resize_with(self.particles.len() + 10, || {
                        Particle::new(
                            Duration::from_millis(200),
                            time,
                            ParticleMode::GunPowderSpark,
                            *pos + Vec3::unit_z(),
                        )
                    });
                }
            },
            Outcome::GroundSlam { pos, .. } => {
                self.particles.resize_with(self.particles.len() + 100, || {
                    Particle::new(
                        Duration::from_millis(1000),
                        time,
                        ParticleMode::BigShrapnel,
                        *pos,
                    )
                });
            },
            Outcome::FlashFreeze { pos, .. } => {
                self.particles.resize_with(
                    self.particles.len()
                        + 2 * usize::from(self.scheduler.heartbeats(Duration::from_millis(1))),
                    || {
                        let start_pos = pos + Vec3::unit_z() - 1.0;
                        let end_pos = pos
                            + Vec3::new(
                                4.0 * rng.gen::<f32>() - 1.0,
                                4.0 * rng.gen::<f32>() - 1.0,
                                0.0,
                            )
                            .normalized()
                                * 1.5
                            + Vec3::unit_z()
                            + 5.0 * rng.gen::<f32>();

                        Particle::new_directed(
                            Duration::from_secs_f32(0.5),
                            time,
                            ParticleMode::GigaSnow,
                            start_pos,
                            end_pos,
                        )
                    },
                );
            },
            Outcome::CyclopsCharge { pos } => {
                self.particles.push(Particle::new_directed(
                    Duration::from_secs_f32(rng.gen_range(0.1..0.2)),
                    time,
                    ParticleMode::CyclopsCharge,
                    *pos + Vec3::new(0.0, 0.0, 5.3),
                    *pos + Vec3::new(0.0, 0.0, 5.6 + 0.5 * rng.gen_range(0.0..0.2)),
                ));
            },
            Outcome::FlamethrowerCharge { pos } | Outcome::FuseCharge { pos } => {
                self.particles.push(Particle::new_directed(
                    Duration::from_secs_f32(rng.gen_range(0.1..0.2)),
                    time,
                    ParticleMode::CampfireFire,
                    *pos + Vec3::new(0.0, 0.0, 1.2),
                    *pos + Vec3::new(0.0, 0.0, 1.5 + 0.5 * rng.gen_range(0.0..0.2)),
                ));
            },
            Outcome::Death { pos, .. } => {
                self.particles.resize_with(self.particles.len() + 40, || {
                    Particle::new(
                        Duration::from_millis(400 + rng.gen_range(0..100)),
                        time,
                        ParticleMode::Death,
                        *pos + Vec3::unit_z()
                            + Vec3::<f32>::zero()
                                .map(|_| rng.gen_range(-0.1..0.1))
                                .normalized(),
                    )
                });
            },
            Outcome::GroundDig { pos, .. } => {
                self.particles.resize_with(self.particles.len() + 12, || {
                    Particle::new(
                        Duration::from_millis(200),
                        time,
                        ParticleMode::BigShrapnel,
                        *pos,
                    )
                });
            },
            Outcome::TeleportedByPortal { pos, .. } => {
                self.particles.resize_with(self.particles.len() + 80, || {
                    Particle::new_directed(
                        Duration::from_millis(500),
                        time,
                        ParticleMode::CultistFlame,
                        *pos,
                        pos + Vec3::unit_z()
                            + Vec3::zero()
                                .map(|_: f32| rng.gen_range(-0.1..0.1))
                                .normalized()
                                * 2.0,
                    )
                });
            },
            Outcome::ClayGolemDash { pos, .. } => {
                self.particles.resize_with(self.particles.len() + 100, || {
                    Particle::new(
                        Duration::from_millis(1000),
                        time,
                        ParticleMode::ClayShrapnel,
                        *pos,
                    )
                });
            },
            Outcome::ProjectileShot { .. }
            | Outcome::Beam { .. }
            | Outcome::ExpChange { .. }
            | Outcome::SkillPointGain { .. }
            | Outcome::ComboChange { .. }
            | Outcome::HealthChange { .. }
            | Outcome::PoiseChange { .. }
            | Outcome::Utterance { .. }
            | Outcome::IceSpikes { .. }
            | Outcome::IceCrack { .. }
            | Outcome::Glider { .. }
            | Outcome::Whoosh { .. }
            | Outcome::Swoosh { .. }
            | Outcome::Slash { .. }
            | Outcome::Steam { .. }
            | Outcome::FireShockwave { .. }
            | Outcome::PortalActivated { .. }
            | Outcome::FromTheAshes { .. }
            | Outcome::LaserBeam { .. } => {},
        }
    }

    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        terrain: &Terrain<TerrainChunk>,
        figure_mgr: &FigureMgr,
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
            self.maintain_char_state_particles(scene_data);
            self.maintain_beam_particles(scene_data, lights);
            self.maintain_block_particles(scene_data, terrain, figure_mgr);
            self.maintain_shockwave_particles(scene_data);
            self.maintain_aura_particles(scene_data);
            self.maintain_buff_particles(scene_data);

            self.upload_particles(renderer);
        } else {
            // remove all particle lifespans
            if !self.particles.is_empty() {
                self.particles.clear();
                self.upload_particles(renderer);
            }

            // remove all timings
            self.scheduler.clear();
        }
    }

    fn maintain_body_particles(&mut self, scene_data: &SceneData) {
        span!(
            _guard,
            "body_particles",
            "ParticleMgr::maintain_body_particles"
        );
        let ecs = scene_data.state.ecs();
        for (body, interpolated, vel) in (
            &ecs.read_storage::<Body>(),
            &ecs.read_storage::<Interpolated>(),
            ecs.read_storage::<Vel>().maybe(),
        )
            .join()
        {
            match body {
                Body::Object(object::Body::CampfireLit) => {
                    self.maintain_campfirelit_particles(scene_data, interpolated.pos, vel)
                },
                Body::Object(object::Body::BarrelOrgan) => {
                    self.maintain_barrel_organ_particles(scene_data, interpolated.pos, vel)
                },
                Body::Object(object::Body::BoltFire) => {
                    self.maintain_boltfire_particles(scene_data, interpolated.pos, vel)
                },
                Body::Object(object::Body::BoltFireBig) => {
                    self.maintain_boltfirebig_particles(scene_data, interpolated.pos, vel)
                },
                Body::Object(object::Body::FireRainDrop) => {
                    self.maintain_fireraindrop_particles(scene_data, interpolated.pos, vel)
                },
                Body::Object(object::Body::BoltNature) => {
                    self.maintain_boltnature_particles(scene_data, interpolated.pos, vel)
                },
                Body::Object(object::Body::Tornado) => {
                    self.maintain_tornado_particles(scene_data, interpolated.pos)
                },
                Body::Object(object::Body::FieryTornado) => {
                    self.maintain_fiery_tornado_particles(scene_data, interpolated.pos)
                },
                Body::Object(object::Body::Mine) => {
                    self.maintain_mine_particles(scene_data, interpolated.pos)
                },
                Body::Object(
                    object::Body::Bomb
                    | object::Body::FireworkBlue
                    | object::Body::FireworkGreen
                    | object::Body::FireworkPurple
                    | object::Body::FireworkRed
                    | object::Body::FireworkWhite
                    | object::Body::FireworkYellow,
                ) => self.maintain_bomb_particles(scene_data, interpolated.pos, vel),
                Body::Object(object::Body::PortalActive) => {
                    self.maintain_active_portal_particles(scene_data, interpolated.pos)
                },
                Body::Object(object::Body::Portal) => {
                    self.maintain_portal_particles(scene_data, interpolated.pos)
                },
                _ => {},
            }
        }
    }

    fn maintain_campfirelit_particles(
        &mut self,
        scene_data: &SceneData,
        pos: Vec3<f32>,
        vel: Option<&Vel>,
    ) {
        span!(
            _guard,
            "campfirelit_particles",
            "ParticleMgr::maintain_campfirelit_particles"
        );
        let time = scene_data.state.get_time();
        let dt = scene_data.state.get_delta_time();
        let mut rng = thread_rng();

        for _ in 0..self.scheduler.heartbeats(Duration::from_millis(50)) {
            self.particles.push(Particle::new(
                Duration::from_millis(250),
                time,
                ParticleMode::CampfireFire,
                pos,
            ));

            self.particles.push(Particle::new(
                Duration::from_secs(10),
                time,
                ParticleMode::CampfireSmoke,
                pos.map(|e| e + thread_rng().gen_range(-0.25..0.25))
                    + vel.map_or(Vec3::zero(), |v| -v.0 * dt * rng.gen::<f32>()),
            ));
        }
    }

    fn maintain_barrel_organ_particles(
        &mut self,
        scene_data: &SceneData,
        pos: Vec3<f32>,
        vel: Option<&Vel>,
    ) {
        span!(
            _guard,
            "barrel_organ_particles",
            "ParticleMgr::maintain_barrel_organ_particles"
        );
        let time = scene_data.state.get_time();
        let dt = scene_data.state.get_delta_time();
        let mut rng = thread_rng();

        for _ in 0..self.scheduler.heartbeats(Duration::from_millis(20)) {
            self.particles.push(Particle::new(
                Duration::from_millis(250),
                time,
                ParticleMode::BarrelOrgan,
                pos,
            ));

            self.particles.push(Particle::new(
                Duration::from_secs(10),
                time,
                ParticleMode::BarrelOrgan,
                pos.map(|e| e + thread_rng().gen_range(-0.25..0.25))
                    + vel.map_or(Vec3::zero(), |v| -v.0 * dt * rng.gen::<f32>()),
            ));
        }
    }

    fn maintain_boltfire_particles(
        &mut self,
        scene_data: &SceneData,
        pos: Vec3<f32>,
        vel: Option<&Vel>,
    ) {
        span!(
            _guard,
            "boltfire_particles",
            "ParticleMgr::maintain_boltfire_particles"
        );
        let time = scene_data.state.get_time();
        let dt = scene_data.state.get_delta_time();
        let mut rng = thread_rng();

        for _ in 0..self.scheduler.heartbeats(Duration::from_millis(4)) {
            self.particles.push(Particle::new(
                Duration::from_millis(500),
                time,
                ParticleMode::CampfireFire,
                pos,
            ));
            self.particles.push(Particle::new(
                Duration::from_secs(1),
                time,
                ParticleMode::CampfireSmoke,
                pos.map(|e| e + rng.gen_range(-0.25..0.25))
                    + vel.map_or(Vec3::zero(), |v| -v.0 * dt * rng.gen::<f32>()),
            ));
        }
    }

    fn maintain_boltfirebig_particles(
        &mut self,
        scene_data: &SceneData,
        pos: Vec3<f32>,
        vel: Option<&Vel>,
    ) {
        span!(
            _guard,
            "boltfirebig_particles",
            "ParticleMgr::maintain_boltfirebig_particles"
        );
        let time = scene_data.state.get_time();
        let dt = scene_data.state.get_delta_time();
        let mut rng = thread_rng();

        // fire
        self.particles.resize_with(
            self.particles.len() + usize::from(self.scheduler.heartbeats(Duration::from_millis(2))),
            || {
                Particle::new(
                    Duration::from_millis(500),
                    time,
                    ParticleMode::CampfireFire,
                    pos.map(|e| e + rng.gen_range(-0.25..0.25))
                        + vel.map_or(Vec3::zero(), |v| -v.0 * dt * rng.gen::<f32>()),
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
                    pos.map(|e| e + rng.gen_range(-0.25..0.25))
                        + vel.map_or(Vec3::zero(), |v| -v.0 * dt),
                )
            },
        );
    }

    fn maintain_fireraindrop_particles(
        &mut self,
        scene_data: &SceneData,
        pos: Vec3<f32>,
        vel: Option<&Vel>,
    ) {
        span!(
            _guard,
            "fireraindrop_particles",
            "ParticleMgr::maintain_fireraindrop_particles"
        );
        let time = scene_data.state.get_time();
        let dt = scene_data.state.get_delta_time();
        let mut rng = thread_rng();

        // trace
        self.particles.resize_with(
            self.particles.len()
                + usize::from(self.scheduler.heartbeats(Duration::from_millis(100))),
            || {
                Particle::new(
                    Duration::from_millis(300),
                    time,
                    ParticleMode::FieryDropletTrace,
                    pos.map(|e| e + rng.gen_range(-0.25..0.25))
                        + Vec3::new(0.0, 0.0, 0.5)
                        + vel.map_or(Vec3::zero(), |v| -v.0 * dt * rng.gen::<f32>()),
                )
            },
        );
    }

    fn maintain_boltnature_particles(
        &mut self,
        scene_data: &SceneData,
        pos: Vec3<f32>,
        vel: Option<&Vel>,
    ) {
        let time = scene_data.state.get_time();
        let dt = scene_data.state.get_delta_time();
        let mut rng = thread_rng();

        // nature
        self.particles.resize_with(
            self.particles.len() + usize::from(self.scheduler.heartbeats(Duration::from_millis(2))),
            || {
                Particle::new(
                    Duration::from_millis(500),
                    time,
                    ParticleMode::CampfireSmoke,
                    pos.map(|e| e + rng.gen_range(-0.25..0.25))
                        + vel.map_or(Vec3::zero(), |v| -v.0 * dt * rng.gen::<f32>()),
                )
            },
        );
    }

    fn maintain_tornado_particles(&mut self, scene_data: &SceneData, pos: Vec3<f32>) {
        let time = scene_data.state.get_time();
        let mut rng = thread_rng();

        // air particles
        self.particles.resize_with(
            self.particles.len() + usize::from(self.scheduler.heartbeats(Duration::from_millis(5))),
            || {
                Particle::new(
                    Duration::from_millis(1000),
                    time,
                    ParticleMode::Tornado,
                    pos.map(|e| e + rng.gen_range(-0.25..0.25)),
                )
            },
        );
    }

    fn maintain_fiery_tornado_particles(&mut self, scene_data: &SceneData, pos: Vec3<f32>) {
        let time = scene_data.state.get_time();
        let mut rng = thread_rng();

        // air particles
        self.particles.resize_with(
            self.particles.len() + usize::from(self.scheduler.heartbeats(Duration::from_millis(5))),
            || {
                Particle::new(
                    Duration::from_millis(1000),
                    time,
                    ParticleMode::FieryTornado,
                    pos.map(|e| e + rng.gen_range(-0.25..0.25)),
                )
            },
        );
    }

    fn maintain_bomb_particles(
        &mut self,
        scene_data: &SceneData,
        pos: Vec3<f32>,
        vel: Option<&Vel>,
    ) {
        span!(
            _guard,
            "bomb_particles",
            "ParticleMgr::maintain_bomb_particles"
        );
        let time = scene_data.state.get_time();
        let dt = scene_data.state.get_delta_time();
        let mut rng = thread_rng();

        for _ in 0..self.scheduler.heartbeats(Duration::from_millis(10)) {
            // sparks
            self.particles.push(Particle::new(
                Duration::from_millis(1500),
                time,
                ParticleMode::GunPowderSpark,
                pos,
            ));

            // smoke
            self.particles.push(Particle::new(
                Duration::from_secs(2),
                time,
                ParticleMode::CampfireSmoke,
                pos + vel.map_or(Vec3::zero(), |v| -v.0 * dt * rng.gen::<f32>()),
            ));
        }
    }

    fn maintain_active_portal_particles(&mut self, scene_data: &SceneData, pos: Vec3<f32>) {
        span!(
            _guard,
            "active_portal_particles",
            "ParticleMgr::maintain_active_portal_particles"
        );

        let time = scene_data.state.get_time();
        let mut rng = thread_rng();

        for _ in 0..self.scheduler.heartbeats(Duration::from_millis(5)) {
            let outer_pos =
                pos + (Vec2::unit_x().rotated_z(rng.gen_range((0.)..PI * 2.)) * 2.7).with_z(0.);

            self.particles.push(Particle::new_directed(
                Duration::from_secs_f32(rng.gen_range(0.4..0.8)),
                time,
                ParticleMode::CultistFlame,
                outer_pos,
                outer_pos + Vec3::unit_z() * rng.gen_range(5.0..7.0),
            ));
        }
    }

    fn maintain_portal_particles(&mut self, scene_data: &SceneData, pos: Vec3<f32>) {
        span!(
            _guard,
            "portal_particles",
            "ParticleMgr::maintain_portal_particles"
        );

        let time = scene_data.state.get_time();
        let mut rng = thread_rng();

        for _ in 0..self.scheduler.heartbeats(Duration::from_millis(150)) {
            let outer_pos = pos
                + (Vec2::unit_x().rotated_z(rng.gen_range((0.)..PI * 2.))
                    * rng.gen_range(1.0..2.9))
                .with_z(0.);

            self.particles.push(Particle::new_directed(
                Duration::from_secs_f32(rng.gen_range(0.5..3.0)),
                time,
                ParticleMode::CultistFlame,
                outer_pos,
                outer_pos + Vec3::unit_z() * rng.gen_range(3.0..4.0),
            ));
        }
    }

    fn maintain_mine_particles(&mut self, scene_data: &SceneData, pos: Vec3<f32>) {
        span!(
            _guard,
            "mine_particles",
            "ParticleMgr::maintain_mine_particles"
        );
        let time = scene_data.state.get_time();

        for _ in 0..self.scheduler.heartbeats(Duration::from_millis(1)) {
            // sparks
            self.particles.push(Particle::new(
                Duration::from_millis(25),
                time,
                ParticleMode::GunPowderSpark,
                pos,
            ));
        }
    }

    fn maintain_char_state_particles(&mut self, scene_data: &SceneData) {
        span!(
            _guard,
            "char_state_particles",
            "ParticleMgr::maintain_char_state_particles"
        );
        let state = scene_data.state;
        let ecs = state.ecs();
        let time = state.get_time();
        let dt = scene_data.state.get_delta_time();
        let mut rng = thread_rng();

        for (entity, interpolated, vel, character_state, body, ori, character_activity, physics) in
            (
                &ecs.entities(),
                &ecs.read_storage::<Interpolated>(),
                ecs.read_storage::<Vel>().maybe(),
                &ecs.read_storage::<CharacterState>(),
                &ecs.read_storage::<Body>(),
                &ecs.read_storage::<Ori>(),
                &ecs.read_storage::<CharacterActivity>(),
                &ecs.read_storage::<PhysicsState>(),
            )
                .join()
        {
            match character_state {
                CharacterState::Boost(_) => {
                    self.particles.resize_with(
                        self.particles.len()
                            + usize::from(self.scheduler.heartbeats(Duration::from_millis(10))),
                        || {
                            Particle::new(
                                Duration::from_secs(15),
                                time,
                                ParticleMode::CampfireSmoke,
                                interpolated.pos
                                    + vel.map_or(Vec3::zero(), |v| -v.0 * dt * rng.gen::<f32>()),
                            )
                        },
                    );
                },
                CharacterState::RapidMelee(c) => {
                    if let Some(specifier) = c.static_data.frontend_specifier {
                        match specifier {
                            states::rapid_melee::FrontendSpecifier::CultistVortex => {
                                if matches!(c.stage_section, StageSection::Action) {
                                    let range = c.static_data.melee_constructor.range;
                                    // Particles for vortex
                                    let heartbeats =
                                        self.scheduler.heartbeats(Duration::from_millis(3));
                                    self.particles.resize_with(
                                        self.particles.len()
                                            + range.powi(2) as usize * usize::from(heartbeats)
                                                / 150,
                                        || {
                                            let rand_dist =
                                                range * (1.0 - rng.gen::<f32>().powi(10));
                                            let init_pos = Vec3::new(
                                                2.0 * rng.gen::<f32>() - 1.0,
                                                2.0 * rng.gen::<f32>() - 1.0,
                                                0.0,
                                            )
                                            .normalized()
                                                * rand_dist
                                                + interpolated.pos
                                                + Vec3::unit_z() * 0.05;
                                            Particle::new_directed(
                                                Duration::from_millis(900),
                                                time,
                                                ParticleMode::CultistFlame,
                                                init_pos,
                                                interpolated.pos,
                                            )
                                        },
                                    );
                                    // Particles for lifesteal effect
                                    for (_entity_b, interpolated_b, body_b, _health_b) in (
                                        &ecs.entities(),
                                        &ecs.read_storage::<Interpolated>(),
                                        &ecs.read_storage::<Body>(),
                                        &ecs.read_storage::<comp::Health>(),
                                    )
                                        .join()
                                        .filter(|(e, _, _, h)| !h.is_dead && entity != *e)
                                    {
                                        if interpolated.pos.distance_squared(interpolated_b.pos)
                                            < range.powi(2)
                                        {
                                            let heartbeats = self
                                                .scheduler
                                                .heartbeats(Duration::from_millis(20));
                                            self.particles.resize_with(
                                                self.particles.len()
                                                    + range.powi(2) as usize
                                                        * usize::from(heartbeats)
                                                        / 150,
                                                || {
                                                    let start_pos = interpolated_b.pos
                                                        + Vec3::unit_z() * body_b.height() * 0.5
                                                        + Vec3::<f32>::zero()
                                                            .map(|_| rng.gen_range(-1.0..1.0))
                                                            .normalized()
                                                            * 1.0;
                                                    Particle::new_directed(
                                                        Duration::from_millis(900),
                                                        time,
                                                        ParticleMode::CultistFlame,
                                                        start_pos,
                                                        interpolated.pos
                                                            + Vec3::unit_z() * body.height() * 0.5,
                                                    )
                                                },
                                            );
                                        }
                                    }
                                }
                            },
                            states::rapid_melee::FrontendSpecifier::Whirlwind => {
                                if matches!(c.stage_section, StageSection::Action) {
                                    let time = scene_data.state.get_time();
                                    let mut rng = thread_rng();
                                    self.particles.resize_with(
                                        self.particles.len()
                                            + 3
                                            + usize::from(
                                                self.scheduler.heartbeats(Duration::from_millis(5)),
                                            ),
                                        || {
                                            Particle::new(
                                                Duration::from_millis(1000),
                                                time,
                                                ParticleMode::Whirlwind,
                                                interpolated
                                                    .pos
                                                    .map(|e| e + rng.gen_range(-0.25..0.25)),
                                            )
                                        },
                                    );
                                }
                            },
                        }
                    }
                },
                CharacterState::RepeaterRanged(repeater) => {
                    if let Some(specifier) = repeater.static_data.specifier {
                        match specifier {
                            states::repeater_ranged::FrontendSpecifier::FireRain => {
                                // base, dark clouds
                                self.particles.resize_with(
                                    self.particles.len()
                                        + 2 * usize::from(
                                            self.scheduler.heartbeats(Duration::from_millis(25)),
                                        ),
                                    || {
                                        let rand_pos = {
                                            let theta = rng.gen::<f32>() * TAU;
                                            let radius = repeater
                                                .static_data
                                                .properties_of_aoe
                                                .map(|aoe| aoe.radius)
                                                .unwrap_or_default()
                                                * rng.gen::<f32>().sqrt();
                                            let x = radius * theta.sin();
                                            let y = radius * theta.cos();
                                            Vec2::new(x, y) + interpolated.pos.xy()
                                        };
                                        let pos1 = rand_pos.with_z(
                                            repeater
                                                .static_data
                                                .properties_of_aoe
                                                .map(|aoe| aoe.height)
                                                .unwrap_or_default()
                                                + interpolated.pos.z
                                                + 2.0 * rng.gen::<f32>(),
                                        );
                                        Particle::new_directed(
                                            Duration::from_secs_f32(3.0),
                                            time,
                                            ParticleMode::PhoenixCloud,
                                            pos1,
                                            pos1 + Vec3::new(7.09, 4.09, 18.09),
                                        )
                                    },
                                );
                                self.particles.resize_with(
                                    self.particles.len()
                                        + 2 * usize::from(
                                            self.scheduler.heartbeats(Duration::from_millis(25)),
                                        ),
                                    || {
                                        let rand_pos = {
                                            let theta = rng.gen::<f32>() * TAU;
                                            let radius = repeater
                                                .static_data
                                                .properties_of_aoe
                                                .map(|aoe| aoe.radius)
                                                .unwrap_or_default()
                                                * rng.gen::<f32>().sqrt();
                                            let x = radius * theta.sin();
                                            let y = radius * theta.cos();
                                            Vec2::new(x, y) + interpolated.pos.xy()
                                        };
                                        let pos1 = rand_pos.with_z(
                                            repeater
                                                .static_data
                                                .properties_of_aoe
                                                .map(|aoe| aoe.height)
                                                .unwrap_or_default()
                                                + interpolated.pos.z
                                                + 1.5 * rng.gen::<f32>(),
                                        );
                                        Particle::new_directed(
                                            Duration::from_secs_f32(2.5),
                                            time,
                                            ParticleMode::PhoenixCloud,
                                            pos1,
                                            pos1 + Vec3::new(10.025, 4.025, 17.025),
                                        )
                                    },
                                );
                            },
                        }
                    }
                },
                CharacterState::Blink(c) => {
                    self.particles.resize_with(
                        self.particles.len()
                            + usize::from(self.scheduler.heartbeats(Duration::from_millis(10))),
                        || {
                            let center_pos =
                                interpolated.pos + Vec3::unit_z() * body.height() / 2.0;
                            let outer_pos = interpolated.pos
                                + Vec3::new(
                                    2.0 * rng.gen::<f32>() - 1.0,
                                    2.0 * rng.gen::<f32>() - 1.0,
                                    0.0,
                                )
                                .normalized()
                                    * (body.max_radius() + 2.0)
                                + Vec3::unit_z() * body.height() * rng.gen::<f32>();

                            let (start_pos, end_pos) =
                                if matches!(c.stage_section, StageSection::Buildup) {
                                    (outer_pos, center_pos)
                                } else {
                                    (center_pos, outer_pos)
                                };

                            Particle::new_directed(
                                Duration::from_secs_f32(0.5),
                                time,
                                ParticleMode::CultistFlame,
                                start_pos,
                                end_pos,
                            )
                        },
                    );
                },
                CharacterState::SelfBuff(c) => {
                    if let Some(specifier) = c.static_data.specifier {
                        match specifier {
                            states::self_buff::FrontendSpecifier::FromTheAshes => {
                                if matches!(c.stage_section, StageSection::Action) {
                                    let pos = interpolated.pos;
                                    self.particles.resize_with(
                                        self.particles.len()
                                            + 2 * usize::from(
                                                self.scheduler.heartbeats(Duration::from_millis(1)),
                                            ),
                                        || {
                                            let start_pos = pos + Vec3::unit_z() - 1.0;
                                            let end_pos = pos
                                                + Vec3::new(
                                                    4.0 * rng.gen::<f32>() - 1.0,
                                                    4.0 * rng.gen::<f32>() - 1.0,
                                                    0.0,
                                                )
                                                .normalized()
                                                    * 1.5
                                                + Vec3::unit_z()
                                                + 5.0 * rng.gen::<f32>();

                                            Particle::new_directed(
                                                Duration::from_secs_f32(0.5),
                                                time,
                                                ParticleMode::FieryBurst,
                                                start_pos,
                                                end_pos,
                                            )
                                        },
                                    );
                                    self.particles.resize_with(
                                        self.particles.len()
                                            + usize::from(
                                                self.scheduler
                                                    .heartbeats(Duration::from_millis(10)),
                                            ),
                                        || {
                                            Particle::new(
                                                Duration::from_millis(650),
                                                time,
                                                ParticleMode::FieryBurstVortex,
                                                pos.map(|e| e + rng.gen_range(-0.25..0.25))
                                                    + Vec3::new(0.0, 0.0, 1.0),
                                            )
                                        },
                                    );
                                    self.particles.resize_with(
                                        self.particles.len()
                                            + usize::from(
                                                self.scheduler
                                                    .heartbeats(Duration::from_millis(40)),
                                            ),
                                        || {
                                            Particle::new(
                                                Duration::from_millis(1000),
                                                time,
                                                ParticleMode::FieryBurstSparks,
                                                pos.map(|e| e + rng.gen_range(-0.25..0.25)),
                                            )
                                        },
                                    );
                                    self.particles.resize_with(
                                        self.particles.len()
                                            + usize::from(
                                                self.scheduler
                                                    .heartbeats(Duration::from_millis(14)),
                                            ),
                                        || {
                                            let pos1 = pos.map(|e| e + rng.gen_range(-0.25..0.25));
                                            Particle::new_directed(
                                                Duration::from_millis(1000),
                                                time,
                                                ParticleMode::FieryBurstAsh,
                                                pos1,
                                                Vec3::new(
                                                    4.5,    // radius of rand spawn
                                                    20.4,   // integer part - radius of the curve part, fractional part - relative time of setting particle on fire
                                                    8.58)   // height of the flight
                                                    + pos1,
                                            )
                                        },
                                    );
                                }
                            },
                        }
                    }
                    use buff::BuffKind;
                    if let BuffKind::Frenzied = c.static_data.buff_kind {
                        if matches!(c.stage_section, StageSection::Action) {
                            self.particles.resize_with(
                                self.particles.len()
                                    + usize::from(
                                        self.scheduler.heartbeats(Duration::from_millis(5)),
                                    ),
                                || {
                                    let start_pos = interpolated.pos
                                        + Vec3::new(
                                            body.max_radius(),
                                            body.max_radius(),
                                            body.height() / 2.0,
                                        )
                                        .map(|d| d * rng.gen_range(-1.0..1.0));
                                    let end_pos =
                                        interpolated.pos + (start_pos - interpolated.pos) * 6.0;
                                    Particle::new_directed(
                                        Duration::from_secs(1),
                                        time,
                                        ParticleMode::Enraged,
                                        start_pos,
                                        end_pos,
                                    )
                                },
                            );
                        }
                    }
                },
                CharacterState::BasicBeam(beam) => {
                    let ori = *ori;
                    let _look_dir = *character_activity.look_dir.unwrap_or(ori.look_dir());
                    let dir = ori.look_dir(); //.with_z(look_dir.z);
                    let specifier = beam.static_data.specifier;
                    if specifier == beam::FrontendSpecifier::PhoenixLaser
                        && matches!(beam.stage_section, StageSection::Buildup)
                    {
                        self.particles.resize_with(
                            self.particles.len()
                                + 2 * usize::from(
                                    self.scheduler.heartbeats(Duration::from_millis(2)),
                                ),
                            || {
                                let mut left_right_alignment =
                                    dir.cross(Vec3::new(0.0, 0.0, 1.0)).normalized();
                                if rng.gen_bool(0.5) {
                                    left_right_alignment *= -1.0;
                                }
                                let start = interpolated.pos
                                    + left_right_alignment * 4.0
                                    + dir.normalized() * 6.0;
                                let lifespan = Duration::from_secs_f32(0.5);
                                Particle::new_directed(
                                    lifespan,
                                    time,
                                    ParticleMode::PhoenixBuildUpAim,
                                    start,
                                    interpolated.pos
                                        + dir.normalized() * 3.0
                                        + left_right_alignment * 0.4
                                        + vel
                                            .map_or(Vec3::zero(), |v| v.0 * lifespan.as_secs_f32()),
                                )
                            },
                        );
                    }
                },
                CharacterState::Glide(_) => {
                    if let Some(Fluid::Air {
                        vel: air_vel,
                        elevation: _,
                    }) = physics.in_fluid
                    {
                        // Empirical observation is that air_vel is somewhere
                        // between 0.0 and 13.0, but we are extending to be sure
                        const MAX_AIR_VEL: f32 = 15.0;
                        const MIN_AIR_VEL: f32 = -2.0;

                        let minmax_norm = |val, min, max| (val - min) / (max - min);

                        let wind_speed = air_vel.0.magnitude();

                        // Less means more frequent particles
                        let heartbeat = 200
                            - Lerp::lerp(
                                50u64,
                                150,
                                minmax_norm(wind_speed, MIN_AIR_VEL, MAX_AIR_VEL),
                            );

                        let new_count = self.particles.len()
                            + usize::from(
                                self.scheduler.heartbeats(Duration::from_millis(heartbeat)),
                            );

                        // More number, longer particles
                        let duration = Lerp::lerp(
                            0u64,
                            1000,
                            minmax_norm(wind_speed, MIN_AIR_VEL, MAX_AIR_VEL),
                        );
                        let duration = Duration::from_millis(duration);

                        self.particles.resize_with(new_count, || {
                            let start_pos = interpolated.pos
                                + Vec3::new(
                                    body.max_radius(),
                                    body.max_radius(),
                                    body.height() / 2.0,
                                )
                                .map(|d| d * rng.gen_range(-10.0..10.0));

                            Particle::new_directed(
                                duration,
                                time,
                                ParticleMode::Airflow,
                                start_pos,
                                start_pos + air_vel.0,
                            )
                        });
                    }
                },
                _ => {},
            }
        }
    }

    fn maintain_beam_particles(&mut self, scene_data: &SceneData, lights: &mut Vec<Light>) {
        let state = scene_data.state;
        let ecs = state.ecs();
        let time = state.get_time();

        for (beam, ori) in (&ecs.read_storage::<Beam>(), &ecs.read_storage::<Ori>()).join() {
            // TODO: Handle this less hackily. Done this way as beam segments are created
            // every server tick, which is approximately 33 ms. Heartbeat scheduler used to
            // account for clients with less than 30 fps because they start the creation
            // time when the segments are received and could receive 2 at once
            // TODO: Above limitation no longer exists. Evaluate changing below behavior
            // later to allow for better beam particles.
            let beam_tick_count = 33.max(self.scheduler.heartbeats(Duration::from_millis(1)));
            let angle = (beam.end_radius / beam.range).atan();
            let beam_dir = (beam.bezier.ctrl - beam.bezier.start)
                .try_normalized()
                .unwrap_or(*ori.look_dir());
            match beam.specifier {
                beam::FrontendSpecifier::Flamethrower => {
                    let mut rng = thread_rng();
                    let (from, to) = (Vec3::<f32>::unit_z(), beam_dir);
                    let m = Mat3::<f32>::rotation_from_to_3d(from, to);
                    // Emit a light when using flames
                    if scene_data.flashing_lights_enabled {
                        lights.push(Light::new(
                            beam.bezier.start,
                            Rgb::new(1.0, 0.25, 0.05).map(|e| e * rng.gen_range(0.8..1.2)),
                            2.0,
                        ));
                    }
                    self.particles.resize_with(
                        self.particles.len() + usize::from(beam_tick_count) / 2,
                        || {
                            let phi: f32 = rng.gen_range(0.0..angle);
                            let theta: f32 = rng.gen_range(0.0..2.0 * PI);
                            let offset_z = Vec3::new(
                                phi.sin() * theta.cos(),
                                phi.sin() * theta.sin(),
                                phi.cos(),
                            );
                            let random_ori = offset_z * m * Vec3::new(-1.0, -1.0, 1.0);
                            Particle::new_directed(
                                Duration::from_secs_f64(beam.duration.0),
                                time,
                                ParticleMode::FlameThrower,
                                beam.bezier.start,
                                beam.bezier.start + random_ori * beam.range,
                            )
                        },
                    );
                },
                beam::FrontendSpecifier::Cultist => {
                    let mut rng = thread_rng();
                    let (from, to) = (Vec3::<f32>::unit_z(), beam_dir);
                    let m = Mat3::<f32>::rotation_from_to_3d(from, to);
                    // Emit a light when using flames
                    if scene_data.flashing_lights_enabled {
                        lights.push(Light::new(
                            beam.bezier.start,
                            Rgb::new(1.0, 0.0, 1.0).map(|e| e * rng.gen_range(0.5..1.0)),
                            2.0,
                        ));
                    }
                    self.particles.resize_with(
                        self.particles.len() + usize::from(beam_tick_count) / 2,
                        || {
                            let phi: f32 = rng.gen_range(0.0..angle);
                            let theta: f32 = rng.gen_range(0.0..2.0 * PI);
                            let offset_z = Vec3::new(
                                phi.sin() * theta.cos(),
                                phi.sin() * theta.sin(),
                                phi.cos(),
                            );
                            let random_ori = offset_z * m * Vec3::new(-1.0, -1.0, 1.0);
                            Particle::new_directed(
                                Duration::from_secs_f64(beam.duration.0),
                                time,
                                ParticleMode::CultistFlame,
                                beam.bezier.start,
                                beam.bezier.start + random_ori * beam.range,
                            )
                        },
                    );
                },
                beam::FrontendSpecifier::LifestealBeam => {
                    // Emit a light when using lifesteal beam
                    if scene_data.flashing_lights_enabled {
                        lights.push(Light::new(beam.bezier.start, Rgb::new(0.8, 1.0, 0.5), 1.0));
                    }
                    self.particles.reserve(beam_tick_count as usize);
                    for i in 0..beam_tick_count {
                        self.particles.push(Particle::new_directed(
                            Duration::from_secs_f64(beam.duration.0),
                            time + i as f64 / 1000.0,
                            ParticleMode::LifestealBeam,
                            beam.bezier.start,
                            beam.bezier.start + beam_dir * beam.range,
                        ));
                    }
                },
                beam::FrontendSpecifier::Gravewarden => {
                    self.particles.resize_with(self.particles.len() + 2, || {
                        Particle::new_directed(
                            Duration::from_secs_f64(beam.duration.0),
                            time,
                            ParticleMode::Laser,
                            beam.bezier.start + beam_dir * 1.5,
                            beam.bezier.start + beam_dir * beam.range,
                        )
                    })
                },
                beam::FrontendSpecifier::WebStrand => {
                    self.particles.resize_with(self.particles.len() + 1, || {
                        Particle::new_directed(
                            Duration::from_secs_f64(beam.duration.0),
                            time,
                            ParticleMode::WebStrand,
                            beam.bezier.start,
                            beam.bezier.start + beam_dir * beam.range,
                        )
                    })
                },
                beam::FrontendSpecifier::Bubbles => {
                    let mut rng = thread_rng();
                    let (from, to) = (Vec3::<f32>::unit_z(), beam_dir);
                    let m = Mat3::<f32>::rotation_from_to_3d(from, to);
                    self.particles.resize_with(
                        self.particles.len() + usize::from(beam_tick_count) / 15,
                        || {
                            let phi: f32 = rng.gen_range(0.0..angle);
                            let theta: f32 = rng.gen_range(0.0..2.0 * PI);
                            let offset_z = Vec3::new(
                                phi.sin() * theta.cos(),
                                phi.sin() * theta.sin(),
                                phi.cos(),
                            );
                            let random_ori = offset_z * m * Vec3::new(-1.0, -1.0, 1.0);
                            Particle::new_directed(
                                Duration::from_secs_f64(beam.duration.0),
                                time,
                                ParticleMode::Bubbles,
                                beam.bezier.start,
                                beam.bezier.start + random_ori * beam.range,
                            )
                        },
                    );
                },
                beam::FrontendSpecifier::Poison => {
                    let mut rng = thread_rng();
                    let (from, to) = (Vec3::<f32>::unit_z(), beam_dir);
                    let m = Mat3::<f32>::rotation_from_to_3d(from, to);
                    self.particles.resize_with(
                        self.particles.len() + usize::from(beam_tick_count) / 15,
                        || {
                            let phi: f32 = rng.gen_range(0.0..angle);
                            let theta: f32 = rng.gen_range(0.0..2.0 * PI);
                            let offset_z = Vec3::new(
                                phi.sin() * theta.cos(),
                                phi.sin() * theta.sin(),
                                phi.cos(),
                            );
                            let random_ori = offset_z * m * Vec3::new(-1.0, -1.0, 1.0);
                            Particle::new_directed(
                                Duration::from_secs_f64(beam.duration.0),
                                time,
                                ParticleMode::CultistFlame,
                                beam.bezier.start,
                                beam.bezier.start + random_ori * beam.range,
                            )
                        },
                    );
                },
                beam::FrontendSpecifier::Ink => {
                    let mut rng = thread_rng();
                    let (from, to) = (Vec3::<f32>::unit_z(), beam_dir);
                    let m = Mat3::<f32>::rotation_from_to_3d(from, to);
                    self.particles.resize_with(
                        self.particles.len() + usize::from(beam_tick_count) / 15,
                        || {
                            let phi: f32 = rng.gen_range(0.0..angle);
                            let theta: f32 = rng.gen_range(0.0..2.0 * PI);
                            let offset_z = Vec3::new(
                                phi.sin() * theta.cos(),
                                phi.sin() * theta.sin(),
                                phi.cos(),
                            );
                            let random_ori = offset_z * m * Vec3::new(-1.0, -1.0, 1.0);
                            Particle::new_directed(
                                Duration::from_secs_f64(beam.duration.0),
                                time,
                                ParticleMode::Ink,
                                beam.bezier.start,
                                beam.bezier.start + random_ori * beam.range,
                            )
                        },
                    );
                },
                beam::FrontendSpecifier::Steam => {
                    let mut rng = thread_rng();
                    let (from, to) = (Vec3::<f32>::unit_z(), beam_dir);
                    let m = Mat3::<f32>::rotation_from_to_3d(from, to);
                    self.particles.resize_with(
                        self.particles.len() + usize::from(beam_tick_count) / 15,
                        || {
                            let phi: f32 = rng.gen_range(0.0..angle);
                            let theta: f32 = rng.gen_range(0.0..2.0 * PI);
                            let offset_z = Vec3::new(
                                phi.sin() * theta.cos(),
                                phi.sin() * theta.sin(),
                                phi.cos(),
                            );
                            let random_ori = offset_z * m * Vec3::new(-1.0, -1.0, 1.0);
                            Particle::new_directed(
                                Duration::from_secs_f64(beam.duration.0),
                                time,
                                ParticleMode::Steam,
                                beam.bezier.start,
                                beam.bezier.start + random_ori * beam.range,
                            )
                        },
                    );
                },
                beam::FrontendSpecifier::Lightning => {
                    self.particles.resize_with(self.particles.len() + 2, || {
                        Particle::new_directed(
                            Duration::from_secs_f64(beam.duration.0),
                            time,
                            ParticleMode::Lightning,
                            beam.bezier.start,
                            beam.bezier.start + beam_dir * beam.range,
                        )
                    })
                },
                beam::FrontendSpecifier::Frost => {
                    let mut rng = thread_rng();
                    let (from, to) = (Vec3::<f32>::unit_z(), beam_dir);
                    let m = Mat3::<f32>::rotation_from_to_3d(from, to);
                    self.particles.resize_with(
                        self.particles.len() + usize::from(beam_tick_count) / 4,
                        || {
                            let phi: f32 = rng.gen_range(0.0..angle);
                            let theta: f32 = rng.gen_range(0.0..2.0 * PI);
                            let offset_z = Vec3::new(
                                phi.sin() * theta.cos(),
                                phi.sin() * theta.sin(),
                                phi.cos(),
                            );
                            let random_ori = offset_z * m * Vec3::new(-1.0, -1.0, 1.0);
                            Particle::new_directed(
                                Duration::from_secs_f64(beam.duration.0),
                                time,
                                ParticleMode::Ice,
                                beam.bezier.start,
                                beam.bezier.start + random_ori * beam.range,
                            )
                        },
                    );
                },
                beam::FrontendSpecifier::PhoenixLaser => {
                    self.particles.resize_with(
                        self.particles.len() + usize::from(beam_tick_count) / 2,
                        || {
                            Particle::new_directed(
                                Duration::from_secs_f64(beam.duration.0),
                                time,
                                ParticleMode::PhoenixBeam,
                                beam.bezier.start,
                                beam.bezier.start + beam_dir * beam.range,
                            )
                        },
                    );
                },
            }
        }
    }

    fn maintain_aura_particles(&mut self, scene_data: &SceneData) {
        let state = scene_data.state;
        let ecs = state.ecs();
        let time = state.get_time();
        let mut rng = thread_rng();
        let dt = scene_data.state.get_delta_time();

        for (interp, pos, auras, body_maybe) in (
            ecs.read_storage::<Interpolated>().maybe(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<comp::Auras>(),
            ecs.read_storage::<comp::Body>().maybe(),
        )
            .join()
        {
            let pos = interp.map_or(pos.0, |i| i.pos);

            for (_, aura) in auras.auras.iter() {
                match aura.aura_kind {
                    aura::AuraKind::Buff {
                        kind: buff::BuffKind::ProtectingWard,
                        ..
                    } => {
                        let heartbeats = self.scheduler.heartbeats(Duration::from_millis(5));
                        self.particles.resize_with(
                            self.particles.len()
                                + aura.radius.powi(2) as usize * usize::from(heartbeats) / 300,
                            || {
                                let rand_dist = aura.radius * (1.0 - rng.gen::<f32>().powi(100));
                                let init_pos = Vec3::new(rand_dist, 0_f32, 0_f32);
                                let duration = Duration::from_secs_f64(
                                    aura.end_time
                                        .map_or(1.0, |end| end.0 - time)
                                        .clamp(0.0, 1.0),
                                );
                                Particle::new_directed(
                                    duration,
                                    time,
                                    ParticleMode::EnergyNature,
                                    pos,
                                    pos + init_pos,
                                )
                            },
                        );
                    },
                    aura::AuraKind::Buff {
                        kind: buff::BuffKind::Regeneration,
                        ..
                    } => {
                        if auras.auras.iter().any(|(_, aura)| {
                            matches!(aura.aura_kind, aura::AuraKind::Buff {
                                kind: buff::BuffKind::ProtectingWard,
                                ..
                            })
                        }) {
                            // If same entity has both protecting ward and regeneration auras, skip
                            // particles for regeneration
                            continue;
                        }
                        let heartbeats = self.scheduler.heartbeats(Duration::from_millis(5));
                        self.particles.resize_with(
                            self.particles.len()
                                + aura.radius.powi(2) as usize * usize::from(heartbeats) / 300,
                            || {
                                let rand_dist = aura.radius * (1.0 - rng.gen::<f32>().powi(100));
                                let init_pos = Vec3::new(rand_dist, 0_f32, 0_f32);
                                let duration = Duration::from_secs_f64(
                                    aura.end_time
                                        .map_or(1.0, |end| end.0 - time)
                                        .clamp(0.0, 1.0),
                                );
                                Particle::new_directed(
                                    duration,
                                    time,
                                    ParticleMode::EnergyHealing,
                                    pos,
                                    pos + init_pos,
                                )
                            },
                        );
                    },
                    aura::AuraKind::Buff {
                        kind: buff::BuffKind::Burning,
                        ..
                    } => {
                        let num_particles = aura.radius.powi(2) * dt / 250.0;
                        let num_particles = num_particles.floor() as usize
                            + usize::from(rng.gen_bool(f64::from(num_particles % 1.0)));
                        self.particles
                            .resize_with(self.particles.len() + num_particles, || {
                                let rand_pos = {
                                    let theta = rng.gen::<f32>() * TAU;
                                    let radius = aura.radius * rng.gen::<f32>().sqrt();
                                    let x = radius * theta.sin();
                                    let y = radius * theta.cos();
                                    Vec2::new(x, y) + pos.xy()
                                };
                                let duration = Duration::from_secs_f64(
                                    aura.end_time
                                        .map_or(1.0, |end| end.0 - time)
                                        .clamp(0.0, 1.0),
                                );
                                Particle::new_directed(
                                    duration,
                                    time,
                                    ParticleMode::FlameThrower,
                                    rand_pos.with_z(pos.z),
                                    rand_pos.with_z(pos.z + 1.0),
                                )
                            });
                    },
                    aura::AuraKind::Buff {
                        kind: buff::BuffKind::Hastened,
                        ..
                    } => {
                        let heartbeats = self.scheduler.heartbeats(Duration::from_millis(5));
                        self.particles.resize_with(
                            self.particles.len()
                                + aura.radius.powi(2) as usize * usize::from(heartbeats) / 300,
                            || {
                                let rand_dist = aura.radius * (1.0 - rng.gen::<f32>().powi(100));
                                let init_pos = Vec3::new(rand_dist, 0_f32, 0_f32);
                                let duration = Duration::from_secs_f64(
                                    aura.end_time
                                        .map_or(1.0, |end| end.0 - time)
                                        .clamp(0.0, 1.0),
                                );
                                Particle::new_directed(
                                    duration,
                                    time,
                                    ParticleMode::EnergyBuffing,
                                    pos,
                                    pos + init_pos,
                                )
                            },
                        );
                    },
                    aura::AuraKind::Buff {
                        kind: buff::BuffKind::Frozen,
                        ..
                    } => {
                        let is_new_aura = aura.data.duration.map_or(true, |max_dur| {
                            let rem_dur = aura.end_time.map_or(time, |e| e.0) - time;
                            rem_dur > max_dur.0 * 0.9
                        });
                        if is_new_aura {
                            let heartbeats = self.scheduler.heartbeats(Duration::from_millis(5));
                            self.particles.resize_with(
                                self.particles.len()
                                    + aura.radius.powi(2) as usize * usize::from(heartbeats) / 300,
                                || {
                                    let rand_angle = rng.gen_range(0.0..TAU);
                                    let offset =
                                        Vec2::new(rand_angle.cos(), rand_angle.sin()) * aura.radius;
                                    let z_start = body_maybe
                                        .map_or(0.0, |b| rng.gen_range(0.5..0.75) * b.height());
                                    let z_end = body_maybe
                                        .map_or(0.0, |b| rng.gen_range(0.0..3.0) * b.height());
                                    Particle::new_directed(
                                        Duration::from_secs(3),
                                        time,
                                        ParticleMode::Ice,
                                        pos.x + Vec3::unit_z() * z_start,
                                        pos.x + offset.with_z(z_end),
                                    )
                                },
                            );
                        }
                    },
                    aura::AuraKind::Buff {
                        kind: buff::BuffKind::Heatstroke,
                        ..
                    } => {
                        let heartbeats = self.scheduler.heartbeats(Duration::from_millis(5));
                        self.particles.resize_with(
                            self.particles.len()
                                + aura.radius.powi(2) as usize * usize::from(heartbeats) / 900,
                            || {
                                let rand_dist = aura.radius * (1.0 - rng.gen::<f32>().powi(100));
                                let init_pos = Vec3::new(rand_dist, 0_f32, 0_f32);
                                let duration = Duration::from_secs_f64(
                                    aura.end_time
                                        .map_or(1.0, |end| end.0 - time)
                                        .clamp(0.0, 1.0),
                                );
                                Particle::new_directed(
                                    duration,
                                    time,
                                    ParticleMode::EnergyPhoenix,
                                    pos,
                                    pos + init_pos,
                                )
                            },
                        );

                        let num_particles = aura.radius.powi(2) * dt / 50.0;
                        let num_particles = num_particles.floor() as usize
                            + usize::from(rng.gen_bool(f64::from(num_particles % 1.0)));
                        self.particles
                            .resize_with(self.particles.len() + num_particles, || {
                                let rand_pos = {
                                    let theta = rng.gen::<f32>() * TAU;
                                    let radius = aura.radius * rng.gen::<f32>().sqrt();
                                    let x = radius * theta.sin();
                                    let y = radius * theta.cos();
                                    Vec2::new(x, y) + pos.xy()
                                };
                                let duration = Duration::from_secs_f64(
                                    aura.end_time
                                        .map_or(1.0, |end| end.0 - time)
                                        .clamp(0.0, 1.0),
                                );
                                Particle::new_directed(
                                    duration,
                                    time,
                                    ParticleMode::FieryBurstAsh,
                                    pos,
                                    Vec3::new(
                                                    0.0,    // radius of rand spawn
                                                    20.0,   // integer part - radius of the curve part, fractional part - relative time of setting particle on fire
                                                    5.5)    // height of the flight
                                                    + rand_pos.with_z(pos.z),
                                )
                            });
                    },
                    _ => {},
                }
            }
        }
    }

    fn maintain_buff_particles(&mut self, scene_data: &SceneData) {
        let state = scene_data.state;
        let ecs = state.ecs();
        let time = state.get_time();
        let mut rng = thread_rng();

        for (interp, pos, buffs, body, ori, scale) in (
            ecs.read_storage::<Interpolated>().maybe(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<comp::Buffs>(),
            &ecs.read_storage::<Body>(),
            &ecs.read_storage::<Ori>(),
            ecs.read_storage::<Scale>().maybe(),
        )
            .join()
        {
            let pos = interp.map_or(pos.0, |i| i.pos);

            for (buff_kind, buff_keys) in buffs
                .kinds
                .iter()
                .filter_map(|(kind, keys)| keys.as_ref().map(|keys| (kind, keys)))
            {
                use buff::BuffKind;
                match buff_kind {
                    BuffKind::Cursed | BuffKind::Burning => {
                        self.particles.resize_with(
                            self.particles.len()
                                + usize::from(self.scheduler.heartbeats(Duration::from_millis(15))),
                            || {
                                let start_pos = pos
                                    + Vec3::unit_z() * body.height() * 0.25
                                    + Vec3::<f32>::zero()
                                        .map(|_| rng.gen_range(-1.0..1.0))
                                        .normalized()
                                        * 0.25;
                                let end_pos = start_pos
                                    + Vec3::unit_z() * body.height()
                                    + Vec3::<f32>::zero()
                                        .map(|_| rng.gen_range(-1.0..1.0))
                                        .normalized();
                                Particle::new_directed(
                                    Duration::from_secs(1),
                                    time,
                                    if matches!(buff_kind, BuffKind::Cursed) {
                                        ParticleMode::CultistFlame
                                    } else {
                                        ParticleMode::FlameThrower
                                    },
                                    start_pos,
                                    end_pos,
                                )
                            },
                        );
                    },
                    BuffKind::PotionSickness => {
                        let mut multiplicity = 0;
                        // Only show particles for potion sickness at the beginning, after the
                        // drinking animation finishes
                        if buff_keys.0
                            .iter()
                            .filter_map(|key| buffs.buffs.get(*key))
                            .any(|buff| {
                                matches!(buff.elapsed(Time(time)), dur if (1.0..=1.5).contains(&dur.0))
                            })
                        {
                            multiplicity = 1;
                        }
                        self.particles.resize_with(
                            self.particles.len()
                                + multiplicity
                                    * usize::from(
                                        self.scheduler.heartbeats(Duration::from_millis(25)),
                                    ),
                            || {
                                let start_pos = pos
                                    + Vec3::unit_z() * body.eye_height(scale.map_or(1.0, |s| s.0));
                                let (radius, theta) =
                                    (rng.gen_range(0.0f32..1.0).sqrt(), rng.gen_range(0.0..TAU));
                                let end_pos = pos
                                    + *ori.look_dir()
                                    + Vec3::<f32>::new(
                                        radius * theta.cos(),
                                        radius * theta.sin(),
                                        0.0,
                                    ) * 0.25;
                                Particle::new_directed(
                                    Duration::from_secs(1),
                                    time,
                                    ParticleMode::PotionSickness,
                                    start_pos,
                                    end_pos,
                                )
                            },
                        );
                    },
                    BuffKind::Frenzied => {
                        self.particles.resize_with(
                            self.particles.len()
                                + usize::from(self.scheduler.heartbeats(Duration::from_millis(15))),
                            || {
                                let start_pos = pos
                                    + Vec3::new(
                                        body.max_radius(),
                                        body.max_radius(),
                                        body.height() / 2.0,
                                    )
                                    .map(|d| d * rng.gen_range(-1.0..1.0));
                                let end_pos = start_pos
                                    + Vec3::unit_z() * body.height()
                                    + Vec3::<f32>::zero()
                                        .map(|_| rng.gen_range(-1.0..1.0))
                                        .normalized();
                                Particle::new_directed(
                                    Duration::from_secs(1),
                                    time,
                                    ParticleMode::Enraged,
                                    start_pos,
                                    end_pos,
                                )
                            },
                        );
                    },
                    BuffKind::Polymorphed => {
                        let mut multiplicity = 0;
                        // Only show particles for polymorph at the beginning, after the
                        // drinking animation finishes
                        if buff_keys.0
                            .iter()
                            .filter_map(|key| buffs.buffs.get(*key))
                            .any(|buff| {
                                matches!(buff.elapsed(Time(time)), dur if (0.1..=0.3).contains(&dur.0))
                            })
                        {
                            multiplicity = 1;
                        }
                        self.particles.resize_with(
                            self.particles.len()
                                + multiplicity
                                    * self.scheduler.heartbeats(Duration::from_millis(3)) as usize,
                            || {
                                let start_pos = pos
                                    + Vec3::unit_z() * body.eye_height(scale.map_or(1.0, |s| s.0))
                                        / 2.0;
                                let end_pos = start_pos
                                    + Vec3::<f32>::zero()
                                        .map(|_| rng.gen_range(-1.0..1.0))
                                        .normalized()
                                        * 5.0;

                                Particle::new_directed(
                                    Duration::from_secs(2),
                                    time,
                                    ParticleMode::Explosion,
                                    start_pos,
                                    end_pos,
                                )
                            },
                        )
                    },
                    _ => {},
                }
            }
        }
    }

    fn maintain_block_particles(
        &mut self,
        scene_data: &SceneData,
        terrain: &Terrain<TerrainChunk>,
        figure_mgr: &FigureMgr,
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
            .read_component_copied::<Interpolated>(scene_data.viewpoint_entity)
            .map(|i| i.pos)
            .unwrap_or_default();
        let player_chunk = player_pos.xy().map2(TerrainChunk::RECT_SIZE, |e, sz| {
            (e.floor() as i32).div_euclid(sz as i32)
        });

        struct BlockParticles<'a> {
            // The function to select the blocks of interest that we should emit from
            blocks: fn(&'a BlocksOfInterest) -> BlockParticleSlice<'a>,
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

        enum BlockParticleSlice<'a> {
            Positions(&'a [Vec3<i32>]),
            PositionsAndDirs(&'a [(Vec3<i32>, Vec3<f32>)]),
        }

        impl<'a> BlockParticleSlice<'a> {
            fn len(&self) -> usize {
                match self {
                    Self::Positions(blocks) => blocks.len(),
                    Self::PositionsAndDirs(blocks) => blocks.len(),
                }
            }
        }

        let particles: &[BlockParticles] = &[
            BlockParticles {
                blocks: |boi| BlockParticleSlice::Positions(&boi.leaves),
                range: 4,
                rate: 0.0125,
                lifetime: 30.0,
                mode: ParticleMode::Leaf,
                cond: |_| true,
            },
            BlockParticles {
                blocks: |boi| BlockParticleSlice::Positions(&boi.drip),
                range: 4,
                rate: 0.004,
                lifetime: 20.0,
                mode: ParticleMode::Drip,
                cond: |_| true,
            },
            BlockParticles {
                blocks: |boi| BlockParticleSlice::Positions(&boi.fires),
                range: 2,
                rate: 20.0,
                lifetime: 0.25,
                mode: ParticleMode::CampfireFire,
                cond: |_| true,
            },
            BlockParticles {
                blocks: |boi| BlockParticleSlice::Positions(&boi.fire_bowls),
                range: 2,
                rate: 20.0,
                lifetime: 0.25,
                mode: ParticleMode::FireBowl,
                cond: |_| true,
            },
            BlockParticles {
                blocks: |boi| BlockParticleSlice::Positions(&boi.fireflies),
                range: 6,
                rate: 0.004,
                lifetime: 40.0,
                mode: ParticleMode::Firefly,
                cond: |sd| sd.state.get_day_period().is_dark(),
            },
            BlockParticles {
                blocks: |boi| BlockParticleSlice::Positions(&boi.flowers),
                range: 5,
                rate: 0.002,
                lifetime: 40.0,
                mode: ParticleMode::Firefly,
                cond: |sd| sd.state.get_day_period().is_dark(),
            },
            BlockParticles {
                blocks: |boi| BlockParticleSlice::Positions(&boi.beehives),
                range: 3,
                rate: 0.5,
                lifetime: 30.0,
                mode: ParticleMode::Bee,
                cond: |sd| sd.state.get_day_period().is_light(),
            },
            BlockParticles {
                blocks: |boi| BlockParticleSlice::Positions(&boi.snow),
                range: 4,
                rate: 0.025,
                lifetime: 15.0,
                mode: ParticleMode::Snow,
                cond: |_| true,
            },
            BlockParticles {
                blocks: |boi| BlockParticleSlice::PositionsAndDirs(&boi.one_way_walls),
                range: 2,
                rate: 12.0,
                lifetime: 1.5,
                mode: ParticleMode::PortalFizz,
                cond: |_| true,
            },
        ];

        let ecs = scene_data.state.ecs();
        let mut rng = thread_rng();
        // Hard cap for performance reasons; Assuming that 25% of a chunk is covered in
        // lava or 32*32*0.25 = 256 TODO: Make this a setting?
        let cap = 512;
        for particles in particles.iter() {
            if !(particles.cond)(scene_data) {
                continue;
            }

            for offset in Spiral2d::new().take((particles.range * 2 + 1).pow(2)) {
                let chunk_pos = player_chunk + offset;

                terrain.get(chunk_pos).map(|chunk_data| {
                    let blocks = (particles.blocks)(&chunk_data.blocks_of_interest);

                    let avg_particles = dt * (blocks.len() as f32 * particles.rate).min(cap as f32);
                    let particle_count = avg_particles.trunc() as usize
                        + (rng.gen::<f32>() < avg_particles.fract()) as usize;

                    self.particles
                        .resize_with(self.particles.len() + particle_count, || {
                            match blocks {
                                BlockParticleSlice::Positions(blocks) => {
                                    // Can't fail, resize only occurs if blocks > 0
                                    let block_pos = Vec3::from(
                                        chunk_pos * TerrainChunk::RECT_SIZE.map(|e| e as i32),
                                    ) + blocks.choose(&mut rng).copied().unwrap();
                                    Particle::new(
                                        Duration::from_secs_f32(particles.lifetime),
                                        time,
                                        particles.mode,
                                        block_pos.map(|e: i32| e as f32 + rng.gen::<f32>()),
                                    )
                                },
                                BlockParticleSlice::PositionsAndDirs(blocks) => {
                                    // Can't fail, resize only occurs if blocks > 0
                                    let (block_offset, particle_dir) =
                                        blocks.choose(&mut rng).copied().unwrap();
                                    let block_pos = Vec3::from(
                                        chunk_pos * TerrainChunk::RECT_SIZE.map(|e| e as i32),
                                    ) + block_offset;
                                    let particle_pos =
                                        block_pos.map(|e: i32| e as f32 + rng.gen::<f32>());
                                    Particle::new_directed(
                                        Duration::from_secs_f32(particles.lifetime),
                                        time,
                                        particles.mode,
                                        particle_pos,
                                        particle_pos + particle_dir,
                                    )
                                },
                            }
                        })
                });
            }

            for (entity, body, interpolated, collider) in (
                &ecs.entities(),
                &ecs.read_storage::<comp::Body>(),
                &ecs.read_storage::<crate::ecs::comp::Interpolated>(),
                ecs.read_storage::<comp::Collider>().maybe(),
            )
                .join()
            {
                if let Some((blocks_of_interest, offset)) =
                    figure_mgr.get_blocks_of_interest(entity, body, collider)
                {
                    let mat = Mat4::from(interpolated.ori.to_quat())
                        .translated_3d(interpolated.pos)
                        * Mat4::translation_3d(offset);

                    let blocks = (particles.blocks)(blocks_of_interest);

                    let avg_particles = dt * blocks.len() as f32 * particles.rate;
                    let particle_count = avg_particles.trunc() as usize
                        + (rng.gen::<f32>() < avg_particles.fract()) as usize;

                    self.particles
                        .resize_with(self.particles.len() + particle_count, || {
                            match blocks {
                                BlockParticleSlice::Positions(blocks) => {
                                    let rel_pos = blocks
                                        .choose(&mut rng)
                                        .copied()
                                        // Can't fail, resize only occurs if blocks > 0
                                        .unwrap()
                                        .map(|e: i32| e as f32 + rng.gen::<f32>());
                                    let wpos = mat.mul_point(rel_pos);

                                    Particle::new(
                                        Duration::from_secs_f32(particles.lifetime),
                                        time,
                                        particles.mode,
                                        wpos,
                                    )
                                },
                                BlockParticleSlice::PositionsAndDirs(blocks) => {
                                    // Can't fail, resize only occurs if blocks > 0
                                    let (block_offset, particle_dir) =
                                        blocks.choose(&mut rng).copied().unwrap();
                                    let particle_pos =
                                        block_offset.map(|e: i32| e as f32 + rng.gen::<f32>());
                                    let wpos = mat.mul_point(particle_pos);
                                    Particle::new_directed(
                                        Duration::from_secs_f32(particles.lifetime),
                                        time,
                                        particles.mode,
                                        wpos,
                                        wpos + mat.mul_direction(particle_dir),
                                    )
                                },
                            }
                        })
                }
            }
        }
        // smoke is more complex as it comes with varying rate and color
        {
            struct SmokeProperties {
                position: Vec3<i32>,
                strength: f32,
                dry_chance: f32,
            }

            let range = 8_usize;
            let rate = 3.0 / 128.0;
            let lifetime = 40.0;
            let time_of_day = scene_data
                .state
                .get_time_of_day()
                .rem_euclid(24.0 * 60.0 * 60.0) as f32;

            for offset in Spiral2d::new().take((range * 2 + 1).pow(2)) {
                let chunk_pos = player_chunk + offset;

                terrain.get(chunk_pos).map(|chunk_data| {
                    let blocks = &chunk_data.blocks_of_interest.smokers;
                    let mut smoke_properties: Vec<SmokeProperties> = Vec::new();
                    let block_pos =
                        Vec3::from(chunk_pos * TerrainChunk::RECT_SIZE.map(|e| e as i32));
                    let mut sum = 0.0_f32;
                    for smoker in blocks.iter() {
                        let position = block_pos + smoker.position;
                        let (strength, dry_chance) = {
                            match smoker.kind {
                                FireplaceType::House => {
                                    let prop = crate::scene::smoke_cycle::smoke_at_time(
                                        position,
                                        chunk_data.blocks_of_interest.temperature,
                                        time_of_day,
                                    );
                                    (
                                        prop.0,
                                        if prop.1 {
                                            // fire started, dark smoke
                                            0.8 - chunk_data.blocks_of_interest.humidity
                                        } else {
                                            // fire continues, light smoke
                                            1.2 - chunk_data.blocks_of_interest.humidity
                                        },
                                    )
                                },
                                FireplaceType::Workshop => (128.0, 1.0),
                            }
                        };
                        sum += strength;
                        smoke_properties.push(SmokeProperties {
                            position,
                            strength,
                            dry_chance,
                        });
                    }
                    let avg_particles = dt * sum * rate;

                    let particle_count = avg_particles.trunc() as usize
                        + (rng.gen::<f32>() < avg_particles.fract()) as usize;
                    let chosen = smoke_properties.choose_multiple_weighted(
                        &mut rng,
                        particle_count,
                        |smoker| smoker.strength,
                    );
                    if let Ok(chosen) = chosen {
                        self.particles.extend(chosen.map(|smoker| {
                            Particle::new(
                                Duration::from_secs_f32(lifetime),
                                time,
                                if rng.gen::<f32>() > smoker.dry_chance {
                                    ParticleMode::BlackSmoke
                                } else {
                                    ParticleMode::CampfireSmoke
                                },
                                smoker.position.map(|e: i32| e as f32 + rng.gen::<f32>()),
                            )
                        }));
                    }
                });
            }
        }
    }

    fn maintain_shockwave_particles(&mut self, scene_data: &SceneData) {
        let state = scene_data.state;
        let ecs = state.ecs();
        let time = state.get_time();
        let dt = scene_data.state.ecs().fetch::<DeltaTime>().0;
        let terrain = scene_data.state.ecs().fetch::<TerrainGrid>();

        for (_entity, interp, pos, ori, shockwave) in (
            &ecs.entities(),
            ecs.read_storage::<Interpolated>().maybe(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Ori>(),
            &ecs.read_storage::<Shockwave>(),
        )
            .join()
        {
            let pos = interp.map_or(pos.0, |i| i.pos);
            let ori = interp.map_or(*ori, |i| i.ori);

            let elapsed = time - shockwave.creation.unwrap_or(time);
            let speed = shockwave.properties.speed;

            let percent = elapsed as f32 / shockwave.properties.duration.as_secs_f32();

            let distance = speed * elapsed as f32;

            let radians = shockwave.properties.angle.to_radians();

            let ori_vec = ori.look_vec();
            let theta = ori_vec.y.atan2(ori_vec.x) - radians / 2.0;
            let dtheta = radians / distance;

            // Number of particles derived from arc length (for new particles at least, old
            // can be converted later)
            let arc_length = distance * radians;

            use shockwave::FrontendSpecifier;
            match shockwave.properties.specifier {
                FrontendSpecifier::Ground => {
                    let heartbeats = self.scheduler.heartbeats(Duration::from_millis(2));
                    for heartbeat in 0..heartbeats {
                        // 1 / 3 the size of terrain voxel
                        let scale = 1.0 / 3.0;

                        let scaled_speed = speed * scale;

                        let sub_tick_interpolation = scaled_speed * 1000.0 * heartbeat as f32;

                        let distance = speed * (elapsed as f32 - sub_tick_interpolation);

                        let particle_count_factor = radians / (3.0 * scale);
                        let new_particle_count = distance * particle_count_factor;
                        self.particles.reserve(new_particle_count as usize);

                        for d in 0..(new_particle_count as i32) {
                            let arc_position = theta + dtheta * d as f32 / particle_count_factor;

                            let position = pos
                                + distance * Vec3::new(arc_position.cos(), arc_position.sin(), 0.0);

                            // Arbitrary number chosen that is large enough to be able to accurately
                            // place particles most of the time, but also not too big as to make ray
                            // be too large (for performance reasons)
                            let half_ray_length = 10.0;
                            let mut last_air = false;
                            // TODO: Optimize ray to only be cast at most once per block per tick if
                            // it becomes an issue.
                            // From imbris:
                            //      each ray is ~2 us
                            //      at 30 FPS, it peaked at 113 rays in a tick
                            //      total time was 240 us (although potentially half that is
                            //          overhead from the profiling of each ray)
                            let _ = terrain
                                .ray(
                                    position + Vec3::unit_z() * half_ray_length,
                                    position - Vec3::unit_z() * half_ray_length,
                                )
                                .for_each(|block: &Block, pos: Vec3<i32>| {
                                    if block.is_solid() && block.get_sprite().is_none() {
                                        if last_air {
                                            let position = position.xy().with_z(pos.z as f32 + 1.0);

                                            let position_snapped =
                                                ((position / scale).floor() + 0.5) * scale;

                                            self.particles.push(Particle::new(
                                                Duration::from_millis(250),
                                                time,
                                                ParticleMode::GroundShockwave,
                                                position_snapped,
                                            ));
                                            last_air = false;
                                        }
                                    } else {
                                        last_air = true;
                                    }
                                })
                                .cast();
                        }
                    }
                },
                FrontendSpecifier::Fire => {
                    let heartbeats = self.scheduler.heartbeats(Duration::from_millis(2));
                    for _ in 0..heartbeats {
                        for d in 0..3 * distance as i32 {
                            let arc_position = theta + dtheta * d as f32 / 3.0;

                            let position = pos
                                + distance * Vec3::new(arc_position.cos(), arc_position.sin(), 0.0);

                            self.particles.push(Particle::new(
                                Duration::from_secs_f32((distance + 10.0) / 50.0),
                                time,
                                ParticleMode::FireShockwave,
                                position,
                            ));
                        }
                    }
                },
                FrontendSpecifier::Water => {
                    // 1 particle per unit length of arc
                    let particles_per_length = arc_length as usize;
                    let dtheta = radians / particles_per_length as f32;
                    // Scales number of desired heartbeats from speed - thicker arc = higher speed =
                    // lower duration = more particles
                    let heartbeats = self
                        .scheduler
                        .heartbeats(Duration::from_secs_f32(1.0 / speed));

                    // Reserves capacity for new particles
                    let new_particle_count = particles_per_length * heartbeats as usize;
                    self.particles.reserve(new_particle_count);

                    for i in 0..particles_per_length {
                        let angle = dtheta * i as f32;
                        let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
                        for j in 0..heartbeats {
                            // Sub tick dt
                            let dt = (j as f32 / heartbeats as f32) * dt;
                            let distance = distance + speed * dt;
                            let pos1 = pos + distance * direction - Vec3::unit_z();
                            let pos2 = pos1 + (Vec3::unit_z() + direction) * 3.0;
                            let time = time + dt as f64;

                            self.particles.push(Particle::new_directed(
                                Duration::from_secs_f32(0.5),
                                time,
                                ParticleMode::Water,
                                pos1,
                                pos2,
                            ));
                        }
                    }
                },
                FrontendSpecifier::Lightning => {
                    // 1 particle per unit length of arc
                    let particles_per_length = arc_length as usize;
                    let dtheta = radians / particles_per_length as f32;
                    // Scales number of desired heartbeats from speed - thicker arc = higher speed =
                    // lower duration = more particles
                    let heartbeats = self
                        .scheduler
                        .heartbeats(Duration::from_secs_f32(1.0 / speed));

                    // Reserves capacity for new particles
                    let new_particle_count = particles_per_length * heartbeats as usize;
                    self.particles.reserve(new_particle_count);

                    for i in 0..particles_per_length {
                        let angle = dtheta * i as f32;
                        let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
                        for j in 0..heartbeats {
                            // Sub tick dt
                            let dt = (j as f32 / heartbeats as f32) * dt;
                            let distance = distance + speed * dt;
                            let pos1 = pos + distance * direction - Vec3::unit_z();
                            let pos2 = pos1 + (Vec3::unit_z() + direction) * 3.0;
                            let time = time + dt as f64;

                            self.particles.push(Particle::new_directed(
                                Duration::from_secs_f32(0.5),
                                time,
                                ParticleMode::Lightning,
                                pos1,
                                pos2,
                            ));
                        }
                    }
                },
                FrontendSpecifier::Steam => {
                    // 1 particle per unit length of arc
                    let particles_per_length = arc_length as usize;
                    let dtheta = radians / particles_per_length as f32;
                    // Scales number of desired heartbeats from speed - thicker arc = higher speed =
                    // lower duration = more particles
                    let heartbeats = self
                        .scheduler
                        .heartbeats(Duration::from_secs_f32(1.0 / speed));

                    // Reserves capacity for new particles
                    let new_particle_count = particles_per_length * heartbeats as usize;
                    self.particles.reserve(new_particle_count);

                    for i in 0..particles_per_length {
                        let angle = dtheta * i as f32;
                        let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
                        for j in 0..heartbeats {
                            // Sub tick dt
                            let dt = (j as f32 / heartbeats as f32) * dt;
                            let distance = distance + speed * dt;
                            let pos1 = pos + distance * direction - Vec3::unit_z();
                            let pos2 = pos1 + (Vec3::unit_z() + direction) * 3.0;
                            let time = time + dt as f64;

                            self.particles.push(Particle::new_directed(
                                Duration::from_secs_f32(0.5),
                                time,
                                ParticleMode::Steam,
                                pos1,
                                pos2,
                            ));
                        }
                    }
                },
                FrontendSpecifier::Poison => {
                    // 1 particle per unit length of arc
                    let particles_per_length = arc_length as usize;
                    let dtheta = radians / particles_per_length as f32;
                    // Scales number of desired heartbeats from speed - thicker arc = higher speed =
                    // lower duration = more particles
                    let heartbeats = self
                        .scheduler
                        .heartbeats(Duration::from_secs_f32(1.0 / speed));

                    // Reserves capacity for new particles
                    let new_particle_count = particles_per_length * heartbeats as usize;
                    self.particles.reserve(new_particle_count);

                    for i in 0..particles_per_length {
                        let angle = dtheta * i as f32;
                        let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
                        for j in 0..heartbeats {
                            // Sub tick dt
                            let dt = (j as f32 / heartbeats as f32) * dt;
                            let distance = distance + speed * dt;
                            let pos1 = pos + distance * direction - Vec3::unit_z();
                            let pos2 = pos1 + (Vec3::unit_z() + direction) * 3.0;
                            let time = time + dt as f64;

                            self.particles.push(Particle::new_directed(
                                Duration::from_secs_f32(0.5),
                                time,
                                ParticleMode::CultistFlame,
                                pos1,
                                pos2,
                            ));
                        }
                    }
                },
                FrontendSpecifier::Ink => {
                    // 1 particle per unit length of arc
                    let particles_per_length = arc_length as usize;
                    let dtheta = radians / particles_per_length as f32;
                    // Scales number of desired heartbeats from speed - thicker arc = higher speed =
                    // lower duration = more particles
                    let heartbeats = self
                        .scheduler
                        .heartbeats(Duration::from_secs_f32(1.0 / speed));

                    // Reserves capacity for new particles
                    let new_particle_count = particles_per_length * heartbeats as usize;
                    self.particles.reserve(new_particle_count);

                    for i in 0..particles_per_length {
                        let angle = dtheta * i as f32;
                        let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
                        for j in 0..heartbeats {
                            // Sub tick dt
                            let dt = (j as f32 / heartbeats as f32) * dt;
                            let distance = distance + speed * dt;
                            let pos1 = pos + distance * direction - Vec3::unit_z();
                            let pos2 = pos1 + (Vec3::unit_z() + direction) * 3.0;
                            let time = time + dt as f64;

                            self.particles.push(Particle::new_directed(
                                Duration::from_secs_f32(0.5),
                                time,
                                ParticleMode::Ink,
                                pos1,
                                pos2,
                            ));
                        }
                    }
                },
                FrontendSpecifier::IceSpikes | FrontendSpecifier::Ice => {
                    // 1 / 3 the size of terrain voxel
                    let scale = 1.0 / 3.0;
                    let scaled_distance = distance / scale;
                    let scaled_speed = speed / scale;

                    // 1 particle per scaled unit length of arc
                    let particles_per_length = (0.25 * arc_length / scale) as usize;
                    let dtheta = radians / particles_per_length as f32;
                    // Scales number of desired heartbeats from speed - thicker arc = higher speed =
                    // lower duration = more particles
                    let heartbeats = self
                        .scheduler
                        .heartbeats(Duration::from_secs_f32(3.0 / scaled_speed));

                    // Reserves capacity for new particles
                    let new_particle_count = particles_per_length * heartbeats as usize;
                    self.particles.reserve(new_particle_count);
                    // higher wave when wave doesn't require ground
                    let wave = if matches!(shockwave.properties.dodgeable, ShockwaveDodgeable::Jump)
                    {
                        0.5
                    } else {
                        8.0
                    };
                    // Used to make taller the further out spikes are
                    let height_scale = wave + 1.5 * percent;
                    for i in 0..particles_per_length {
                        let angle = theta + dtheta * i as f32;
                        let direction = Vec3::new(angle.cos(), angle.sin(), 0.0);
                        for j in 0..heartbeats {
                            // Sub tick dt
                            let dt = (j as f32 / heartbeats as f32) * dt;
                            let scaled_distance = scaled_distance + scaled_speed * dt;
                            let mut pos1 = pos + (scaled_distance * direction).floor() * scale;
                            let time = time + dt as f64;

                            // Arbitrary number chosen that is large enough to be able to accurately
                            // place particles most of the time, but also not too big as to make ray
                            // be too large (for performance reasons)
                            let half_ray_length = 10.0;
                            let mut last_air = false;
                            // TODO: Optimize ray to only be cast at most once per block per tick if
                            // it becomes an issue.
                            // From imbris:
                            //      each ray is ~2 us
                            //      at 30 FPS, it peaked at 113 rays in a tick
                            //      total time was 240 us (although potentially half that is
                            //          overhead from the profiling of each ray)
                            let _ = terrain
                                .ray(
                                    pos1 + Vec3::unit_z() * half_ray_length,
                                    pos1 - Vec3::unit_z() * half_ray_length,
                                )
                                .for_each(|block: &Block, pos: Vec3<i32>| {
                                    if block.is_solid() && block.get_sprite().is_none() {
                                        if last_air {
                                            pos1 = pos1.xy().with_z(pos.z as f32 + 1.0);
                                            last_air = false;
                                        }
                                    } else {
                                        last_air = true;
                                    }
                                })
                                .cast();

                            let get_positions = |a| {
                                let pos1 = match a {
                                    2 => pos1 + Vec3::unit_x() * scale,
                                    3 => pos1 - Vec3::unit_x() * scale,
                                    4 => pos1 + Vec3::unit_y() * scale,
                                    5 => pos1 - Vec3::unit_y() * scale,
                                    _ => pos1,
                                };
                                let pos2 = if a == 1 {
                                    pos1 + Vec3::unit_z() * 5.0 * height_scale
                                } else {
                                    pos1 + Vec3::unit_z() * 1.0 * height_scale
                                };
                                (pos1, pos2)
                            };

                            for a in 1..=5 {
                                let (pos1, pos2) = get_positions(a);
                                self.particles.push(Particle::new_directed(
                                    Duration::from_secs_f32(0.5),
                                    time,
                                    ParticleMode::IceSpikes,
                                    pos1,
                                    pos2,
                                ));
                            }
                        }
                    }
                },
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
        let gpu_instances = renderer.create_instances(&all_cpu_instances);

        self.instances = gpu_instances;
    }

    pub fn render<'a>(&'a self, drawer: &mut ParticleDrawer<'_, 'a>, scene_data: &SceneData) {
        span!(_guard, "render", "ParticleMgr::render");
        if scene_data.particles_enabled {
            let model = &self
                .model_cache
                .get(DEFAULT_MODEL_KEY)
                .expect("Expected particle model in cache");

            drawer.draw(model, &self.instances);
        }
    }

    pub fn particle_count(&self) -> usize { self.instances.count() }

    pub fn particle_count_visible(&self) -> usize { self.instances.count() }
}

fn default_instances(renderer: &mut Renderer) -> Instances<ParticleInstance> {
    let empty_vec = Vec::new();

    renderer.create_instances(&empty_vec)
}

const DEFAULT_MODEL_KEY: &str = "voxygen.voxel.particle";

fn default_cache(renderer: &mut Renderer) -> HashMap<&'static str, Model<ParticleVertex>> {
    let mut model_cache = HashMap::new();

    model_cache.entry(DEFAULT_MODEL_KEY).or_insert_with(|| {
        let vox = DotVoxAsset::load_expect(DEFAULT_MODEL_KEY);

        // NOTE: If we add texturing we may eventually try to share it among all
        // particles in a single atlas.
        let max_texture_size = renderer.max_texture_size();
        let max_size = Vec2::from(u16::try_from(max_texture_size).unwrap_or(u16::MAX));
        let mut greedy = GreedyMesh::new(max_size, crate::mesh::greedy::general_config());

        let segment = Segment::from_vox_model_index(&vox.read().0, 0);
        let segment_size = segment.size();
        let mut mesh = generate_mesh_base_vol_particle(segment, &mut greedy).0;
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
