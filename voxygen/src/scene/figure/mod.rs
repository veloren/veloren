mod cache;
pub mod load;

pub use cache::FigureModelCache;
pub use load::load_mesh; // TODO: Don't make this public.

use crate::{
    ecs::comp::Interpolated,
    mesh::greedy::GreedyMesh,
    render::{
        self, BoneMeshes, ColLightFmt, Consts, FigureBoneData, FigureLocals, FigureModel, Globals,
        Light, RenderError, Renderer, Shadow, ShadowLocals, ShadowPipeline, Texture,
    },
    scene::{
        camera::{Camera, CameraMode},
        LodData, SceneData,
    },
};
use anim::{
    biped_large::BipedLargeSkeleton, bird_medium::BirdMediumSkeleton,
    bird_small::BirdSmallSkeleton, character::CharacterSkeleton, critter::CritterSkeleton,
    dragon::DragonSkeleton, fish_medium::FishMediumSkeleton, fish_small::FishSmallSkeleton,
    golem::GolemSkeleton, object::ObjectSkeleton, quadruped_medium::QuadrupedMediumSkeleton,
    quadruped_small::QuadrupedSmallSkeleton, Animation, Skeleton,
};
use common::{
    comp::{
        item::ItemKind, Body, CharacterState, Last, LightAnimation, LightEmitter, Loadout, Ori,
        PhysicsState, Pos, Scale, Stats, Vel,
    },
    state::{DeltaTime, State},
    states::triple_strike,
    terrain::TerrainChunk,
    vol::RectRasterableVol,
};
use core::{
    borrow::Borrow,
    hash::Hash,
    ops::{Deref, DerefMut},
};
use guillotiere::AtlasAllocator;
use hashbrown::HashMap;
use specs::{Entity as EcsEntity, Join, WorldExt};
use treeculler::{BVol, BoundingSphere};
use vek::*;

const DAMAGE_FADE_COEFFICIENT: f64 = 5.0;
const MOVING_THRESHOLD: f32 = 0.7;
const MOVING_THRESHOLD_SQR: f32 = MOVING_THRESHOLD * MOVING_THRESHOLD;

struct FigureMgrStates {
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
    golem_states: HashMap<EcsEntity, FigureState<GolemSkeleton>>,
    object_states: HashMap<EcsEntity, FigureState<ObjectSkeleton>>,
}

impl FigureMgrStates {
    pub fn default() -> Self {
        Self {
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
            golem_states: HashMap::new(),
            object_states: HashMap::new(),
        }
    }

    /* fn get<'a, Q: ?Sized>(&'a self, body: &Body, entity: &Q) -> Option<&'a FigureStateMeta>
    where
        EcsEntity: Borrow<Q>,
        Q: Hash + Eq,
    {
        match body {
            Body::Humanoid(_) => self.character_states.get(&entity).map(Deref::deref),
            Body::QuadrupedSmall(_) => self.quadruped_small_states.get(&entity).map(Deref::deref),
            Body::QuadrupedMedium(_) => self.quadruped_medium_states.get(&entity).map(Deref::deref),
            Body::BirdMedium(_) => self.bird_medium_states.get(&entity).map(Deref::deref),
            Body::FishMedium(_) => self.fish_medium_states.get(&entity).map(Deref::deref),
            Body::Critter(_) => self.critter_states.get(&entity).map(Deref::deref),
            Body::Dragon(_) => self.dragon_states.get(&entity).map(Deref::deref),
            Body::BirdSmall(_) => self.bird_small_states.get(&entity).map(Deref::deref),
            Body::FishSmall(_) => self.fish_small_states.get(&entity).map(Deref::deref),
            Body::BipedLarge(_) => self.biped_large_states.get(&entity).map(Deref::deref),
            Body::Golem(_) => self.golem_states.get(&entity).map(Deref::deref),
            Body::Object(_) => self.object_states.get(&entity).map(Deref::deref),
        }
    } */

    fn get_mut<'a, Q: ?Sized>(
        &'a mut self,
        body: &Body,
        entity: &Q,
    ) -> Option<&'a mut FigureStateMeta>
    where
        EcsEntity: Borrow<Q>,
        Q: Hash + Eq,
    {
        match body {
            Body::Humanoid(_) => self
                .character_states
                .get_mut(&entity)
                .map(DerefMut::deref_mut),
            Body::QuadrupedSmall(_) => self
                .quadruped_small_states
                .get_mut(&entity)
                .map(DerefMut::deref_mut),
            Body::QuadrupedMedium(_) => self
                .quadruped_medium_states
                .get_mut(&entity)
                .map(DerefMut::deref_mut),
            Body::BirdMedium(_) => self
                .bird_medium_states
                .get_mut(&entity)
                .map(DerefMut::deref_mut),
            Body::FishMedium(_) => self
                .fish_medium_states
                .get_mut(&entity)
                .map(DerefMut::deref_mut),
            Body::Critter(_) => self
                .critter_states
                .get_mut(&entity)
                .map(DerefMut::deref_mut),
            Body::Dragon(_) => self.dragon_states.get_mut(&entity).map(DerefMut::deref_mut),
            Body::BirdSmall(_) => self
                .bird_small_states
                .get_mut(&entity)
                .map(DerefMut::deref_mut),
            Body::FishSmall(_) => self
                .fish_small_states
                .get_mut(&entity)
                .map(DerefMut::deref_mut),
            Body::BipedLarge(_) => self
                .biped_large_states
                .get_mut(&entity)
                .map(DerefMut::deref_mut),
            Body::Golem(_) => self.golem_states.get_mut(&entity).map(DerefMut::deref_mut),
            Body::Object(_) => self.object_states.get_mut(&entity).map(DerefMut::deref_mut),
        }
    }

    fn remove<'a, Q: ?Sized>(&'a mut self, body: &Body, entity: &Q) -> Option<FigureStateMeta>
    where
        EcsEntity: Borrow<Q>,
        Q: Hash + Eq,
    {
        match body {
            Body::Humanoid(_) => self.character_states.remove(&entity).map(|e| e.meta),
            Body::QuadrupedSmall(_) => self.quadruped_small_states.remove(&entity).map(|e| e.meta),
            Body::QuadrupedMedium(_) => {
                self.quadruped_medium_states.remove(&entity).map(|e| e.meta)
            },
            Body::BirdMedium(_) => self.bird_medium_states.remove(&entity).map(|e| e.meta),
            Body::FishMedium(_) => self.fish_medium_states.remove(&entity).map(|e| e.meta),
            Body::Critter(_) => self.critter_states.remove(&entity).map(|e| e.meta),
            Body::Dragon(_) => self.dragon_states.remove(&entity).map(|e| e.meta),
            Body::BirdSmall(_) => self.bird_small_states.remove(&entity).map(|e| e.meta),
            Body::FishSmall(_) => self.fish_small_states.remove(&entity).map(|e| e.meta),
            Body::BipedLarge(_) => self.biped_large_states.remove(&entity).map(|e| e.meta),
            Body::Golem(_) => self.golem_states.remove(&entity).map(|e| e.meta),
            Body::Object(_) => self.object_states.remove(&entity).map(|e| e.meta),
        }
    }

    fn retain<'a>(&'a mut self, mut f: impl FnMut(&EcsEntity, &mut FigureStateMeta) -> bool) {
        self.character_states.retain(|k, v| f(k, &mut *v));
        self.quadruped_small_states.retain(|k, v| f(k, &mut *v));
        self.quadruped_medium_states.retain(|k, v| f(k, &mut *v));
        self.bird_medium_states.retain(|k, v| f(k, &mut *v));
        self.fish_medium_states.retain(|k, v| f(k, &mut *v));
        self.critter_states.retain(|k, v| f(k, &mut *v));
        self.dragon_states.retain(|k, v| f(k, &mut *v));
        self.bird_small_states.retain(|k, v| f(k, &mut *v));
        self.fish_small_states.retain(|k, v| f(k, &mut *v));
        self.biped_large_states.retain(|k, v| f(k, &mut *v));
        self.golem_states.retain(|k, v| f(k, &mut *v));
        self.object_states.retain(|k, v| f(k, &mut *v));
    }

    fn count(&self) -> usize {
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
            + self.golem_states.len()
            + self.object_states.len()
    }

    fn count_visible(&self) -> usize {
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
            + self.golem_states.iter().filter(|(_, c)| c.visible).count()
            + self.object_states.iter().filter(|(_, c)| c.visible).count()
    }
}

pub struct FigureMgr {
    col_lights: FigureColLights,
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
    golem_model_cache: FigureModelCache<GolemSkeleton>,
    states: FigureMgrStates,
}

impl FigureMgr {
    pub fn new(renderer: &mut Renderer) -> Self {
        Self {
            col_lights: FigureColLights::new(renderer),
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
            golem_model_cache: FigureModelCache::new(),
            states: FigureMgrStates::default(),
        }
    }

    pub fn col_lights(&self) -> &FigureColLights { &self.col_lights }

    pub fn clean(&mut self, tick: u64) {
        self.model_cache.clean(&mut self.col_lights, tick);
        self.critter_model_cache.clean(&mut self.col_lights, tick);
        self.quadruped_small_model_cache
            .clean(&mut self.col_lights, tick);
        self.quadruped_medium_model_cache
            .clean(&mut self.col_lights, tick);
        self.bird_medium_model_cache
            .clean(&mut self.col_lights, tick);
        self.bird_small_model_cache
            .clean(&mut self.col_lights, tick);
        self.dragon_model_cache.clean(&mut self.col_lights, tick);
        self.fish_medium_model_cache
            .clean(&mut self.col_lights, tick);
        self.fish_small_model_cache
            .clean(&mut self.col_lights, tick);
        self.biped_large_model_cache
            .clean(&mut self.col_lights, tick);
        self.golem_model_cache.clean(&mut self.col_lights, tick);
    }

    #[allow(clippy::redundant_pattern_matching)] // TODO: Pending review in #587
    pub fn update_lighting(&mut self, scene_data: &SceneData) {
        let ecs = scene_data.state.ecs();
        for (entity, light_emitter) in (&ecs.entities(), &ecs.read_storage::<LightEmitter>()).join()
        {
            // Add LightAnimation for objects with a LightEmitter
            let mut anim_storage = ecs.write_storage::<LightAnimation>();
            if let None = anim_storage.get_mut(entity) {
                let anim = LightAnimation {
                    offset: Vec3::zero(), //Vec3::new(0.0, 0.0, 2.0),
                    col: light_emitter.col,
                    strength: 0.0,
                };
                let _ = anim_storage.insert(entity, anim);
            }
        }
        let dt = ecs.fetch::<DeltaTime>().0;
        for (entity, waypoint, light_emitter_opt, light_anim) in (
            &ecs.entities(),
            ecs.read_storage::<common::comp::Waypoint>().maybe(),
            ecs.read_storage::<LightEmitter>().maybe(),
            &mut ecs.write_storage::<LightAnimation>(),
        )
            .join()
        {
            let (target_col, target_strength, flicker, animated) =
                if let Some(emitter) = light_emitter_opt {
                    (
                        emitter.col,
                        if emitter.strength.is_finite() {
                            emitter.strength
                        } else {
                            0.0
                        },
                        emitter.flicker,
                        emitter.animated,
                    )
                } else {
                    (Rgb::zero(), 0.0, 0.0, true)
                };
            if let Some(_) = waypoint {
                light_anim.offset = Vec3::unit_z() * 0.5;
            }
            if let Some(state) = self.states.character_states.get(&entity) {
                light_anim.offset = state.lantern_offset;
            }
            if !light_anim.strength.is_finite() {
                light_anim.strength = 0.0;
            }
            if animated {
                let flicker = (rand::random::<f32>() - 0.5) * flicker / dt.sqrt();
                // Close gap between current and target strength by 95% per second
                let delta = 0.05_f32.powf(dt);
                light_anim.strength =
                    light_anim.strength * delta + (target_strength + flicker) * (1.0 - delta);
                light_anim.col = light_anim.col * delta + target_col * (1.0 - delta)
            } else {
                light_anim.strength = target_strength;
                light_anim.col = target_col;
            }
        }
    }

    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        camera: &Camera,
    ) -> Aabb<f32> {
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
        let mut visible_aabb = Aabb {
            min: player_pos - 2.0,
            max: player_pos + 2.0,
        };

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
            let is_player = scene_data.player_entity == entity;
            let (pos, ori) = interpolated
                .map(|i| (Pos(i.pos), *i.ori))
                .unwrap_or((*pos, Vec3::unit_y()));

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

            // Don't process figures outside the vd
            let vd_frac = Vec2::from(pos.0 - player_pos)
                .map2(TerrainChunk::RECT_SIZE, |d: f32, sz| {
                    d.abs() as f32 / sz as f32
                })
                .magnitude()
                / view_distance as f32;
            // Keep from re-adding/removing entities on the border of the vd
            if vd_frac > 1.2 {
                self.states.remove(body, &entity);
                continue;
            } else if vd_frac > 1.0 {
                self.states
                    .get_mut(body, &entity)
                    .map(|state| state.visible = false);
                continue;
            }

            // Don't display figures outside the frustum spectrum (this is important to do
            // for any figure that potentially casts a shadow, since we use this
            // to estimate bounds for shadow maps).  Currently, we don't do this before the update
            // cull, so it's possible that faraway figures will not shadow correctly until their
            // next update.  For now, we treat this as an acceptable tradeoff.
            let radius = scale.unwrap_or(&Scale(1.0)).0 * 2.0;
            let (in_frustum, lpindex) = if let Some(mut meta) = self.states.get_mut(body, &entity) {
                let (in_frustum, lpindex) = BoundingSphere::new(pos.0.into_array(), radius)
                    .coherent_test_against_frustum(frustum, meta.lpindex);
                meta.visible = in_frustum;
                meta.lpindex = lpindex;
                (in_frustum, lpindex)
            } else {
                (true, 0)
            };
            if in_frustum {
                // Update visible bounds.
                visible_aabb.expand_to_contain(Aabb {
                    min: pos.0 - radius,
                    max: pos.0 + radius,
                });
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

            let second_item_kind = loadout
                .and_then(|l| l.second_item.as_ref())
                .map(|i| &i.item.kind);

            let second_tool_kind = if let Some(ItemKind::Tool(tool)) = second_item_kind {
                Some(tool.kind)
            } else {
                None
            };

            match body {
                Body::Humanoid(_) => {
                    let (model, skeleton_attr) = self.model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        CameraMode::default(),
                        None,
                    );

                    let state = self
                        .states
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
                            (active_tool_kind, second_tool_kind, time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, _) => anim::character::RunAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (
                                active_tool_kind,
                                second_tool_kind,
                                vel.0,
                                ori,
                                state.last_ori,
                                time,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::character::JumpAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (
                                active_tool_kind,
                                second_tool_kind,
                                ori,
                                state.last_ori,
                                time,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Swim
                        (false, _, true) => anim::character::SwimAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (
                                active_tool_kind,
                                second_tool_kind,
                                vel.0,
                                ori,
                                state.last_ori,
                                time,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::Roll { .. } => {
                            anim::character::RollAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    ori,
                                    state.last_ori,
                                    time,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicMelee(_) => {
                            anim::character::AlphaAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, second_tool_kind, vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicRanged(data) => {
                            if data.exhausted {
                                anim::character::ShootAnimation::update_skeleton(
                                    &target_base,
                                    (active_tool_kind, second_tool_kind, vel.0.magnitude(), time),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            } else {
                                anim::character::ChargeAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        active_tool_kind,
                                        second_tool_kind,
                                        vel.0.magnitude(),
                                        ori,
                                        state.last_ori,
                                        time,
                                    ),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            }
                        },
                        CharacterState::Boost(_) => {
                            anim::character::AlphaAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, second_tool_kind, vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(_) => {
                            anim::character::DashAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, second_tool_kind, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::LeapMelee(_) => {
                            anim::character::LeapAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, second_tool_kind, vel.0, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::TripleStrike(s) => match s.stage {
                            triple_strike::Stage::First => {
                                anim::character::AlphaAnimation::update_skeleton(
                                    &target_base,
                                    (active_tool_kind, second_tool_kind, vel.0.magnitude(), time),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            },
                            triple_strike::Stage::Second => {
                                anim::character::SpinAnimation::update_skeleton(
                                    &target_base,
                                    (active_tool_kind, second_tool_kind, time),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            },
                            triple_strike::Stage::Third => {
                                anim::character::BetaAnimation::update_skeleton(
                                    &target_base,
                                    (active_tool_kind, second_tool_kind, vel.0.magnitude(), time),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            },
                        },
                        CharacterState::BasicBlock { .. } => {
                            anim::character::BlockIdleAnimation::update_skeleton(
                                &CharacterSkeleton::new(),
                                (active_tool_kind, second_tool_kind, time),
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
                                (active_tool_kind, second_tool_kind, vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Wielding { .. } => {
                            anim::character::WieldAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, second_tool_kind, vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Glide { .. } => {
                            anim::character::GlidingAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0,
                                    ori,
                                    state.last_ori,
                                    time,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Climb { .. } => {
                            anim::character::ClimbAnimation::update_skeleton(
                                &CharacterSkeleton::new(),
                                (active_tool_kind, second_tool_kind, vel.0, ori, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Sit { .. } => {
                            anim::character::SitAnimation::update_skeleton(
                                &CharacterSkeleton::new(),
                                (active_tool_kind, second_tool_kind, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::GlideWield { .. } => {
                            anim::character::GlideWieldAnimation::update_skeleton(
                                &CharacterSkeleton::new(),
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0,
                                    ori,
                                    state.last_ori,
                                    time,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Dance { .. } => {
                            anim::character::DanceAnimation::update_skeleton(
                                &CharacterSkeleton::new(),
                                (active_tool_kind, second_tool_kind, time),
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
                        &model[0],
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                    );
                },
                Body::QuadrupedSmall(_) => {
                    let (model, skeleton_attr) =
                        self.quadruped_small_model_cache.get_or_create_model(
                            renderer,
                            &mut self.col_lights,
                            *body,
                            loadout,
                            tick,
                            CameraMode::default(),
                            None,
                        );

                    let state = self
                        .states
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
                        &model[0],
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                    );
                },
                Body::QuadrupedMedium(_) => {
                    let (model, skeleton_attr) =
                        self.quadruped_medium_model_cache.get_or_create_model(
                            renderer,
                            &mut self.col_lights,
                            *body,
                            loadout,
                            tick,
                            CameraMode::default(),
                            None,
                        );

                    let state = self
                        .states
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
                        &model[0],
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                    );
                },
                Body::BirdMedium(_) => {
                    let (model, skeleton_attr) = self.bird_medium_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        CameraMode::default(),
                        None,
                    );

                    let state = self
                        .states
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
                        (false, _, false) => anim::bird_medium::FlyAnimation::update_skeleton(
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
                        &model[0],
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                    );
                },
                Body::FishMedium(_) => {
                    let (model, skeleton_attr) = self.fish_medium_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        CameraMode::default(),
                        None,
                    );

                    let state = self
                        .states
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
                        &model[0],
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                    );
                },
                Body::Dragon(_) => {
                    let (model, skeleton_attr) = self.dragon_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        CameraMode::default(),
                        None,
                    );

                    let state = self
                        .states
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
                        (false, _, false) => anim::dragon::FlyAnimation::update_skeleton(
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
                        &model[0],
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                    );
                },
                Body::Critter(_) => {
                    let (model, skeleton_attr) = self.critter_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        CameraMode::default(),
                        None,
                    );

                    let state = self
                        .states
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
                        &model[0],
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                    );
                },
                Body::BirdSmall(_) => {
                    let (model, skeleton_attr) = self.bird_small_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        CameraMode::default(),
                        None,
                    );

                    let state = self
                        .states
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
                        &model[0],
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                    );
                },
                Body::FishSmall(_) => {
                    let (model, skeleton_attr) = self.fish_small_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        CameraMode::default(),
                        None,
                    );

                    let state = self
                        .states
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
                        &model[0],
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                    );
                },
                Body::BipedLarge(_) => {
                    let (model, skeleton_attr) = self.biped_large_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        CameraMode::default(),
                        None,
                    );

                    let state = self
                        .states
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
                        &model[0],
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                    );
                },
                Body::Golem(_) => {
                    let (model, skeleton_attr) = self.golem_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        CameraMode::default(),
                        None,
                    );

                    let state = self
                        .states
                        .golem_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, GolemSkeleton::new()));

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
                        (true, false, false) => anim::golem::IdleAnimation::update_skeleton(
                            &GolemSkeleton::new(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::golem::RunAnimation::update_skeleton(
                            &GolemSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::golem::JumpAnimation::update_skeleton(
                            &GolemSkeleton::new(),
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
                        &model[0],
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                    );
                },
                Body::Object(_) => {
                    let (model, _) = &self.model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        CameraMode::default(),
                        None,
                    );

                    let state = self
                        .states
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
                        &model[0],
                        lpindex,
                        true,
                        is_player,
                        camera,
                    );
                },
            }
        }

        // Update lighting (lanterns) for figures
        self.update_lighting(scene_data);

        // Clear states that have deleted entities.
        self.states
            .retain(|entity, _| ecs.entities().is_alive(*entity));

        visible_aabb
    }

    pub fn render_shadows(
        &mut self,
        renderer: &mut Renderer,
        state: &State,
        tick: u64,
        globals: &Consts<Globals>,
        shadow_mats: &Consts<ShadowLocals>,
        is_daylight: bool,
        _light_data: &[Light],
        camera: &Camera,
        figure_lod_render_distance: f32,
    ) {
        let ecs = state.ecs();

        if is_daylight && renderer.render_mode().shadow == render::ShadowMode::Map {
            (
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
            .for_each(|(entity, pos, _, body, _, loadout, _)| {
                if let Some((locals, bone_consts, model, _)) = self.get_model_for_render(
                    renderer,
                    tick,
                    camera,
                    None,
                    entity,
                    body,
                    loadout,
                    false,
                    pos.0,
                    figure_lod_render_distance,
                    |state| state.visible,
                ) {
                    renderer.render_figure_shadow_directed(
                        model,
                        globals,
                        locals,
                        bone_consts,
                        shadow_mats,
                    );
                }
            });
        }
    }

    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        state: &State,
        player_entity: EcsEntity,
        tick: u64,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
        shadow_mats: &Consts<ShadowLocals>,
        lod: &LodData,
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
                if let Some((locals, bone_consts, model, col_lights)) = self.get_model_for_render(
                    renderer,
                    tick,
                    camera,
                    character_state,
                    entity,
                    body,
                    loadout,
                    false,
                    pos.0,
                    figure_lod_render_distance,
                    |state| state.visible,
                ) {
                    renderer.render_figure(
                        model,
                        &col_lights.col_lights,
                        globals,
                        locals,
                        bone_consts,
                        lights,
                        shadows,
                        shadow_mats,
                        &lod.map,
                        &lod.horizon,
                    );
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn render_player(
        &mut self,
        renderer: &mut Renderer,
        state: &State,
        player_entity: EcsEntity,
        tick: u64,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
        shadow_mats: &Consts<ShadowLocals>,
        lod: &LodData,
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

            if let Some((locals, bone_consts, model, col_lights)) = self.get_model_for_render(
                renderer,
                tick,
                camera,
                character_state,
                player_entity,
                body,
                loadout,
                true,
                pos.0,
                figure_lod_render_distance,
                |state| state.visible,
            ) {
                renderer.render_player(
                    model,
                    &col_lights.col_lights,
                    globals,
                    locals,
                    bone_consts,
                    lights,
                    shadows,
                    shadow_mats,
                    &lod.map,
                    &lod.horizon,
                );
                renderer.render_player_shadow(
                    model,
                    &col_lights.col_lights,
                    globals,
                    locals,
                    bone_consts,
                    lights,
                    shadows,
                    shadow_mats,
                    &lod.map,
                    &lod.horizon,
                );
            }
        }
    }

    /* fn do_models_for_render(
        state: &State,
        player_entity: EcsEntity,
    ) -> impl IntoIterator<> + Clone {
        let ecs = state.ecs();

        (
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
            .for_each(|(entity, pos, _, body, _, loadout, _)| {

            });
    } */

    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    fn get_model_for_render(
        &mut self,
        renderer: &mut Renderer,
        tick: u64,
        camera: &Camera,
        character_state: Option<&CharacterState>,
        entity: EcsEntity,
        body: &Body,
        loadout: Option<&Loadout>,
        is_player: bool,
        // is_shadow: bool,
        pos: Vec3<f32>,
        figure_lod_render_distance: f32,
        filter_state: impl Fn(&FigureStateMeta) -> bool,
    ) -> Option<(
        &Consts<FigureLocals>,
        &Consts<FigureBoneData>,
        &FigureModel,
        &FigureColLights,
    )> {
        let player_camera_mode = if is_player {
            camera.get_mode()
        } else {
            CameraMode::default()
        };
        let focus_pos = camera.get_focus_pos();
        let cam_pos = camera.dependents().cam_pos + focus_pos.map(|e| e.trunc());
        let character_state = if is_player { character_state } else { None };

        let FigureMgr {
            col_lights: ref mut col_lights_,
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
            golem_model_cache,
            states:
                FigureMgrStates {
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
                    golem_states,
                    object_states,
                },
        } = self;
        let col_lights = &mut *col_lights_;
        if let Some((locals, bone_consts, model)) = match body {
            Body::Humanoid(_) => character_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        &model_cache
                            .get_or_create_model(
                                renderer,
                                col_lights,
                                *body,
                                loadout,
                                tick,
                                player_camera_mode,
                                character_state,
                            )
                            .0,
                    )
                }),
            Body::QuadrupedSmall(_) => quadruped_small_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        &quadruped_small_model_cache
                            .get_or_create_model(
                                renderer,
                                col_lights,
                                *body,
                                loadout,
                                tick,
                                player_camera_mode,
                                character_state,
                            )
                            .0,
                    )
                }),
            Body::QuadrupedMedium(_) => quadruped_medium_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        &quadruped_medium_model_cache
                            .get_or_create_model(
                                renderer,
                                col_lights,
                                *body,
                                loadout,
                                tick,
                                player_camera_mode,
                                character_state,
                            )
                            .0,
                    )
                }),
            Body::BirdMedium(_) => bird_medium_states.get(&entity).map(move |state| {
                (
                    state.locals(),
                    state.bone_consts(),
                    &bird_medium_model_cache
                        .get_or_create_model(
                            renderer,
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        )
                        .0,
                )
            }),
            Body::FishMedium(_) => fish_medium_states.get(&entity).map(move |state| {
                (
                    state.locals(),
                    state.bone_consts(),
                    &fish_medium_model_cache
                        .get_or_create_model(
                            renderer,
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        )
                        .0,
                )
            }),
            Body::Critter(_) => critter_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        &critter_model_cache
                            .get_or_create_model(
                                renderer,
                                col_lights,
                                *body,
                                loadout,
                                tick,
                                player_camera_mode,
                                character_state,
                            )
                            .0,
                    )
                }),
            Body::Dragon(_) => dragon_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        &dragon_model_cache
                            .get_or_create_model(
                                renderer,
                                col_lights,
                                *body,
                                loadout,
                                tick,
                                player_camera_mode,
                                character_state,
                            )
                            .0,
                    )
                }),
            Body::BirdSmall(_) => bird_small_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        &bird_small_model_cache
                            .get_or_create_model(
                                renderer,
                                col_lights,
                                *body,
                                loadout,
                                tick,
                                player_camera_mode,
                                character_state,
                            )
                            .0,
                    )
                }),
            Body::FishSmall(_) => fish_small_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        &fish_small_model_cache
                            .get_or_create_model(
                                renderer,
                                col_lights,
                                *body,
                                loadout,
                                tick,
                                player_camera_mode,
                                character_state,
                            )
                            .0,
                    )
                }),
            Body::BipedLarge(_) => biped_large_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        &biped_large_model_cache
                            .get_or_create_model(
                                renderer,
                                col_lights,
                                *body,
                                loadout,
                                tick,
                                player_camera_mode,
                                character_state,
                            )
                            .0,
                    )
                }),
            Body::Golem(_) => golem_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        &golem_model_cache
                            .get_or_create_model(
                                renderer,
                                col_lights,
                                *body,
                                loadout,
                                tick,
                                player_camera_mode,
                                character_state,
                            )
                            .0,
                    )
                }),
            Body::Object(_) => object_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        &model_cache
                            .get_or_create_model(
                                renderer,
                                col_lights,
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

            let model = if pos.distance_squared(cam_pos) > figure_low_detail_distance.powf(2.0) {
                &model[2]
            } else if pos.distance_squared(cam_pos) > figure_mid_detail_distance.powf(2.0) {
                &model[1]
            } else {
                &model[0]
            };

            Some((locals, bone_consts, model, &*col_lights_))
        } else {
            // trace!("Body has no saved figure");
            None
        }
    }

    pub fn figure_count(&self) -> usize { self.states.count() }

    pub fn figure_count_visible(&self) -> usize { self.states.count_visible() }
}

pub struct FigureColLights {
    atlas: AtlasAllocator,
    col_lights: Texture<ColLightFmt>,
}

impl FigureColLights {
    pub fn new(renderer: &mut Renderer) -> Self {
        let (atlas, col_lights) =
            Self::make_atlas(renderer).expect("Failed to create texture atlas for figures");
        Self { atlas, col_lights }
    }

    pub fn texture(&self) -> &Texture<ColLightFmt> { &self.col_lights }

    pub fn create_figure<'a>(
        &mut self,
        renderer: &mut Renderer,
        greedy: GreedyMesh<'a>,
        /* (opaque, shadow) */ (opaque, bounds): BoneMeshes,
    ) -> Result<FigureModel, RenderError> {
        // println!("Figure bounds: {:?}", bounds);
        let (tex, tex_size) = greedy.finalize();
        let atlas = &mut self.atlas;
        let allocation = atlas
            .allocate(guillotiere::Size::new(
                i32::from(tex_size.x),
                i32::from(tex_size.y),
            ))
            .expect("Not yet implemented: allocate new atlas on allocation faillure.");
        // println!("Allocation {:?} for {:?} (original size = {:?}... ugh)",
        // allocation, response.pos, tex_size); NOTE: Cast is safe since the
        // origin was a u16.
        let atlas_offs = Vec2::new(
            allocation.rectangle.min.x as u16,
            allocation.rectangle.min.y as u16,
        );
        if atlas_offs == Vec2::zero() {
            // println!("Model: {:?}", &response.opaque_mesh.vertices());
            // println!("Texture: {:?}", tex);
        }

        /* if let Err(err) = renderer.update_texture(
            &self.col_lights,
            // &col_lights,
            // NOTE: Cast is safe since the origin was a u16.
            atlas_offs.into_array(),
            tex_size.into_array(),
            &tex,
        ) {
            panic!("Ahhh {:?}", err);
            log::warn!("Failed to update texture: {:?}", err);
        } */
        // FIXME: Deal with allocation failure!
        /* renderer.update_texture(
            &self.col_lights,
            // &col_lights,
            // NOTE: Cast is safe since the origin was a u16.
            atlas_offs.into_array(),
            tex_size.into_array(),
            &tex,
        )?; */
        let col_lights = ShadowPipeline::create_col_lights(renderer, (tex, tex_size))?;

        Ok(FigureModel {
            bounds,
            opaque: renderer.create_model(&opaque)?,
            // shadow: renderer.create_model(&shadow)?,
            col_lights,
            allocation,
        })
    }

    fn make_atlas(
        renderer: &mut Renderer,
    ) -> Result<(AtlasAllocator, Texture<ColLightFmt>), RenderError> {
        let max_texture_size = renderer.max_texture_size();
        let atlas_size =
            guillotiere::Size::new(i32::from(max_texture_size), i32::from(max_texture_size));
        // let atlas_size = guillotiere::Size::new(1, 1);
        let atlas = AtlasAllocator::with_options(atlas_size, &guillotiere::AllocatorOptions {
            // TODO: Verify some good empirical constants.
            small_size_threshold: 32,
            large_size_threshold: 256,
            ..guillotiere::AllocatorOptions::default()
        });
        let texture = renderer.create_texture_raw(
            gfx::texture::Kind::D2(
                max_texture_size,
                max_texture_size,
                gfx::texture::AaMode::Single,
            ),
            1 as gfx::texture::Level,
            // gfx::memory::Upload,
            gfx::memory::Bind::SHADER_RESOURCE, /* | gfx::memory::Bind::TRANSFER_DST */
            gfx::memory::Usage::Dynamic,
            (0, 0),
            gfx::format::Swizzle::new(),
            gfx::texture::SamplerInfo::new(
                gfx::texture::FilterMethod::Bilinear,
                gfx::texture::WrapMode::Clamp,
            ),
        )?;
        /* renderer.flush();
        renderer.update_texture(
            &texture,
            [0, 0],
            [max_texture_size, max_texture_size],
            &vec![[0u8; 4]; (usize::from(max_texture_size) * usize::from(max_texture_size))],
            //&[[255u8; 4]; 64 * 64],
            // NOTE: Cast is safe since the origin was a u16.
        )?;
        renderer.flush(); */
        // texture.cleanup();
        // Not sure if this is necessary...
        // renderer.flush();
        // texture.update();
        // // FIXME: Currently, there seems to be a bug where the very first texture
        // update always // fails.  Not sure why, but we currently work around
        // it with a dummy allocation (which we // proceed to leak, in case the
        // bug can return after it's freed). let _ = atlas.allocate(guillotiere:
        // :Size::new(64, 64));
        Ok((atlas, texture))
    }
}

pub struct FigureStateMeta {
    bone_consts: Consts<FigureBoneData>,
    locals: Consts<FigureLocals>,
    lantern_offset: Vec3<f32>,
    state_time: f64,
    last_ori: Vec3<f32>,
    lpindex: u8,
    visible: bool,
}

pub struct FigureState<S> {
    meta: FigureStateMeta,
    skeleton: S,
}

impl<S> Deref for FigureState<S> {
    type Target = FigureStateMeta;

    fn deref(&self) -> &Self::Target { &self.meta }
}

impl<S> DerefMut for FigureState<S> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.meta }
}

impl<S: Skeleton> FigureState<S> {
    pub fn new(renderer: &mut Renderer, skeleton: S) -> Self {
        let (bone_mats, lantern_offset) = skeleton.compute_matrices();
        let bone_consts = figure_bone_data_from_anim(bone_mats, |mat| {
            FigureBoneData::new(mat, mat.map_cols(|c| c.normalized()))
        });
        Self {
            meta: FigureStateMeta {
                bone_consts: renderer.create_consts(&bone_consts).unwrap(),
                locals: renderer.create_consts(&[FigureLocals::default()]).unwrap(),
                lantern_offset,
                state_time: 0.0,
                last_ori: Vec3::zero(),
                lpindex: 0,
                visible: false,
            },
            skeleton,
        }
    }

    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        pos: Vec3<f32>,
        ori: Vec3<f32>,
        scale: f32,
        col: Rgba<f32>,
        dt: f32,
        state_animation_rate: f32,
        model: &FigureModel,
        _lpindex: u8,
        _visible: bool,
        is_player: bool,
        camera: &Camera,
    ) {
        let _frustum = camera.frustum();

        // Approximate as a sphere with radius equal to the
        // largest dimension (if we were exact, it should just be half the largest
        // dimension, but we're not, so we double it and use size() instead of
        // half_size()).
        let radius = model.bounds.half_size().reduce_partial_max();
        let _bounds = BoundingSphere::new(pos.into_array(), scale * 0.8 * radius);

        /* let (in_frustum, lpindex) = bounds.coherent_test_against_frustum(frustum, self.lpindex);
        let visible = visible && in_frustum;

        self.lpindex = lpindex;
        self.visible = visible; */
        // What is going on here?
        // (note: that ori is now the slerped ori)
        self.last_ori = Lerp::lerp(self.last_ori, ori, 15.0 * dt);

        self.state_time += (dt * state_animation_rate) as f64;

        let _focus_off = camera.get_focus_pos().map(|e| e.trunc());
        let mat = Mat4::<f32>::identity()
            // * Mat4::translation_3d(pos - focus_off)
            * Mat4::rotation_z(-ori.x.atan2(ori.y))
            * Mat4::rotation_x(ori.z.atan2(Vec2::from(ori).magnitude()))
            * Mat4::scaling_3d(Vec3::from(0.8 * scale));

        /* let dependents = camera.get_dependents();
        let all_mat = dependents.proj_mat * dependents.view_mat; */

        let atlas_offs = model.allocation.rectangle.min;
        let locals = FigureLocals::new(
            mat,
            col,
            pos,
            Vec2::new(atlas_offs.x, atlas_offs.y),
            is_player,
        );
        renderer.update_consts(&mut self.locals, &[locals]).unwrap();

        let (new_bone_mats, lantern_offset) = self.skeleton.compute_matrices();

        let new_bone_consts = figure_bone_data_from_anim(new_bone_mats, |bone_mat| {
            let model_mat = mat * bone_mat;
            FigureBoneData::new(model_mat, model_mat.map_cols(|c| c.normalized()))
        });

        renderer
            .update_consts(
                &mut self.meta.bone_consts,
                &new_bone_consts[0..self.skeleton.bone_count()],
            )
            .unwrap();
        self.lantern_offset = lantern_offset;
    }

    pub fn locals(&self) -> &Consts<FigureLocals> { &self.locals }

    pub fn bone_consts(&self) -> &Consts<FigureBoneData> { &self.bone_consts }

    pub fn skeleton_mut(&mut self) -> &mut S { &mut self.skeleton }
}

fn figure_bone_data_from_anim(
    mats: [anim::FigureBoneData; 16],
    mut make_bone: impl FnMut(Mat4<f32>) -> FigureBoneData,
) -> [FigureBoneData; 16] {
    [
        make_bone(mats[0].0),
        make_bone(mats[1].0),
        make_bone(mats[2].0),
        make_bone(mats[3].0),
        make_bone(mats[4].0),
        make_bone(mats[5].0),
        make_bone(mats[6].0),
        make_bone(mats[7].0),
        make_bone(mats[8].0),
        make_bone(mats[9].0),
        make_bone(mats[10].0),
        make_bone(mats[11].0),
        make_bone(mats[12].0),
        make_bone(mats[13].0),
        make_bone(mats[14].0),
        make_bone(mats[15].0),
    ]
}
