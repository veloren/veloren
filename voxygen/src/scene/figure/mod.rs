mod cache;
pub mod load;

pub use cache::FigureModelCache;
pub use load::load_mesh; // TODO: Don't make this public.

use crate::{
    anim::{
        self, biped_large::BipedLargeSkeleton, bird_medium::BirdMediumSkeleton,
        bird_small::BirdSmallSkeleton, character::CharacterSkeleton, critter::CritterSkeleton,
        dragon::DragonSkeleton, fish_medium::FishMediumSkeleton, fish_small::FishSmallSkeleton,
        object::ObjectSkeleton, quadruped_medium::QuadrupedMediumSkeleton,
        quadruped_small::QuadrupedSmallSkeleton, Animation, Skeleton,
    },
    ecs::comp::Interpolated,
    render::{Consts, FigureBoneData, FigureLocals, Globals, Light, Renderer, Shadow},
    scene::{
        camera::{Camera, CameraMode},
        SceneData,
    },
};
use common::{
    comp::{
        item::ItemKind, Body, CharacterState, Last, Loadout, Ori, PhysicsState, Pos, Scale, Stats,
        Vel,
    },
    state::State,
    states::triple_strike,
    terrain::TerrainChunk,
    vol::RectRasterableVol,
};
use hashbrown::HashMap;
use log::trace;
use specs::{Entity as EcsEntity, Join, WorldExt};
use treeculler::{BVol, BoundingSphere};
use vek::*;

const DAMAGE_FADE_COEFFICIENT: f64 = 5.0;
const MOVING_THRESHOLD: f32 = 0.7;
const MOVING_THRESHOLD_SQR: f32 = MOVING_THRESHOLD * MOVING_THRESHOLD;

pub struct FigureMgr {
    model_cache: FigureModelCache,
    critter_model_cache: FigureModelCache<CritterSkeleton>,
    quadruped_small_model_cache: FigureModelCache<QuadrupedSmallSkeleton>,
    quadruped_medium_model_cache: FigureModelCache<QuadrupedMediumSkeleton>,
    bird_medium_model_cache: FigureModelCache<BirdMediumSkeleton>,
    bird_small_model_cache: FigureModelCache<BirdSmallSkeleton>,
    dragon_model_cache: FigureModelCache<DragonSkeleton>,
    fish_medium_model_cache: FigureModelCache<FishMediumSkeleton>,
    fish_small_model_cache: FigureModelCache<FishSmallSkeleton>,
    biped_large_model_cache: FigureModelCache<BipedLargeSkeleton>,
    character_states: HashMap<EcsEntity, FigureState<CharacterSkeleton>>,
    quadruped_small_states: HashMap<EcsEntity, FigureState<QuadrupedSmallSkeleton>>,
    quadruped_medium_states: HashMap<EcsEntity, FigureState<QuadrupedMediumSkeleton>>,
    bird_medium_states: HashMap<EcsEntity, FigureState<BirdMediumSkeleton>>,
    fish_medium_states: HashMap<EcsEntity, FigureState<FishMediumSkeleton>>,
    critter_states: HashMap<EcsEntity, FigureState<CritterSkeleton>>,
    dragon_states: HashMap<EcsEntity, FigureState<DragonSkeleton>>,
    bird_small_states: HashMap<EcsEntity, FigureState<BirdSmallSkeleton>>,
    fish_small_states: HashMap<EcsEntity, FigureState<FishSmallSkeleton>>,
    biped_large_states: HashMap<EcsEntity, FigureState<BipedLargeSkeleton>>,
    object_states: HashMap<EcsEntity, FigureState<ObjectSkeleton>>,
}

impl FigureMgr {
    pub fn new() -> Self {
        Self {
            model_cache: FigureModelCache::new(),
            critter_model_cache: FigureModelCache::new(),
            quadruped_small_model_cache: FigureModelCache::new(),
            quadruped_medium_model_cache: FigureModelCache::new(),
            bird_medium_model_cache: FigureModelCache::new(),
            bird_small_model_cache: FigureModelCache::new(),
            dragon_model_cache: FigureModelCache::new(),
            fish_medium_model_cache: FigureModelCache::new(),
            fish_small_model_cache: FigureModelCache::new(),
            biped_large_model_cache: FigureModelCache::new(),
            character_states: HashMap::new(),
            quadruped_small_states: HashMap::new(),
            quadruped_medium_states: HashMap::new(),
            bird_medium_states: HashMap::new(),
            fish_medium_states: HashMap::new(),
            critter_states: HashMap::new(),
            dragon_states: HashMap::new(),
            bird_small_states: HashMap::new(),
            fish_small_states: HashMap::new(),
            biped_large_states: HashMap::new(),
            object_states: HashMap::new(),
        }
    }

    pub fn clean(&mut self, tick: u64) {
        self.model_cache.clean(tick);
        self.critter_model_cache.clean(tick);
        self.quadruped_small_model_cache.clean(tick);
        self.quadruped_medium_model_cache.clean(tick);
        self.bird_medium_model_cache.clean(tick);
        self.bird_small_model_cache.clean(tick);
        self.dragon_model_cache.clean(tick);
        self.fish_medium_model_cache.clean(tick);
        self.fish_small_model_cache.clean(tick);
        self.biped_large_model_cache.clean(tick);
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, scene_data: &SceneData, camera: &Camera) {
        let state = scene_data.state;
        let time = state.get_time();
        let tick = scene_data.tick;
        let ecs = state.ecs();
        let view_distance = scene_data.view_distance;
        let dt = state.get_delta_time();
        let frustum = camera.frustum();
        // Get player position.
        let player_pos = ecs
            .read_storage::<Pos>()
            .get(scene_data.player_entity)
            .map_or(Vec3::zero(), |pos| pos.0);

        for (
            i,
            (
                entity,
                pos,
                interpolated,
                vel,
                scale,
                body,
                character,
                last_character,
                physics,
                stats,
                loadout,
            ),
        ) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            ecs.read_storage::<Interpolated>().maybe(),
            &ecs.read_storage::<Vel>(),
            ecs.read_storage::<Scale>().maybe(),
            &ecs.read_storage::<Body>(),
            ecs.read_storage::<CharacterState>().maybe(),
            ecs.read_storage::<Last<CharacterState>>().maybe(),
            &ecs.read_storage::<PhysicsState>(),
            ecs.read_storage::<Stats>().maybe(),
            ecs.read_storage::<Loadout>().maybe(),
        )
            .join()
            .enumerate()
        {
            // Maintaining figure data and sending new figure data to the GPU turns out to
            // be a very expensive operation. We want to avoid doing it as much
            // as possible, so we make the assumption that players don't care so
            // much about the update *rate* for far away things. As the entity
            // goes further and further away, we start to 'skip' update ticks.
            // TODO: Investigate passing the velocity into the shader so we can at least
            // interpolate motion
            const MIN_PERFECT_RATE_DIST: f32 = 50.0;
            if (i as u64 + tick)
                % (1 + ((pos.0.distance_squared(camera.get_focus_pos()).powf(0.25)
                    - MIN_PERFECT_RATE_DIST.powf(0.5))
                .max(0.0)
                    / 3.0) as u64)
                != 0
            {
                continue;
            }

            let is_player = scene_data.player_entity == entity;

            let (pos, ori) = interpolated
                .map(|i| (Pos(i.pos), *i.ori))
                .unwrap_or((*pos, Vec3::unit_y()));

            // Don't process figures outside the vd
            let vd_frac = Vec2::from(pos.0 - player_pos)
                .map2(TerrainChunk::RECT_SIZE, |d: f32, sz| {
                    d.abs() as f32 / sz as f32
                })
                .magnitude()
                / view_distance as f32;
            // Keep from re-adding/removing entities on the border of the vd
            if vd_frac > 1.2 {
                match body {
                    Body::Humanoid(_) => {
                        self.character_states.remove(&entity);
                    },
                    Body::QuadrupedSmall(_) => {
                        self.quadruped_small_states.remove(&entity);
                    },
                    Body::QuadrupedMedium(_) => {
                        self.quadruped_medium_states.remove(&entity);
                    },
                    Body::BirdMedium(_) => {
                        self.bird_medium_states.remove(&entity);
                    },
                    Body::FishMedium(_) => {
                        self.fish_medium_states.remove(&entity);
                    },
                    Body::Critter(_) => {
                        self.critter_states.remove(&entity);
                    },
                    Body::Dragon(_) => {
                        self.dragon_states.remove(&entity);
                    },
                    Body::BirdSmall(_) => {
                        self.bird_small_states.remove(&entity);
                    },
                    Body::FishSmall(_) => {
                        self.fish_small_states.remove(&entity);
                    },
                    Body::BipedLarge(_) => {
                        self.biped_large_states.remove(&entity);
                    },
                    Body::Object(_) => {
                        self.object_states.remove(&entity);
                    },
                }
                continue;
            } else if vd_frac > 1.0 {
                match body {
                    Body::Humanoid(_) => {
                        self.character_states
                            .get_mut(&entity)
                            .map(|state| state.visible = false);
                    },
                    Body::QuadrupedSmall(_) => {
                        self.quadruped_small_states
                            .get_mut(&entity)
                            .map(|state| state.visible = false);
                    },
                    Body::QuadrupedMedium(_) => {
                        self.quadruped_medium_states
                            .get_mut(&entity)
                            .map(|state| state.visible = false);
                    },
                    Body::BirdMedium(_) => {
                        self.bird_medium_states
                            .get_mut(&entity)
                            .map(|state| state.visible = false);
                    },
                    Body::FishMedium(_) => {
                        self.fish_medium_states
                            .get_mut(&entity)
                            .map(|state| state.visible = false);
                    },
                    Body::Critter(_) => {
                        self.critter_states
                            .get_mut(&entity)
                            .map(|state| state.visible = false);
                    },
                    Body::Dragon(_) => {
                        self.dragon_states
                            .get_mut(&entity)
                            .map(|state| state.visible = false);
                    },
                    Body::BirdSmall(_) => {
                        self.bird_small_states
                            .get_mut(&entity)
                            .map(|state| state.visible = false);
                    },
                    Body::FishSmall(_) => {
                        self.fish_small_states
                            .get_mut(&entity)
                            .map(|state| state.visible = false);
                    },
                    Body::BipedLarge(_) => {
                        self.biped_large_states
                            .get_mut(&entity)
                            .map(|state| state.visible = false);
                    },
                    Body::Object(_) => {
                        self.object_states
                            .get_mut(&entity)
                            .map(|state| state.visible = false);
                    },
                }
                continue;
            }

            // Don't process figures outside the frustum spectrum
            let (in_frustum, lpindex) =
                BoundingSphere::new(pos.0.into_array(), scale.unwrap_or(&Scale(1.0)).0 * 2.0)
                    .coherent_test_against_frustum(
                        &frustum,
                        match body {
                            Body::Humanoid(_) => self
                                .character_states
                                .get(&entity)
                                .map(|state| state.lpindex),
                            Body::QuadrupedSmall(_) => self
                                .quadruped_small_states
                                .get(&entity)
                                .map(|state| state.lpindex),
                            Body::QuadrupedMedium(_) => self
                                .quadruped_medium_states
                                .get(&entity)
                                .map(|state| state.lpindex),
                            Body::BirdMedium(_) => self
                                .bird_medium_states
                                .get(&entity)
                                .map(|state| state.lpindex),
                            Body::FishMedium(_) => self
                                .fish_medium_states
                                .get(&entity)
                                .map(|state| state.lpindex),
                            Body::Critter(_) => {
                                self.critter_states.get(&entity).map(|state| state.lpindex)
                            },
                            Body::Dragon(_) => {
                                self.dragon_states.get(&entity).map(|state| state.lpindex)
                            },
                            Body::BirdSmall(_) => self
                                .bird_small_states
                                .get(&entity)
                                .map(|state| state.lpindex),
                            Body::FishSmall(_) => self
                                .fish_small_states
                                .get(&entity)
                                .map(|state| state.lpindex),
                            Body::BipedLarge(_) => self
                                .biped_large_states
                                .get(&entity)
                                .map(|state| state.lpindex),
                            Body::Object(_) => {
                                self.object_states.get(&entity).map(|state| state.lpindex)
                            },
                        }
                        .unwrap_or(0),
                    );

            if !in_frustum {
                match body {
                    Body::Humanoid(_) => {
                        self.character_states.get_mut(&entity).map(|state| {
                            state.lpindex = lpindex;
                            state.visible = false
                        });
                    },
                    Body::QuadrupedSmall(_) => {
                        self.quadruped_small_states.get_mut(&entity).map(|state| {
                            state.lpindex = lpindex;
                            state.visible = false
                        });
                    },
                    Body::QuadrupedMedium(_) => {
                        self.quadruped_medium_states.get_mut(&entity).map(|state| {
                            state.lpindex = lpindex;
                            state.visible = false
                        });
                    },
                    Body::BirdMedium(_) => {
                        self.bird_medium_states.get_mut(&entity).map(|state| {
                            state.lpindex = lpindex;
                            state.visible = false
                        });
                    },
                    Body::FishMedium(_) => {
                        self.fish_medium_states.get_mut(&entity).map(|state| {
                            state.lpindex = lpindex;
                            state.visible = false
                        });
                    },
                    Body::Critter(_) => {
                        self.critter_states.get_mut(&entity).map(|state| {
                            state.lpindex = lpindex;
                            state.visible = false
                        });
                    },
                    Body::Dragon(_) => {
                        self.dragon_states.get_mut(&entity).map(|state| {
                            state.lpindex = lpindex;
                            state.visible = false
                        });
                    },
                    Body::BirdSmall(_) => {
                        self.bird_small_states.get_mut(&entity).map(|state| {
                            state.lpindex = lpindex;
                            state.visible = false
                        });
                    },
                    Body::FishSmall(_) => {
                        self.fish_small_states.get_mut(&entity).map(|state| {
                            state.lpindex = lpindex;
                            state.visible = false
                        });
                    },
                    Body::BipedLarge(_) => {
                        self.biped_large_states.get_mut(&entity).map(|state| {
                            state.lpindex = lpindex;
                            state.visible = false
                        });
                    },
                    Body::Object(_) => {
                        self.object_states.get_mut(&entity).map(|state| {
                            state.lpindex = lpindex;
                            state.visible = false
                        });
                    },
                }
            }

            // Change in health as color!
            let col = stats
                .map(|s| {
                    Rgba::broadcast(1.0)
                        + Rgba::new(2.0, 2.0, 2., 0.00).map(|c| {
                            (c / (1.0 + DAMAGE_FADE_COEFFICIENT * s.health.last_change.0)) as f32
                        })
                })
                .unwrap_or(Rgba::broadcast(1.0));

            let scale = scale.map(|s| s.0).unwrap_or(1.0);

            let mut state_animation_rate = 1.0;

            let active_item_kind = loadout
                .and_then(|l| l.active_item.as_ref())
                .map(|i| &i.item.kind);
            let active_tool_kind = if let Some(ItemKind::Tool(tool)) = active_item_kind {
                Some(tool.kind)
            } else {
                None
            };

            match body {
                Body::Humanoid(_) => {
                    let skeleton_attr = &self
                        .model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            CameraMode::default(),
                            None,
                        )
                        .1;

                    let state = self
                        .character_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, CharacterSkeleton::new()));
                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground,
                        vel.0.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_fluid,                                 // In water
                    ) {
                        // Standing
                        (true, false, _) => anim::character::StandAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (active_tool_kind, time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, _) => anim::character::RunAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (active_tool_kind, vel.0, ori, state.last_ori, time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::character::JumpAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (active_tool_kind, time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Swim
                        (false, _, true) => anim::character::SwimAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (active_tool_kind, vel.0, ori.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::Roll { .. } => {
                            anim::character::RollAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, ori, state.last_ori, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicMelee(_) => {
                            anim::character::AlphaAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicRanged(data) => {
                            if data.exhausted {
                                anim::character::ShootAnimation::update_skeleton(
                                    &target_base,
                                    (active_tool_kind, vel.0.magnitude(), time),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            } else {
                                anim::character::ChargeAnimation::update_skeleton(
                                    &target_base,
                                    (active_tool_kind, vel.0.magnitude(), time),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            }
                        },
                        CharacterState::Boost(_) => {
                            anim::character::AlphaAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(_) => {
                            anim::character::DashAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::TripleStrike(s) => match s.stage {
                            triple_strike::Stage::First => {
                                anim::character::AlphaAnimation::update_skeleton(
                                    &target_base,
                                    (active_tool_kind, vel.0.magnitude(), time),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            },
                            triple_strike::Stage::Second => {
                                anim::character::SpinAnimation::update_skeleton(
                                    &target_base,
                                    (active_tool_kind, time),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            },
                            triple_strike::Stage::Third => {
                                anim::character::BetaAnimation::update_skeleton(
                                    &target_base,
                                    (active_tool_kind, vel.0.magnitude(), time),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            },
                        },
                        CharacterState::BasicBlock { .. } => {
                            anim::character::BlockIdleAnimation::update_skeleton(
                                &CharacterSkeleton::new(),
                                (active_tool_kind, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        /*
                        CharacterState::Charge(_) => {
                            anim::character::ChargeAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        }*/
                        CharacterState::Equipping { .. } => {
                            anim::character::EquipAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Wielding { .. } => {
                            anim::character::WieldAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Glide { .. } => {
                            anim::character::GlidingAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, vel.0, ori, state.last_ori, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Climb { .. } => {
                            anim::character::ClimbAnimation::update_skeleton(
                                &CharacterSkeleton::new(),
                                (active_tool_kind, vel.0, ori, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Sit { .. } => {
                            anim::character::SitAnimation::update_skeleton(
                                &CharacterSkeleton::new(),
                                (active_tool_kind, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        _ => target_base,
                    };

                    state.skeleton.interpolate(&target_bones, dt);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        lpindex,
                        true,
                        is_player,
                    );
                },
                Body::QuadrupedSmall(_) => {
                    let skeleton_attr = &self
                        .quadruped_small_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            CameraMode::default(),
                            None,
                        )
                        .1;

                    let state = self
                        .quadruped_small_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, QuadrupedSmallSkeleton::new())
                        });

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground,
                        vel.0.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_fluid,                                 // In water
                    ) {
                        // Standing
                        (true, false, false) => {
                            anim::quadruped_small::IdleAnimation::update_skeleton(
                                &QuadrupedSmallSkeleton::new(),
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Running
                        (true, true, false) => {
                            anim::quadruped_small::RunAnimation::update_skeleton(
                                &QuadrupedSmallSkeleton::new(),
                                (vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // In air
                        (false, _, false) => anim::quadruped_small::JumpAnimation::update_skeleton(
                            &QuadrupedSmallSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        lpindex,
                        true,
                        is_player,
                    );
                },
                Body::QuadrupedMedium(_) => {
                    let skeleton_attr = &self
                        .quadruped_medium_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            CameraMode::default(),
                            None,
                        )
                        .1;

                    let state = self
                        .quadruped_medium_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, QuadrupedMediumSkeleton::new())
                        });

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground,
                        vel.0.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_fluid,                                 // In water
                    ) {
                        // Standing
                        (true, false, false) => {
                            anim::quadruped_medium::IdleAnimation::update_skeleton(
                                &QuadrupedMediumSkeleton::new(),
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Running
                        (true, true, false) => {
                            anim::quadruped_medium::RunAnimation::update_skeleton(
                                &QuadrupedMediumSkeleton::new(),
                                (vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // In air
                        (false, _, false) => {
                            anim::quadruped_medium::JumpAnimation::update_skeleton(
                                &QuadrupedMediumSkeleton::new(),
                                (vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        lpindex,
                        true,
                        is_player,
                    );
                },
                Body::BirdMedium(_) => {
                    let skeleton_attr = &self
                        .bird_medium_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            CameraMode::default(),
                            None,
                        )
                        .1;

                    let state = self
                        .bird_medium_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, BirdMediumSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground,
                        vel.0.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_fluid,                                 // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::bird_medium::IdleAnimation::update_skeleton(
                            &BirdMediumSkeleton::new(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::bird_medium::RunAnimation::update_skeleton(
                            &BirdMediumSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::bird_medium::JumpAnimation::update_skeleton(
                            &BirdMediumSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        lpindex,
                        true,
                        is_player,
                    );
                },
                Body::FishMedium(_) => {
                    let skeleton_attr = &self
                        .fish_medium_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            CameraMode::default(),
                            None,
                        )
                        .1;

                    let state = self
                        .fish_medium_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, FishMediumSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground,
                        vel.0.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_fluid,                                 // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::fish_medium::IdleAnimation::update_skeleton(
                            &FishMediumSkeleton::new(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::fish_medium::RunAnimation::update_skeleton(
                            &FishMediumSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::fish_medium::JumpAnimation::update_skeleton(
                            &FishMediumSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        lpindex,
                        true,
                        is_player,
                    );
                },
                Body::Dragon(_) => {
                    let skeleton_attr = &self
                        .dragon_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            CameraMode::default(),
                            None,
                        )
                        .1;

                    let state = self
                        .dragon_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, DragonSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground,
                        vel.0.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_fluid,                                 // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::dragon::IdleAnimation::update_skeleton(
                            &DragonSkeleton::new(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::dragon::RunAnimation::update_skeleton(
                            &DragonSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::dragon::JumpAnimation::update_skeleton(
                            &DragonSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        lpindex,
                        true,
                        is_player,
                    );
                },
                Body::Critter(_) => {
                    let skeleton_attr = &self
                        .critter_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            CameraMode::default(),
                            None,
                        )
                        .1;

                    let state = self
                        .critter_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, CritterSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground,
                        vel.0.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_fluid,                                 // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::critter::IdleAnimation::update_skeleton(
                            &CritterSkeleton::new(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::critter::RunAnimation::update_skeleton(
                            &CritterSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::critter::JumpAnimation::update_skeleton(
                            &CritterSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        lpindex,
                        true,
                        is_player,
                    );
                },
                Body::BirdSmall(_) => {
                    let skeleton_attr = &self
                        .bird_small_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            CameraMode::default(),
                            None,
                        )
                        .1;

                    let state = self
                        .bird_small_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, BirdSmallSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground,
                        vel.0.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_fluid,                                 // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::bird_small::IdleAnimation::update_skeleton(
                            &BirdSmallSkeleton::new(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::bird_small::RunAnimation::update_skeleton(
                            &BirdSmallSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::bird_small::JumpAnimation::update_skeleton(
                            &BirdSmallSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        lpindex,
                        true,
                        is_player,
                    );
                },
                Body::FishSmall(_) => {
                    let skeleton_attr = &self
                        .fish_small_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            CameraMode::default(),
                            None,
                        )
                        .1;

                    let state = self
                        .fish_small_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, FishSmallSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground,
                        vel.0.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_fluid,                                 // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::fish_small::IdleAnimation::update_skeleton(
                            &FishSmallSkeleton::new(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::fish_small::RunAnimation::update_skeleton(
                            &FishSmallSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::fish_small::JumpAnimation::update_skeleton(
                            &FishSmallSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        lpindex,
                        true,
                        is_player,
                    );
                },
                Body::BipedLarge(_) => {
                    let skeleton_attr = &self
                        .biped_large_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            CameraMode::default(),
                            None,
                        )
                        .1;

                    let state = self
                        .biped_large_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, BipedLargeSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.same_variant(&last_character.0) {
                        state.state_time = 0.0;
                    }

                    let target_base = match (
                        physics.on_ground,
                        vel.0.magnitude_squared() > MOVING_THRESHOLD_SQR, // Moving
                        physics.in_fluid,                                 // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::biped_large::IdleAnimation::update_skeleton(
                            &BipedLargeSkeleton::new(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::biped_large::RunAnimation::update_skeleton(
                            &BipedLargeSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::biped_large::JumpAnimation::update_skeleton(
                            &BipedLargeSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        lpindex,
                        true,
                        is_player,
                    );
                },
                Body::Object(_) => {
                    let state = self
                        .object_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, ObjectSkeleton::new()));

                    state.skeleton = state.skeleton_mut().clone();
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        lpindex,
                        true,
                        is_player,
                    );
                },
            }
        }

        // Clear states that have deleted entities.
        self.character_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.quadruped_small_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.quadruped_medium_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.bird_medium_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.fish_medium_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.critter_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.dragon_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.bird_small_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.fish_small_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.biped_large_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.object_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
    }

    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        state: &State,
        player_entity: EcsEntity,
        tick: u64,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
        camera: &Camera,
        figure_lod_render_distance: f32,
    ) {
        let ecs = state.ecs();

        let character_state_storage = state.read_storage::<common::comp::CharacterState>();
        let character_state = character_state_storage.get(player_entity);

        for (entity, pos, _, body, _, loadout, _) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            ecs.read_storage::<Ori>().maybe(),
            &ecs.read_storage::<Body>(),
            ecs.read_storage::<Stats>().maybe(),
            ecs.read_storage::<Loadout>().maybe(),
            ecs.read_storage::<Scale>().maybe(),
        )
            .join()
        // Don't render dead entities
        .filter(|(_, _, _, _, stats, _, _)| stats.map_or(true, |s| !s.is_dead))
        {
            let is_player = entity == player_entity;

            if !is_player {
                self.render_figure(
                    renderer,
                    tick,
                    globals,
                    lights,
                    shadows,
                    camera,
                    character_state,
                    entity,
                    body,
                    loadout,
                    false,
                    pos.0,
                    figure_lod_render_distance,
                );
            }
        }
    }

    pub fn render_player(
        &mut self,
        renderer: &mut Renderer,
        state: &State,
        player_entity: EcsEntity,
        tick: u64,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
        camera: &Camera,
        figure_lod_render_distance: f32,
    ) {
        let ecs = state.ecs();

        let character_state_storage = state.read_storage::<common::comp::CharacterState>();
        let character_state = character_state_storage.get(player_entity);

        if let (Some(pos), Some(body)) = (
            ecs.read_storage::<Pos>().get(player_entity),
            ecs.read_storage::<Body>().get(player_entity),
        ) {
            let stats_storage = state.read_storage::<Stats>();
            let stats = stats_storage.get(player_entity);

            if stats.map_or(false, |s| s.is_dead) {
                return;
            }

            let loadout_storage = ecs.read_storage::<Loadout>();
            let loadout = loadout_storage.get(player_entity);

            self.render_figure(
                renderer,
                tick,
                globals,
                lights,
                shadows,
                camera,
                character_state,
                player_entity,
                body,
                loadout,
                true,
                pos.0,
                figure_lod_render_distance,
            );
        }
    }

    fn render_figure(
        &mut self,
        renderer: &mut Renderer,
        tick: u64,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
        camera: &Camera,
        character_state: Option<&CharacterState>,
        entity: EcsEntity,
        body: &Body,
        loadout: Option<&Loadout>,
        is_player: bool,
        pos: Vec3<f32>,
        figure_lod_render_distance: f32,
    ) {
        let player_camera_mode = if is_player {
            camera.get_mode()
        } else {
            CameraMode::default()
        };
        let character_state = if is_player { character_state } else { None };

        let FigureMgr {
            model_cache,
            critter_model_cache,
            quadruped_small_model_cache,
            quadruped_medium_model_cache,
            bird_medium_model_cache,
            bird_small_model_cache,
            dragon_model_cache,
            fish_medium_model_cache,
            fish_small_model_cache,
            biped_large_model_cache,
            character_states,
            quadruped_small_states,
            quadruped_medium_states,
            bird_medium_states,
            fish_medium_states,
            critter_states,
            dragon_states,
            bird_small_states,
            fish_small_states,
            biped_large_states,
            object_states,
        } = self;
        if let Some((locals, bone_consts, model)) = match body {
            Body::Humanoid(_) => character_states
                .get(&entity)
                .filter(|state| state.visible)
                .map(|state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        &model_cache
                            .get_or_create_model(
                                renderer,
                                *body,
                                loadout,
                                tick,
                                player_camera_mode,
                                character_state,
                            )
                            .0,
                    )
                }),
            Body::QuadrupedSmall(_) => quadruped_small_states.get(&entity).map(|state| {
                (
                    state.locals(),
                    state.bone_consts(),
                    &quadruped_small_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        )
                        .0,
                )
            }),
            Body::QuadrupedMedium(_) => quadruped_medium_states.get(&entity).map(|state| {
                (
                    state.locals(),
                    state.bone_consts(),
                    &quadruped_medium_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        )
                        .0,
                )
            }),
            Body::BirdMedium(_) => bird_medium_states.get(&entity).map(|state| {
                (
                    state.locals(),
                    state.bone_consts(),
                    &bird_medium_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        )
                        .0,
                )
            }),
            Body::FishMedium(_) => fish_medium_states.get(&entity).map(|state| {
                (
                    state.locals(),
                    state.bone_consts(),
                    &fish_medium_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        )
                        .0,
                )
            }),
            Body::Critter(_) => critter_states.get(&entity).map(|state| {
                (
                    state.locals(),
                    state.bone_consts(),
                    &critter_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        )
                        .0,
                )
            }),
            Body::Dragon(_) => dragon_states.get(&entity).map(|state| {
                (
                    state.locals(),
                    state.bone_consts(),
                    &dragon_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        )
                        .0,
                )
            }),
            Body::BirdSmall(_) => bird_small_states.get(&entity).map(|state| {
                (
                    state.locals(),
                    state.bone_consts(),
                    &bird_small_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        )
                        .0,
                )
            }),
            Body::FishSmall(_) => fish_small_states.get(&entity).map(|state| {
                (
                    state.locals(),
                    state.bone_consts(),
                    &fish_small_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        )
                        .0,
                )
            }),
            Body::BipedLarge(_) => biped_large_states.get(&entity).map(|state| {
                (
                    state.locals(),
                    state.bone_consts(),
                    &biped_large_model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        )
                        .0,
                )
            }),
            Body::Object(_) => object_states.get(&entity).map(|state| {
                (
                    state.locals(),
                    state.bone_consts(),
                    &model_cache
                        .get_or_create_model(
                            renderer,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        )
                        .0,
                )
            }),
        } {
            let figure_low_detail_distance = figure_lod_render_distance * 0.75;
            let figure_mid_detail_distance = figure_lod_render_distance * 0.5;

            let model = if pos.distance_squared(camera.get_focus_pos())
                > figure_low_detail_distance.powf(2.0)
            {
                &model[2]
            } else if pos.distance_squared(camera.get_focus_pos())
                > figure_mid_detail_distance.powf(2.0)
            {
                &model[1]
            } else {
                &model[0]
            };

            if is_player {
                renderer.render_player(model, globals, locals, bone_consts, lights, shadows);
                renderer.render_player_shadow(model, globals, locals, bone_consts, lights, shadows);
            } else {
                renderer.render_figure(model, globals, locals, bone_consts, lights, shadows);
            }
        } else {
            trace!("Body has no saved figure");
        }
    }

    pub fn figure_count(&self) -> usize {
        self.character_states.len()
            + self.quadruped_small_states.len()
            + self.character_states.len()
            + self.quadruped_medium_states.len()
            + self.bird_medium_states.len()
            + self.fish_medium_states.len()
            + self.critter_states.len()
            + self.dragon_states.len()
            + self.bird_small_states.len()
            + self.fish_small_states.len()
            + self.biped_large_states.len()
            + self.object_states.len()
    }

    pub fn figure_count_visible(&self) -> usize {
        self.character_states
            .iter()
            .filter(|(_, c)| c.visible)
            .count()
            + self
                .quadruped_small_states
                .iter()
                .filter(|(_, c)| c.visible)
                .count()
            + self
                .quadruped_medium_states
                .iter()
                .filter(|(_, c)| c.visible)
                .count()
            + self
                .bird_medium_states
                .iter()
                .filter(|(_, c)| c.visible)
                .count()
            + self
                .critter_states
                .iter()
                .filter(|(_, c)| c.visible)
                .count()
            + self.dragon_states.iter().filter(|(_, c)| c.visible).count()
            + self
                .fish_medium_states
                .iter()
                .filter(|(_, c)| c.visible)
                .count()
            + self
                .bird_small_states
                .iter()
                .filter(|(_, c)| c.visible)
                .count()
            + self
                .fish_small_states
                .iter()
                .filter(|(_, c)| c.visible)
                .count()
            + self
                .biped_large_states
                .iter()
                .filter(|(_, c)| c.visible)
                .count()
            + self.object_states.iter().filter(|(_, c)| c.visible).count()
    }
}

pub struct FigureState<S: Skeleton> {
    bone_consts: Consts<FigureBoneData>,
    locals: Consts<FigureLocals>,
    state_time: f64,
    skeleton: S,
    last_ori: Vec3<f32>,
    lpindex: u8,
    visible: bool,
}

impl<S: Skeleton> FigureState<S> {
    pub fn new(renderer: &mut Renderer, skeleton: S) -> Self {
        Self {
            bone_consts: renderer
                .create_consts(&skeleton.compute_matrices())
                .unwrap(),
            locals: renderer.create_consts(&[FigureLocals::default()]).unwrap(),
            state_time: 0.0,
            skeleton,
            last_ori: Vec3::zero(),
            lpindex: 0,
            visible: false,
        }
    }

    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        pos: Vec3<f32>,
        ori: Vec3<f32>,
        scale: f32,
        col: Rgba<f32>,
        dt: f32,
        state_animation_rate: f32,
        lpindex: u8,
        visible: bool,
        is_player: bool,
    ) {
        self.visible = visible;
        self.lpindex = lpindex;
        // What is going on here?
        // (note: that ori is now the slerped ori)
        self.last_ori = Lerp::lerp(self.last_ori, ori, 15.0 * dt);

        self.state_time += (dt * state_animation_rate) as f64;

        let mat = Mat4::<f32>::identity()
            * Mat4::translation_3d(pos)
            * Mat4::rotation_z(-ori.x.atan2(ori.y))
            * Mat4::rotation_x(ori.z.atan2(Vec2::from(ori).magnitude()))
            * Mat4::scaling_3d(Vec3::from(0.8 * scale));

        let locals = FigureLocals::new(mat, col, is_player);
        renderer.update_consts(&mut self.locals, &[locals]).unwrap();

        renderer
            .update_consts(
                &mut self.bone_consts,
                &self.skeleton.compute_matrices()[0..self.skeleton.bone_count()],
            )
            .unwrap();
    }

    pub fn locals(&self) -> &Consts<FigureLocals> { &self.locals }

    pub fn bone_consts(&self) -> &Consts<FigureBoneData> { &self.bone_consts }

    pub fn skeleton_mut(&mut self) -> &mut S { &mut self.skeleton }
}
