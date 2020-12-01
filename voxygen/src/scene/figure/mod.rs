mod cache;
pub mod load;

pub use cache::FigureModelCache;
pub use load::load_mesh; // TODO: Don't make this public.

use crate::{
    ecs::comp::Interpolated,
    render::{
        ColLightFmt, ColLightInfo, Consts, FigureBoneData, FigureLocals, FigureModel, GlobalModel,
        Mesh, RenderError, Renderer, ShadowPipeline, TerrainPipeline, Texture,
    },
    scene::{
        camera::{Camera, CameraMode, Dependents},
        math,
        terrain::Terrain,
        LodData, SceneData,
    },
};
use anim::{
    biped_large::BipedLargeSkeleton, bird_medium::BirdMediumSkeleton,
    bird_small::BirdSmallSkeleton, character::CharacterSkeleton, dragon::DragonSkeleton,
    fish_medium::FishMediumSkeleton, fish_small::FishSmallSkeleton, golem::GolemSkeleton,
    object::ObjectSkeleton, quadruped_low::QuadrupedLowSkeleton,
    quadruped_medium::QuadrupedMediumSkeleton, quadruped_small::QuadrupedSmallSkeleton,
    theropod::TheropodSkeleton, Animation, Skeleton,
};
use common::{
    comp::{
        item::{ItemKind, ToolKind},
        Body, CharacterState, Health, Item, Last, LightAnimation, LightEmitter, Loadout, Ori,
        PhysicsState, Pos, Scale, Vel,
    },
    resources::DeltaTime,
    span,
    states::utils::StageSection,
    terrain::TerrainChunk,
    vol::RectRasterableVol,
};
use common_state::State;
use core::{
    borrow::Borrow,
    convert::TryFrom,
    hash::Hash,
    ops::{Deref, DerefMut, Range},
};
use guillotiere::AtlasAllocator;
use hashbrown::HashMap;
use specs::{Entity as EcsEntity, Join, LazyUpdate, WorldExt};
use treeculler::{BVol, BoundingSphere};
use vek::*;

const DAMAGE_FADE_COEFFICIENT: f64 = 15.0;
const MOVING_THRESHOLD: f32 = 0.7;
const MOVING_THRESHOLD_SQR: f32 = MOVING_THRESHOLD * MOVING_THRESHOLD;

/// camera data, figure LOD render distance.
pub type CameraData<'a> = (&'a Camera, f32);

/// Enough data to render a figure model.
pub type FigureModelRef<'a> = (
    &'a Consts<FigureLocals>,
    &'a Consts<FigureBoneData>,
    &'a FigureModel,
    &'a Texture<ColLightFmt>,
);

/// An entry holding enough information to draw or destroy a figure in a
/// particular cache.
pub struct FigureModelEntry<const N: usize> {
    /// The estimated bounds of this figure, in voxels.  This may not be very
    /// useful yet.
    _bounds: math::Aabb<f32>,
    /// Hypothetical texture atlas allocation data for the current figure.
    /// Will be useful if we decide to use a packed texture atlas for figures
    /// like we do for terrain.
    allocation: guillotiere::Allocation,
    /// Texture used to store color/light information for this figure entry.
    /* TODO: Consider using mipmaps instead of storing multiple texture atlases for different
     * LOD levels. */
    col_lights: Texture<ColLightFmt>,
    /// Models stored in this figure entry; there may be several for one figure,
    /// because of LOD models.
    pub models: [FigureModel; N],
}

struct FigureMgrStates {
    character_states: HashMap<EcsEntity, FigureState<CharacterSkeleton>>,
    quadruped_small_states: HashMap<EcsEntity, FigureState<QuadrupedSmallSkeleton>>,
    quadruped_medium_states: HashMap<EcsEntity, FigureState<QuadrupedMediumSkeleton>>,
    quadruped_low_states: HashMap<EcsEntity, FigureState<QuadrupedLowSkeleton>>,
    bird_medium_states: HashMap<EcsEntity, FigureState<BirdMediumSkeleton>>,
    fish_medium_states: HashMap<EcsEntity, FigureState<FishMediumSkeleton>>,
    theropod_states: HashMap<EcsEntity, FigureState<TheropodSkeleton>>,
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
            quadruped_low_states: HashMap::new(),
            bird_medium_states: HashMap::new(),
            fish_medium_states: HashMap::new(),
            theropod_states: HashMap::new(),
            dragon_states: HashMap::new(),
            bird_small_states: HashMap::new(),
            fish_small_states: HashMap::new(),
            biped_large_states: HashMap::new(),
            golem_states: HashMap::new(),
            object_states: HashMap::new(),
        }
    }

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
            Body::QuadrupedLow(_) => self
                .quadruped_low_states
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
            Body::Theropod(_) => self
                .theropod_states
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
            Body::QuadrupedLow(_) => self.quadruped_low_states.remove(&entity).map(|e| e.meta),
            Body::BirdMedium(_) => self.bird_medium_states.remove(&entity).map(|e| e.meta),
            Body::FishMedium(_) => self.fish_medium_states.remove(&entity).map(|e| e.meta),
            Body::Theropod(_) => self.theropod_states.remove(&entity).map(|e| e.meta),
            Body::Dragon(_) => self.dragon_states.remove(&entity).map(|e| e.meta),
            Body::BirdSmall(_) => self.bird_small_states.remove(&entity).map(|e| e.meta),
            Body::FishSmall(_) => self.fish_small_states.remove(&entity).map(|e| e.meta),
            Body::BipedLarge(_) => self.biped_large_states.remove(&entity).map(|e| e.meta),
            Body::Golem(_) => self.golem_states.remove(&entity).map(|e| e.meta),
            Body::Object(_) => self.object_states.remove(&entity).map(|e| e.meta),
        }
    }

    fn retain(&mut self, mut f: impl FnMut(&EcsEntity, &mut FigureStateMeta) -> bool) {
        span!(_guard, "retain", "FigureManagerStates::retain");
        self.character_states.retain(|k, v| f(k, &mut *v));
        self.quadruped_small_states.retain(|k, v| f(k, &mut *v));
        self.quadruped_medium_states.retain(|k, v| f(k, &mut *v));
        self.quadruped_low_states.retain(|k, v| f(k, &mut *v));
        self.bird_medium_states.retain(|k, v| f(k, &mut *v));
        self.fish_medium_states.retain(|k, v| f(k, &mut *v));
        self.theropod_states.retain(|k, v| f(k, &mut *v));
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
            + self.quadruped_low_states.len()
            + self.bird_medium_states.len()
            + self.fish_medium_states.len()
            + self.theropod_states.len()
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
            .filter(|(_, c)| c.visible())
            .count()
            + self
                .quadruped_small_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .quadruped_medium_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .quadruped_low_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .bird_medium_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .theropod_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .dragon_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .fish_medium_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .bird_small_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .fish_small_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .biped_large_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .golem_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
            + self
                .object_states
                .iter()
                .filter(|(_, c)| c.visible())
                .count()
    }
}

pub struct FigureMgr {
    col_lights: FigureColLights,
    model_cache: FigureModelCache,
    theropod_model_cache: FigureModelCache<TheropodSkeleton>,
    quadruped_small_model_cache: FigureModelCache<QuadrupedSmallSkeleton>,
    quadruped_medium_model_cache: FigureModelCache<QuadrupedMediumSkeleton>,
    quadruped_low_model_cache: FigureModelCache<QuadrupedLowSkeleton>,
    bird_medium_model_cache: FigureModelCache<BirdMediumSkeleton>,
    bird_small_model_cache: FigureModelCache<BirdSmallSkeleton>,
    dragon_model_cache: FigureModelCache<DragonSkeleton>,
    fish_medium_model_cache: FigureModelCache<FishMediumSkeleton>,
    fish_small_model_cache: FigureModelCache<FishSmallSkeleton>,
    biped_large_model_cache: FigureModelCache<BipedLargeSkeleton>,
    object_model_cache: FigureModelCache<ObjectSkeleton>,
    golem_model_cache: FigureModelCache<GolemSkeleton>,
    states: FigureMgrStates,
}

impl FigureMgr {
    pub fn new(renderer: &mut Renderer) -> Self {
        Self {
            col_lights: FigureColLights::new(renderer),
            model_cache: FigureModelCache::new(),
            theropod_model_cache: FigureModelCache::new(),
            quadruped_small_model_cache: FigureModelCache::new(),
            quadruped_medium_model_cache: FigureModelCache::new(),
            quadruped_low_model_cache: FigureModelCache::new(),
            bird_medium_model_cache: FigureModelCache::new(),
            bird_small_model_cache: FigureModelCache::new(),
            dragon_model_cache: FigureModelCache::new(),
            fish_medium_model_cache: FigureModelCache::new(),
            fish_small_model_cache: FigureModelCache::new(),
            biped_large_model_cache: FigureModelCache::new(),
            object_model_cache: FigureModelCache::new(),
            golem_model_cache: FigureModelCache::new(),
            states: FigureMgrStates::default(),
        }
    }

    pub fn col_lights(&self) -> &FigureColLights { &self.col_lights }

    pub fn clean(&mut self, tick: u64) {
        span!(_guard, "clean", "FigureManager::clean");
        self.model_cache.clean(&mut self.col_lights, tick);
        self.theropod_model_cache.clean(&mut self.col_lights, tick);
        self.quadruped_small_model_cache
            .clean(&mut self.col_lights, tick);
        self.quadruped_medium_model_cache
            .clean(&mut self.col_lights, tick);
        self.quadruped_low_model_cache
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
        self.object_model_cache.clean(&mut self.col_lights, tick);
        self.golem_model_cache.clean(&mut self.col_lights, tick);
    }

    #[allow(clippy::redundant_pattern_matching)]
    // TODO: Pending review in #587
    pub fn update_lighting(&mut self, scene_data: &SceneData) {
        span!(_guard, "update_lighting", "FigureManager::update_lighting");
        let ecs = scene_data.state.ecs();
        for (entity, body, light_emitter) in (
            &ecs.entities(),
            ecs.read_storage::<common::comp::Body>().maybe(),
            &ecs.read_storage::<LightEmitter>(),
        )
            .join()
        {
            // Add LightAnimation for objects with a LightEmitter
            let mut anim_storage = ecs.write_storage::<LightAnimation>();
            if let None = anim_storage.get_mut(entity) {
                let anim = LightAnimation {
                    offset: body
                        .map(|b| b.default_light_offset())
                        .unwrap_or_else(Vec3::zero),
                    col: light_emitter.col,
                    strength: 0.0,
                };
                let _ = anim_storage.insert(entity, anim);
            }
        }
        let dt = ecs.fetch::<DeltaTime>().0;
        let updater = ecs.read_resource::<LazyUpdate>();
        for (entity, light_emitter_opt, body, light_anim) in (
            &ecs.entities(),
            ecs.read_storage::<LightEmitter>().maybe(),
            ecs.read_storage::<Body>().maybe(),
            &mut ecs.write_storage::<LightAnimation>(),
        )
            .join()
        {
            let (target_col, target_strength, flicker, animated) =
                if let Some(emitter) = light_emitter_opt {
                    (
                        emitter.col,
                        if emitter.strength.is_normal() {
                            emitter.strength
                        } else {
                            0.0
                        },
                        emitter.flicker,
                        emitter.animated,
                    )
                } else {
                    (vek::Rgb::zero(), 0.0, 0.0, true)
                };
            if let Some(state) = body.and_then(|body| self.states.get_mut(body, &entity)) {
                light_anim.offset = vek::Vec3::from(state.lantern_offset);
            }
            if !light_anim.strength.is_normal() {
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
            // NOTE: We add `LIGHT_EPSILON` because if we wait for numbers to become
            // equal to target (or even within a subnormal), it will take a minimum
            // of 30 seconds for a light to fully turn off (for initial
            // strength ≥ 1), which prevents optimizations (particularly those that
            // can kick in with zero lights).
            const LIGHT_EPSILON: f32 = 0.0001;
            if (light_anim.strength - target_strength).abs() < LIGHT_EPSILON {
                light_anim.strength = target_strength;
                if light_anim.strength == 0.0 {
                    updater.remove::<LightAnimation>(entity);
                }
            }
        }
    }

    #[allow(clippy::or_fun_call)]
    // TODO: Pending review in #587
    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        scene_data: &SceneData,
        // Visible chunk data.
        visible_psr_bounds: math::Aabr<f32>,
        camera: &Camera,
        terrain: Option<&Terrain>,
    ) -> anim::vek::Aabb<f32> {
        span!(_guard, "maintain", "FigureManager::maintain");
        let state = scene_data.state;
        let time = state.get_time();
        let tick = scene_data.tick;
        let ecs = state.ecs();
        let view_distance = scene_data.view_distance;
        let dt = state.get_delta_time();
        let dt_lerp = (15.0 * dt).min(1.0);
        let frustum = camera.frustum();

        // Sun shadows--find the bounding box of the shadow map plane (i.e. the bounds
        // of the image rendered from the light).  If the position projected
        // with the ray_mat matrix is valid, and shadows are otherwise enabled,
        // we mark can_shadow.
        let can_shadow_sun = {
            let ray_direction = scene_data.get_sun_dir();
            let is_daylight = ray_direction.z < 0.0/*0.6*/;
            // Are shadows enabled at all?
            let can_shadow_sun = renderer.render_mode().shadow.is_map() && is_daylight;
            let Dependents {
                proj_mat: _,
                view_mat: _,
                cam_pos,
                ..
            } = camera.dependents();
            let cam_pos = math::Vec3::from(cam_pos);
            let ray_direction = math::Vec3::from(ray_direction);

            // Transform (semi) world space to light space.
            let ray_mat: math::Mat4<f32> =
                math::Mat4::look_at_rh(cam_pos, cam_pos + ray_direction, math::Vec3::up());
            let focus_off = math::Vec3::from(camera.get_focus_pos().map(f32::trunc));
            let ray_mat = ray_mat * math::Mat4::translation_3d(-focus_off);

            let collides_with_aabr = |a: math::Aabr<f32>, b: math::Aabr<f32>| {
                let min = math::Vec4::new(a.min.x, a.min.y, b.min.x, b.min.y);
                let max = math::Vec4::new(b.max.x, b.max.y, a.max.x, a.max.y);
                #[cfg(feature = "simd")]
                return min.partial_cmple_simd(max).reduce_and();
                #[cfg(not(feature = "simd"))]
                return min.partial_cmple(&max).reduce_and();
            };
            move |pos: (anim::vek::Vec3<f32>,), radius: f32| {
                // Short circuit when there are no shadows to cast.
                if !can_shadow_sun {
                    return false;
                }
                // First project center onto shadow map.
                let center = (ray_mat * math::Vec4::new(pos.0.x, pos.0.y, pos.0.z, 1.0)).xy();
                // Then, create an approximate bounding box (± radius).
                let figure_box = math::Aabr {
                    min: center - radius,
                    max: center + radius,
                };
                // Quick intersection test for membership in the PSC (potential shader caster)
                // list.
                collides_with_aabr(figure_box, visible_psr_bounds)
            }
        };

        // Get player position.
        let player_pos = ecs
            .read_storage::<Pos>()
            .get(scene_data.player_entity)
            .map_or(anim::vek::Vec3::zero(), |pos| anim::vek::Vec3::from(pos.0));
        let visible_aabb = anim::vek::Aabb {
            min: player_pos - 2.0,
            max: player_pos + 2.0,
        };
        let camera_mode = camera.get_mode();
        let character_state_storage = state.read_storage::<common::comp::CharacterState>();
        let character_state = character_state_storage.get(scene_data.player_entity);

        let focus_pos = anim::vek::Vec3::<f32>::from(camera.get_focus_pos());

        let mut update_buf = [Default::default(); anim::MAX_BONE_COUNT];

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
                health,
                loadout,
                item,
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
            ecs.read_storage::<Health>().maybe(),
            ecs.read_storage::<Loadout>().maybe(),
            ecs.read_storage::<Item>().maybe(),
        )
            .join()
            .enumerate()
        {
            let vel = (anim::vek::Vec3::<f32>::from(vel.0),);
            let is_player = scene_data.player_entity == entity;
            let player_camera_mode = if is_player {
                camera_mode
            } else {
                CameraMode::default()
            };
            let player_character_state = if is_player { character_state } else { None };

            let (pos, ori) = interpolated
                .map(|i| {
                    (
                        (anim::vek::Vec3::from(i.pos),),
                        anim::vek::Vec3::from(*i.ori),
                    )
                })
                .unwrap_or((
                    (anim::vek::Vec3::<f32>::from(pos.0),),
                    anim::vek::Vec3::<f32>::unit_y(),
                ));

            // Maintaining figure data and sending new figure data to the GPU turns out to
            // be a very expensive operation. We want to avoid doing it as much
            // as possible, so we make the assumption that players don't care so
            // much about the update *rate* for far away things. As the entity
            // goes further and further away, we start to 'skip' update ticks.
            // TODO: Investigate passing the velocity into the shader so we can at least
            // interpolate motion
            const MIN_PERFECT_RATE_DIST: f32 = 50.0;

            if (i as u64 + tick)
                % (1 + ((pos.0.distance_squared(focus_pos).powf(0.25)
                    - MIN_PERFECT_RATE_DIST.sqrt())
                .max(0.0)
                    / 3.0) as u64)
                != 0
            {
                continue;
            }

            // Check whether we could have been shadowing last frame.
            let mut state = self.states.get_mut(body, &entity);
            let can_shadow_prev = state
                .as_mut()
                .map(|state| state.can_shadow_sun())
                .unwrap_or(false);

            // Don't process figures outside the vd
            let vd_frac = anim::vek::Vec2::from(pos.0 - player_pos)
                .map2(
                    anim::vek::Vec2::<u32>::from(TerrainChunk::RECT_SIZE),
                    |d: f32, sz| d.abs() as f32 / sz as f32,
                )
                .magnitude()
                / view_distance as f32;

            // Keep from re-adding/removing entities on the border of the vd
            if vd_frac > 1.2 {
                self.states.remove(body, &entity);
                continue;
            } else if vd_frac > 1.0 {
                state.as_mut().map(|state| state.visible = false);
                // Keep processing if this might be a shadow caster.
                if !can_shadow_prev {
                    continue;
                }
            }

            // Don't display figures outside the frustum spectrum (this is important to do
            // for any figure that potentially casts a shadow, since we use this
            // to estimate bounds for shadow maps).  Currently, we don't do this before the
            // update cull, so it's possible that faraway figures will not
            // shadow correctly until their next update.  For now, we treat this
            // as an acceptable tradeoff.
            let radius = scale.unwrap_or(&Scale(1.0)).0 * 2.0;
            let (in_frustum, lpindex) = if let Some(mut meta) = state {
                let (in_frustum, lpindex) = BoundingSphere::new(pos.0.into_array(), radius)
                    .coherent_test_against_frustum(frustum, meta.lpindex);
                meta.visible = in_frustum;
                meta.lpindex = lpindex;
                if in_frustum {
                    /* // Update visible bounds.
                    visible_aabb.expand_to_contain(Aabb {
                        min: pos.0 - radius,
                        max: pos.0 + radius,
                    }); */
                } else {
                    // Check whether we can shadow.
                    meta.can_shadow_sun = can_shadow_sun(pos, radius);
                }
                (in_frustum, lpindex)
            } else {
                (true, 0)
            };

            // Change in health as color!
            let col = health
                .map(|h| {
                    vek::Rgba::broadcast(1.0)
                        + vek::Rgba::new(10.0, 10.0, 10.0, 0.0).map(|c| {
                            (c / (1.0 + DAMAGE_FADE_COEFFICIENT * h.last_change.0)) as f32
                        })
                })
                .unwrap_or(vek::Rgba::broadcast(1.0))
            // Highlight targeted collectible entities
            * if item.is_some() && scene_data.target_entity.map_or(false, |e| e == entity) {
                vek::Rgba::new(5.0, 5.0, 5.0, 1.0)
            } else {
                vek::Rgba::one()
            };

            let scale = scale.map(|s| s.0).unwrap_or(1.0);

            let mut state_animation_rate = 1.0;

            let active_item_kind = loadout
                .and_then(|l| l.active_item.as_ref())
                .map(|i| i.item.kind());
            let active_tool_kind = if let Some(ItemKind::Tool(tool)) = active_item_kind {
                Some(tool.kind)
            } else {
                None
            };

            let second_item_kind = loadout
                .and_then(|l| l.second_item.as_ref())
                .map(|i| i.item.kind());

            let second_tool_kind = if let Some(ItemKind::Tool(tool)) = second_item_kind {
                Some(tool.kind)
            } else {
                None
            };

            match body {
                Body::Humanoid(body) => {
                    let (model, skeleton_attr) = self.model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        player_camera_mode,
                        player_character_state,
                        scene_data.thread_pool,
                    );

                    let state = self
                        .states
                        .character_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, CharacterSkeleton::default())
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
                        physics.in_liquid.is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::character::StandAnimation::update_skeleton(
                            &CharacterSkeleton::default(),
                            (active_tool_kind, second_tool_kind, time, state.avg_vel),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::character::RunAnimation::update_skeleton(
                            &CharacterSkeleton::default(),
                            (
                                active_tool_kind,
                                second_tool_kind,
                                vel.0,
                                ori,
                                state.last_ori,
                                time,
                                state.avg_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::character::JumpAnimation::update_skeleton(
                            &CharacterSkeleton::default(),
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
                        (_, _, true) => anim::character::SwimAnimation::update_skeleton(
                            &CharacterSkeleton::default(),
                            (
                                active_tool_kind,
                                second_tool_kind,
                                vel.0,
                                ori,
                                state.last_ori,
                                time,
                                state.avg_vel,
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
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0.magnitude(),
                                    time,
                                    None,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ChargedRanged(s) => {
                            let stage_time = s.timer.as_secs_f64();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },

                                _ => 0.0,
                            };

                            anim::character::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0.magnitude(),
                                    ori,
                                    state.last_ori,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicRanged(s) => {
                            let stage_time = s.timer.as_secs_f64();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },

                                _ => 0.0,
                            };

                            anim::character::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0.magnitude(),
                                    ori,
                                    state.last_ori,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ChargedMelee(s) => {
                            let stage_time = s.timer.as_secs_f64();

                            let stage_progress = match s.stage_section {
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time / s.static_data.swing_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },
                                _ => 0.0,
                            };

                            anim::character::ChargeswingAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::RepeaterRanged(s) => {
                            let stage_time = s.timer.as_secs_f64();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Movement => {
                                    stage_time / s.static_data.movement_duration.as_secs_f64()
                                },
                                StageSection::Shoot => {
                                    stage_time / s.static_data.shoot_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },
                                _ => 0.0,
                            };

                            anim::character::RepeaterAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Sneak { .. } => {
                            anim::character::SneakAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, vel.0, ori, state.last_ori, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Boost(_) => {
                            anim::character::AlphaAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0.magnitude(),
                                    time,
                                    None,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time / s.static_data.swing_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            anim::character::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Shockwave(s) => {
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time / s.static_data.swing_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            anim::character::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    time,
                                    vel.0.magnitude(),
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::LeapMelee(s) => {
                            let stage_progress = match active_tool_kind {
                                Some(ToolKind::Axe | ToolKind::Hammer) => {
                                    let stage_time = s.timer.as_secs_f64();
                                    match s.stage_section {
                                        StageSection::Buildup => {
                                            stage_time
                                                / s.static_data.buildup_duration.as_secs_f64()
                                        },
                                        StageSection::Movement => {
                                            stage_time
                                                / s.static_data.movement_duration.as_secs_f64()
                                        },
                                        StageSection::Swing => {
                                            stage_time / s.static_data.swing_duration.as_secs_f64()
                                        },
                                        StageSection::Recover => {
                                            stage_time
                                                / s.static_data.recover_duration.as_secs_f64()
                                        },
                                        _ => 0.0,
                                    }
                                },
                                _ => state.state_time,
                            };

                            anim::character::LeapAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::SpinMelee(s) => {
                            let stage_progress = match active_tool_kind {
                                Some(ToolKind::Sword) => {
                                    let stage_time = s.timer.as_secs_f64();
                                    match s.stage_section {
                                        StageSection::Buildup => {
                                            stage_time
                                                / s.static_data.buildup_duration.as_secs_f64()
                                        },
                                        StageSection::Swing => {
                                            stage_time / s.static_data.swing_duration.as_secs_f64()
                                        },
                                        StageSection::Recover => {
                                            stage_time
                                                / s.static_data.recover_duration.as_secs_f64()
                                        },
                                        _ => 0.0,
                                    }
                                },
                                _ => state.state_time,
                            };

                            anim::character::SpinMeleeAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicBeam(s) => {
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Cast => s.timer.as_secs_f64(),
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            anim::character::BeamAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    time,
                                    vel.0.magnitude(),
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ComboMelee(s) => {
                            let stage_index = (s.stage - 1) as usize;
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_buildup_duration
                                            .as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_swing_duration
                                            .as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_recover_duration
                                            .as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            match s.stage {
                                1 => anim::character::AlphaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        active_tool_kind,
                                        second_tool_kind,
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                                2 => anim::character::SpinAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        active_tool_kind,
                                        second_tool_kind,
                                        vel.0,
                                        time,
                                        Some(s.stage_section),
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                                _ => anim::character::BetaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        active_tool_kind,
                                        second_tool_kind,
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                            }
                        },
                        CharacterState::BasicBlock { .. } => {
                            anim::character::BlockAnimation::update_skeleton(
                                &CharacterSkeleton::default(),
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
                            if physics.in_liquid.is_some() {
                                anim::character::SwimWieldAnimation::update_skeleton(
                                    &target_base,
                                    (active_tool_kind, second_tool_kind, vel.0.magnitude(), time),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            } else {
                                anim::character::WieldAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        active_tool_kind,
                                        second_tool_kind,
                                        ori,
                                        state.last_ori,
                                        vel.0,
                                        time,
                                    ),
                                    state.state_time,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            }
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
                                &target_base,
                                (active_tool_kind, second_tool_kind, vel.0, ori, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Sit { .. } => {
                            anim::character::SitAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, second_tool_kind, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::GlideWield { .. } => {
                            anim::character::GlideWieldAnimation::update_skeleton(
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
                        CharacterState::Dance { .. } => {
                            anim::character::DanceAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, second_tool_kind, time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        _ => target_base,
                    };

                    state.skeleton = anim::vek::Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
                    );
                },
                Body::QuadrupedSmall(body) => {
                    let (model, skeleton_attr) =
                        self.quadruped_small_model_cache.get_or_create_model(
                            renderer,
                            &mut self.col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            player_character_state,
                            scene_data.thread_pool,
                        );

                    let state = self
                        .states
                        .quadruped_small_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, QuadrupedSmallSkeleton::default())
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
                        physics.in_liquid.is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => {
                            anim::quadruped_small::IdleAnimation::update_skeleton(
                                &QuadrupedSmallSkeleton::default(),
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Running
                        (true, true, false) => {
                            anim::quadruped_small::RunAnimation::update_skeleton(
                                &QuadrupedSmallSkeleton::default(),
                                (vel.0.magnitude(), ori, state.last_ori, time, state.avg_vel),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Running
                        (false, _, true) => anim::quadruped_small::RunAnimation::update_skeleton(
                            &QuadrupedSmallSkeleton::default(),
                            (vel.0.magnitude(), ori, state.last_ori, time, state.avg_vel),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::quadruped_small::JumpAnimation::update_skeleton(
                            &QuadrupedSmallSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        _ => anim::quadruped_small::IdleAnimation::update_skeleton(
                            &QuadrupedSmallSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::ComboMelee(s) => {
                            let stage_index = (s.stage - 1) as usize;
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_buildup_duration
                                            .as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_swing_duration
                                            .as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_recover_duration
                                            .as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            {
                                anim::quadruped_small::AlphaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                        state.state_time,
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                )
                            }
                        },
                        CharacterState::Sit { .. } => {
                            anim::quadruped_small::FeedAnimation::update_skeleton(
                                &target_base,
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = anim::vek::Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
                    );
                },
                Body::QuadrupedMedium(body) => {
                    let (model, skeleton_attr) =
                        self.quadruped_medium_model_cache.get_or_create_model(
                            renderer,
                            &mut self.col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            player_character_state,
                            scene_data.thread_pool,
                        );

                    let state = self
                        .states
                        .quadruped_medium_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, QuadrupedMediumSkeleton::default())
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
                        vel.0.magnitude_squared() > 0.25, // Moving
                        physics.in_liquid.is_some(),      // In water
                    ) {
                        // Standing
                        (true, false, false) => {
                            anim::quadruped_medium::IdleAnimation::update_skeleton(
                                &QuadrupedMediumSkeleton::default(),
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Running
                        (true, true, false) => {
                            anim::quadruped_medium::RunAnimation::update_skeleton(
                                &QuadrupedMediumSkeleton::default(),
                                (vel.0.magnitude(), ori, state.last_ori, time, state.avg_vel),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        //Swimming
                        (false, _, true) => anim::quadruped_medium::RunAnimation::update_skeleton(
                            &QuadrupedMediumSkeleton::default(),
                            (vel.0.magnitude(), ori, state.last_ori, time, state.avg_vel),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => {
                            anim::quadruped_medium::JumpAnimation::update_skeleton(
                                &QuadrupedMediumSkeleton::default(),
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        _ => anim::quadruped_medium::IdleAnimation::update_skeleton(
                            &QuadrupedMediumSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::BasicMelee(s) => {
                            let stage_time = s.timer.as_secs_f64();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time / s.static_data.swing_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },

                                _ => 0.0,
                            };
                            anim::quadruped_medium::HoofAnimation::update_skeleton(
                                &target_base,
                                (
                                    vel.0.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Charge => stage_time,
                                StageSection::Swing => {
                                    stage_time / s.static_data.swing_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            anim::quadruped_medium::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    vel.0.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::LeapMelee(s) => {
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Movement => {
                                    stage_time / s.static_data.movement_duration.as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time / s.static_data.swing_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            anim::quadruped_medium::LeapMeleeAnimation::update_skeleton(
                                &target_base,
                                (
                                    vel.0.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ComboMelee(s) => {
                            let stage_index = (s.stage - 1) as usize;
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_buildup_duration
                                            .as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_swing_duration
                                            .as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_recover_duration
                                            .as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            match s.stage {
                                1 => anim::quadruped_medium::AlphaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                        state.state_time,
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                                2 => anim::quadruped_medium::BetaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                        state.state_time,
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                                _ => anim::quadruped_medium::AlphaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                        state.state_time,
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                            }
                        },
                        CharacterState::Sit { .. } => {
                            anim::quadruped_medium::FeedAnimation::update_skeleton(
                                &target_base,
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = anim::vek::Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
                    );
                },
                Body::QuadrupedLow(body) => {
                    let (model, skeleton_attr) =
                        self.quadruped_low_model_cache.get_or_create_model(
                            renderer,
                            &mut self.col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            player_character_state,
                            scene_data.thread_pool,
                        );

                    let state = self
                        .states
                        .quadruped_low_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, QuadrupedLowSkeleton::default())
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
                        physics.in_liquid.is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => {
                            anim::quadruped_low::IdleAnimation::update_skeleton(
                                &QuadrupedLowSkeleton::default(),
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // Running
                        (true, true, false) => anim::quadruped_low::RunAnimation::update_skeleton(
                            &QuadrupedLowSkeleton::default(),
                            (vel.0.magnitude(), ori, state.last_ori, time, state.avg_vel),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Swimming
                        (false, _, true) => anim::quadruped_low::RunAnimation::update_skeleton(
                            &QuadrupedLowSkeleton::default(),
                            (vel.0.magnitude(), ori, state.last_ori, time, state.avg_vel),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::quadruped_low::JumpAnimation::update_skeleton(
                            &QuadrupedLowSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        _ => anim::quadruped_low::IdleAnimation::update_skeleton(
                            &QuadrupedLowSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::BasicRanged(s) => {
                            let stage_time = s.timer.as_secs_f64();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },

                                _ => 0.0,
                            };
                            anim::quadruped_low::ShootAnimation::update_skeleton(
                                &target_base,
                                (vel.0.magnitude(), time, Some(s.stage_section)),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicMelee(s) => {
                            let stage_time = s.timer.as_secs_f64();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time / s.static_data.swing_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },

                                _ => 0.0,
                            };
                            anim::quadruped_low::BetaAnimation::update_skeleton(
                                &target_base,
                                (
                                    vel.0.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ChargedMelee(s) => {
                            let stage_time = s.timer.as_secs_f64();

                            let stage_progress = match s.stage_section {
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time / s.static_data.swing_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },

                                _ => 0.0,
                            };
                            anim::quadruped_low::TailwhipAnimation::update_skeleton(
                                &target_base,
                                (
                                    vel.0.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ComboMelee(s) => {
                            let stage_index = (s.stage - 1) as usize;
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_buildup_duration
                                            .as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_swing_duration
                                            .as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_recover_duration
                                            .as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            match s.stage {
                                1 => anim::quadruped_low::AlphaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                        state.state_time,
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                                2 => anim::quadruped_low::BetaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                        state.state_time,
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                                _ => anim::quadruped_low::AlphaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                        state.state_time,
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                            }
                        },
                        CharacterState::BasicBeam(s) => {
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Cast => s.timer.as_secs_f64(),
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            anim::quadruped_low::BreatheAnimation::update_skeleton(
                                &target_base,
                                (
                                    vel.0.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Charge => stage_time,
                                StageSection::Swing => {
                                    stage_time / s.static_data.swing_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            anim::quadruped_low::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    vel.0.magnitude(),
                                    time,
                                    Some(s.stage_section),
                                    state.state_time,
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = anim::vek::Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
                    );
                },
                Body::BirdMedium(body) => {
                    let (model, skeleton_attr) = self.bird_medium_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        player_camera_mode,
                        player_character_state,
                        scene_data.thread_pool,
                    );

                    let state = self
                        .states
                        .bird_medium_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, BirdMediumSkeleton::default())
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
                        physics.in_liquid.is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::bird_medium::IdleAnimation::update_skeleton(
                            &BirdMediumSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::bird_medium::RunAnimation::update_skeleton(
                            &BirdMediumSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (false, _, true) => anim::bird_medium::RunAnimation::update_skeleton(
                            &BirdMediumSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::bird_medium::FlyAnimation::update_skeleton(
                            &BirdMediumSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        _ => anim::bird_medium::IdleAnimation::update_skeleton(
                            &BirdMediumSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::Sit { .. } => {
                            anim::bird_medium::FeedAnimation::update_skeleton(
                                &target_base,
                                time,
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = anim::vek::Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
                    );
                },
                Body::FishMedium(body) => {
                    let (model, skeleton_attr) = self.fish_medium_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        player_camera_mode,
                        player_character_state,
                        scene_data.thread_pool,
                    );

                    let state = self
                        .states
                        .fish_medium_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, FishMediumSkeleton::default())
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
                        physics.in_liquid.is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::fish_medium::IdleAnimation::update_skeleton(
                            &FishMediumSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::fish_medium::RunAnimation::update_skeleton(
                            &FishMediumSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::fish_medium::JumpAnimation::update_skeleton(
                            &FishMediumSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => anim::fish_medium::IdleAnimation::update_skeleton(
                            &FishMediumSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };

                    state.skeleton = anim::vek::Lerp::lerp(&state.skeleton, &target_base, dt_lerp);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
                    );
                },
                Body::Dragon(body) => {
                    let (model, skeleton_attr) = self.dragon_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        player_camera_mode,
                        player_character_state,
                        scene_data.thread_pool,
                    );

                    let state =
                        self.states.dragon_states.entry(entity).or_insert_with(|| {
                            FigureState::new(renderer, DragonSkeleton::default())
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
                        physics.in_liquid.is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::dragon::IdleAnimation::update_skeleton(
                            &DragonSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::dragon::RunAnimation::update_skeleton(
                            &DragonSkeleton::default(),
                            (vel.0.magnitude(), ori, state.last_ori, time, state.avg_vel),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::dragon::FlyAnimation::update_skeleton(
                            &DragonSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // TODO!
                        _ => anim::dragon::IdleAnimation::update_skeleton(
                            &DragonSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };

                    state.skeleton = anim::vek::Lerp::lerp(&state.skeleton, &target_base, dt_lerp);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
                    );
                },
                Body::Theropod(body) => {
                    let (model, skeleton_attr) = self.theropod_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        player_camera_mode,
                        player_character_state,
                        scene_data.thread_pool,
                    );

                    let state = self
                        .states
                        .theropod_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, TheropodSkeleton::default()));

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
                        physics.in_liquid.is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::theropod::IdleAnimation::update_skeleton(
                            &TheropodSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::theropod::RunAnimation::update_skeleton(
                            &TheropodSkeleton::default(),
                            (vel.0.magnitude(), ori, state.last_ori, time, state.avg_vel),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::theropod::JumpAnimation::update_skeleton(
                            &TheropodSkeleton::default(),
                            (vel.0.magnitude(), ori, state.last_ori, time, state.avg_vel),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        _ => anim::theropod::IdleAnimation::update_skeleton(
                            &TheropodSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::ComboMelee(s) => {
                            let stage_index = (s.stage - 1) as usize;
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_buildup_duration
                                            .as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_swing_duration
                                            .as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_recover_duration
                                            .as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            match s.stage {
                                1 => anim::theropod::AlphaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                        state.state_time,
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                                _ => anim::theropod::BetaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                        state.state_time,
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                            }
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = anim::vek::Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
                    );
                },
                Body::BirdSmall(body) => {
                    let (model, skeleton_attr) = self.bird_small_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        player_camera_mode,
                        player_character_state,
                        scene_data.thread_pool,
                    );

                    let state = self
                        .states
                        .bird_small_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, BirdSmallSkeleton::default())
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
                        physics.in_liquid.is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::bird_small::IdleAnimation::update_skeleton(
                            &BirdSmallSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::bird_small::RunAnimation::update_skeleton(
                            &BirdSmallSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::bird_small::JumpAnimation::update_skeleton(
                            &BirdSmallSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => anim::bird_small::IdleAnimation::update_skeleton(
                            &BirdSmallSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };

                    state.skeleton = anim::vek::Lerp::lerp(&state.skeleton, &target_base, dt_lerp);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
                    );
                },
                Body::FishSmall(body) => {
                    let (model, skeleton_attr) = self.fish_small_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        player_camera_mode,
                        player_character_state,
                        scene_data.thread_pool,
                    );

                    let state = self
                        .states
                        .fish_small_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, FishSmallSkeleton::default())
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
                        physics.in_liquid.is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::fish_small::IdleAnimation::update_skeleton(
                            &FishSmallSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::fish_small::RunAnimation::update_skeleton(
                            &FishSmallSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::fish_small::JumpAnimation::update_skeleton(
                            &FishSmallSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => anim::fish_small::IdleAnimation::update_skeleton(
                            &FishSmallSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };

                    state.skeleton = anim::vek::Lerp::lerp(&state.skeleton, &target_base, dt_lerp);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
                    );
                },
                Body::BipedLarge(body) => {
                    let (model, skeleton_attr) = self.biped_large_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        player_camera_mode,
                        player_character_state,
                        scene_data.thread_pool,
                    );

                    let state = self
                        .states
                        .biped_large_states
                        .entry(entity)
                        .or_insert_with(|| {
                            FigureState::new(renderer, BipedLargeSkeleton::default())
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
                        physics.in_liquid.is_some(),                      // In water
                    ) {
                        // Running
                        (true, true, false) => anim::biped_large::RunAnimation::update_skeleton(
                            &BipedLargeSkeleton::default(),
                            (
                                active_tool_kind,
                                second_tool_kind,
                                vel.0.magnitude(),
                                ori,
                                state.last_ori,
                                time,
                                state.avg_vel,
                            ),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::biped_large::JumpAnimation::update_skeleton(
                            &BipedLargeSkeleton::default(),
                            (active_tool_kind, second_tool_kind, time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        _ => anim::biped_large::IdleAnimation::update_skeleton(
                            &BipedLargeSkeleton::default(),
                            (active_tool_kind, second_tool_kind, time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::Equipping { .. } => {
                            anim::biped_large::EquipAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, second_tool_kind, vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Wielding { .. } => {
                            anim::biped_large::WieldAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, second_tool_kind, vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicMelee(_) => {
                            anim::biped_large::AlphaAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0.magnitude(),
                                    time,
                                    None,
                                ),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicRanged(s) => {
                            let stage_time = s.timer.as_secs_f64();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },

                                _ => 0.0,
                            };

                            anim::biped_large::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0.magnitude(),
                                    ori,
                                    state.last_ori,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ChargedRanged(s) => {
                            let stage_time = s.timer.as_secs_f64();

                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },

                                _ => 0.0,
                            };

                            anim::biped_large::ShootAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0.magnitude(),
                                    ori,
                                    state.last_ori,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::DashMelee(s) => {
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Charge => {
                                    stage_time / s.static_data.charge_duration.as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time / s.static_data.swing_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            anim::biped_large::DashAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::ComboMelee(s) => {
                            let stage_index = (s.stage - 1) as usize;
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_buildup_duration
                                            .as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_swing_duration
                                            .as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time
                                        / s.static_data.stage_data[stage_index]
                                            .base_recover_duration
                                            .as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            match s.stage {
                                1 => anim::biped_large::AlphaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        active_tool_kind,
                                        second_tool_kind,
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                                2 => anim::biped_large::SpinAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        active_tool_kind,
                                        second_tool_kind,
                                        time,
                                        Some(s.stage_section),
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                                _ => anim::biped_large::BetaAnimation::update_skeleton(
                                    &target_base,
                                    (
                                        active_tool_kind,
                                        second_tool_kind,
                                        vel.0.magnitude(),
                                        time,
                                        Some(s.stage_section),
                                    ),
                                    stage_progress,
                                    &mut state_animation_rate,
                                    skeleton_attr,
                                ),
                            }
                        },
                        CharacterState::SpinMelee(s) => {
                            let stage_progress = match active_tool_kind {
                                Some(ToolKind::Sword) => {
                                    let stage_time = s.timer.as_secs_f64();
                                    match s.stage_section {
                                        StageSection::Buildup => {
                                            stage_time
                                                / s.static_data.buildup_duration.as_secs_f64()
                                        },
                                        StageSection::Swing => {
                                            stage_time / s.static_data.swing_duration.as_secs_f64()
                                        },
                                        StageSection::Recover => {
                                            stage_time
                                                / s.static_data.recover_duration.as_secs_f64()
                                        },
                                        _ => 0.0,
                                    }
                                },
                                _ => state.state_time,
                            };

                            anim::biped_large::SpinMeleeAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::LeapMelee(s) => {
                            let stage_progress = match active_tool_kind {
                                Some(ToolKind::Axe | ToolKind::Hammer) => {
                                    let stage_time = s.timer.as_secs_f64();
                                    match s.stage_section {
                                        StageSection::Buildup => {
                                            stage_time
                                                / s.static_data.buildup_duration.as_secs_f64()
                                        },
                                        StageSection::Movement => {
                                            stage_time
                                                / s.static_data.movement_duration.as_secs_f64()
                                        },
                                        StageSection::Swing => {
                                            stage_time / s.static_data.swing_duration.as_secs_f64()
                                        },
                                        StageSection::Recover => {
                                            stage_time
                                                / s.static_data.recover_duration.as_secs_f64()
                                        },
                                        _ => 0.0,
                                    }
                                },
                                _ => state.state_time,
                            };

                            anim::biped_large::LeapAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    vel.0,
                                    time,
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Shockwave(s) => {
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Swing => {
                                    stage_time / s.static_data.swing_duration.as_secs_f64()
                                },
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            anim::biped_large::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    time,
                                    vel.0.magnitude(),
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::BasicBeam(s) => {
                            let stage_time = s.timer.as_secs_f64();
                            let stage_progress = match s.stage_section {
                                StageSection::Buildup => {
                                    stage_time / s.static_data.buildup_duration.as_secs_f64()
                                },
                                StageSection::Cast => s.timer.as_secs_f64(),
                                StageSection::Recover => {
                                    stage_time / s.static_data.recover_duration.as_secs_f64()
                                },
                                _ => 0.0,
                            };
                            anim::biped_large::BeamAnimation::update_skeleton(
                                &target_base,
                                (
                                    active_tool_kind,
                                    second_tool_kind,
                                    time,
                                    vel.0.magnitude(),
                                    Some(s.stage_section),
                                ),
                                stage_progress,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = anim::vek::Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
                    );
                },
                Body::Golem(body) => {
                    let (model, skeleton_attr) = self.golem_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        player_camera_mode,
                        player_character_state,
                        scene_data.thread_pool,
                    );

                    let state =
                        self.states.golem_states.entry(entity).or_insert_with(|| {
                            FigureState::new(renderer, GolemSkeleton::default())
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
                        physics.in_liquid.is_some(),                      // In water
                    ) {
                        // Standing
                        (true, false, false) => anim::golem::IdleAnimation::update_skeleton(
                            &GolemSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // Running
                        (true, true, false) => anim::golem::RunAnimation::update_skeleton(
                            &GolemSkeleton::default(),
                            (vel.0.magnitude(), ori, state.last_ori, time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                        // In air
                        (false, _, false) => anim::golem::JumpAnimation::update_skeleton(
                            &GolemSkeleton::default(),
                            (vel.0.magnitude(), time),
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),

                        _ => anim::golem::IdleAnimation::update_skeleton(
                            &GolemSkeleton::default(),
                            time,
                            state.state_time,
                            &mut state_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match &character {
                        CharacterState::BasicMelee(_) => {
                            anim::golem::AlphaAnimation::update_skeleton(
                                &target_base,
                                (vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        CharacterState::Shockwave(_) => {
                            anim::golem::ShockwaveAnimation::update_skeleton(
                                &target_base,
                                (vel.0.magnitude(), time),
                                state.state_time,
                                &mut state_animation_rate,
                                skeleton_attr,
                            )
                        },
                        // TODO!
                        _ => target_base,
                    };

                    state.skeleton = anim::vek::Lerp::lerp(&state.skeleton, &target_bones, dt_lerp);
                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        in_frustum,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
                    );
                },
                Body::Object(body) => {
                    let (model, _) = self.object_model_cache.get_or_create_model(
                        renderer,
                        &mut self.col_lights,
                        *body,
                        loadout,
                        tick,
                        player_camera_mode,
                        player_character_state,
                        scene_data.thread_pool,
                    );

                    let state =
                        self.states.object_states.entry(entity).or_insert_with(|| {
                            FigureState::new(renderer, ObjectSkeleton::default())
                        });

                    state.update(
                        renderer,
                        pos.0,
                        ori,
                        scale,
                        col,
                        dt,
                        state_animation_rate,
                        model,
                        lpindex,
                        true,
                        is_player,
                        camera,
                        &mut update_buf,
                        terrain,
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
        &self,
        renderer: &mut Renderer,
        state: &State,
        tick: u64,
        global: &GlobalModel,
        (is_daylight, _light_data): super::LightData,
        (camera, figure_lod_render_distance): CameraData,
    ) {
        span!(_guard, "render_shadows", "FigureManager::render_shadows");
        let ecs = state.ecs();

        if is_daylight && renderer.render_mode().shadow.is_map() {
            (
                &ecs.entities(),
                &ecs.read_storage::<Pos>(),
                ecs.read_storage::<Ori>().maybe(),
                &ecs.read_storage::<Body>(),
                ecs.read_storage::<Health>().maybe(),
                ecs.read_storage::<Loadout>().maybe(),
                ecs.read_storage::<Scale>().maybe(),
            )
            .join()
            // Don't render dead entities
            .filter(|(_, _, _, _, health, _, _)| health.map_or(true, |h| !h.is_dead))
            .for_each(|(entity, pos, _, body, _, loadout, _)| {
                if let Some((locals, bone_consts, model, _)) = self.get_model_for_render(
                    tick,
                    camera,
                    None,
                    entity,
                    body,
                    loadout,
                    false,
                    pos.0,
                    figure_lod_render_distance,
                    |state| state.can_shadow_sun(),
                ) {
                    renderer.render_figure_shadow_directed(
                        model,
                        global,
                        locals,
                        bone_consts,
                        &global.shadow_mats,
                    );
                }
            });
        }
    }

    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn render(
        &self,
        renderer: &mut Renderer,
        state: &State,
        player_entity: EcsEntity,
        tick: u64,
        global: &GlobalModel,
        lod: &LodData,
        (camera, figure_lod_render_distance): CameraData,
    ) {
        span!(_guard, "render", "FigureManager::render");
        let ecs = state.ecs();

        let character_state_storage = state.read_storage::<common::comp::CharacterState>();
        let character_state = character_state_storage.get(player_entity);

        for (entity, pos, _, body, _, loadout, _) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            ecs.read_storage::<Ori>().maybe(),
            &ecs.read_storage::<Body>(),
            ecs.read_storage::<Health>().maybe(),
            ecs.read_storage::<Loadout>().maybe(),
            ecs.read_storage::<Scale>().maybe(),
        )
            .join()
        // Don't render dead entities
        .filter(|(_, _, _, _, health, _, _)| health.map_or(true, |h| !h.is_dead))
        {
            let is_player = entity == player_entity;

            if !is_player {
                if let Some((locals, bone_consts, model, col_lights)) = self.get_model_for_render(
                    tick,
                    camera,
                    character_state,
                    entity,
                    body,
                    loadout,
                    false,
                    pos.0,
                    figure_lod_render_distance,
                    |state| state.visible(),
                ) {
                    renderer.render_figure(model, &col_lights, global, locals, bone_consts, lod);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn render_player(
        &self,
        renderer: &mut Renderer,
        state: &State,
        player_entity: EcsEntity,
        tick: u64,
        global: &GlobalModel,
        lod: &LodData,
        (camera, figure_lod_render_distance): CameraData,
    ) {
        span!(_guard, "render_player", "FigureManager::render_player");
        let ecs = state.ecs();

        let character_state_storage = state.read_storage::<common::comp::CharacterState>();
        let character_state = character_state_storage.get(player_entity);

        if let (Some(pos), Some(body)) = (
            ecs.read_storage::<Pos>().get(player_entity),
            ecs.read_storage::<Body>().get(player_entity),
        ) {
            let healths = state.read_storage::<Health>();
            let health = healths.get(player_entity);
            if health.map_or(false, |h| h.is_dead) {
                return;
            }

            let loadout_storage = ecs.read_storage::<Loadout>();
            let loadout = loadout_storage.get(player_entity);

            if let Some((locals, bone_consts, model, col_lights)) = self.get_model_for_render(
                tick,
                camera,
                character_state,
                player_entity,
                body,
                loadout,
                true,
                pos.0,
                figure_lod_render_distance,
                |state| state.visible(),
            ) {
                renderer.render_player(model, &col_lights, global, locals, bone_consts, lod);
                renderer.render_player_shadow(
                    model,
                    &col_lights,
                    global,
                    bone_consts,
                    lod,
                    &global.shadow_mats,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    fn get_model_for_render(
        &self,
        tick: u64,
        camera: &Camera,
        character_state: Option<&CharacterState>,
        entity: EcsEntity,
        body: &Body,
        loadout: Option<&Loadout>,
        is_player: bool,
        pos: vek::Vec3<f32>,
        figure_lod_render_distance: f32,
        filter_state: impl Fn(&FigureStateMeta) -> bool,
    ) -> Option<FigureModelRef> {
        let player_camera_mode = if is_player {
            camera.get_mode()
        } else {
            CameraMode::default()
        };
        let focus_pos = camera.get_focus_pos();
        let cam_pos = camera.dependents().cam_pos + focus_pos.map(|e| e.trunc());
        let character_state = if is_player { character_state } else { None };

        let FigureMgr {
            col_lights: ref col_lights_,
            model_cache,
            theropod_model_cache,
            quadruped_small_model_cache,
            quadruped_medium_model_cache,
            quadruped_low_model_cache,
            bird_medium_model_cache,
            bird_small_model_cache,
            dragon_model_cache,
            fish_medium_model_cache,
            fish_small_model_cache,
            biped_large_model_cache,
            object_model_cache,
            golem_model_cache,
            states:
                FigureMgrStates {
                    character_states,
                    quadruped_small_states,
                    quadruped_medium_states,
                    quadruped_low_states,
                    bird_medium_states,
                    fish_medium_states,
                    theropod_states,
                    dragon_states,
                    bird_small_states,
                    fish_small_states,
                    biped_large_states,
                    golem_states,
                    object_states,
                },
        } = self;
        let col_lights = &*col_lights_;
        if let Some((locals, bone_consts, model_entry)) = match body {
            Body::Humanoid(body) => character_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
            Body::QuadrupedSmall(body) => quadruped_small_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        quadruped_small_model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
            Body::QuadrupedMedium(body) => quadruped_medium_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        quadruped_medium_model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
            Body::QuadrupedLow(body) => quadruped_low_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        quadruped_low_model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
            Body::BirdMedium(body) => bird_medium_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        bird_medium_model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
            Body::FishMedium(body) => fish_medium_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        fish_medium_model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
            Body::Theropod(body) => theropod_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        theropod_model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
            Body::Dragon(body) => dragon_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        dragon_model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
            Body::BirdSmall(body) => bird_small_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        bird_small_model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
            Body::FishSmall(body) => fish_small_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        fish_small_model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
            Body::BipedLarge(body) => biped_large_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        biped_large_model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
            Body::Golem(body) => golem_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        golem_model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
            Body::Object(body) => object_states
                .get(&entity)
                .filter(|state| filter_state(&*state))
                .map(move |state| {
                    (
                        state.locals(),
                        state.bone_consts(),
                        object_model_cache.get_model(
                            col_lights,
                            *body,
                            loadout,
                            tick,
                            player_camera_mode,
                            character_state,
                        ),
                    )
                }),
        } {
            let model_entry = model_entry?;

            let figure_low_detail_distance = figure_lod_render_distance * 0.75;
            let figure_mid_detail_distance = figure_lod_render_distance * 0.5;

            let model = if pos.distance_squared(cam_pos) > figure_low_detail_distance.powi(2) {
                &model_entry.models[2]
            } else if pos.distance_squared(cam_pos) > figure_mid_detail_distance.powi(2) {
                &model_entry.models[1]
            } else {
                &model_entry.models[0]
            };

            Some((locals, bone_consts, model, col_lights_.texture(model_entry)))
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
    // col_lights: Texture<ColLightFmt>,
}

impl FigureColLights {
    pub fn new(renderer: &mut Renderer) -> Self {
        let atlas = Self::make_atlas(renderer).expect("Failed to create texture atlas for figures");
        Self {
            atlas, /* col_lights, */
        }
    }

    /// Find the correct texture for this model entry.
    pub fn texture<'a, const N: usize>(
        &'a self,
        model: &'a FigureModelEntry<N>,
    ) -> &'a Texture<ColLightFmt> {
        /* &self.col_lights */
        &model.col_lights
    }

    /// NOTE: Panics if the opaque model's length does not fit in a u32.
    /// This is part of the function contract.
    ///
    /// NOTE: Panics if the vertex range bounds are not in range of the opaque
    /// model stored in the BoneMeshes parameter.  This is part of the
    /// function contract.
    pub fn create_figure<const N: usize>(
        &mut self,
        renderer: &mut Renderer,
        (tex, tex_size): ColLightInfo,
        (opaque, bounds): (Mesh<TerrainPipeline>, math::Aabb<f32>),
        vertex_range: [Range<u32>; N],
    ) -> Result<FigureModelEntry<N>, RenderError> {
        span!(_guard, "create_figure", "FigureColLights::create_figure");
        let atlas = &mut self.atlas;
        let allocation = atlas
            .allocate(guillotiere::Size::new(
                i32::from(tex_size.x),
                i32::from(tex_size.y),
            ))
            .expect("Not yet implemented: allocate new atlas on allocation failure.");
        let col_lights = ShadowPipeline::create_col_lights(renderer, (tex, tex_size))?;
        let model_len = u32::try_from(opaque.vertices().len())
            .expect("The model size for this figure does not fit in a u32!");
        let model = renderer.create_model(&opaque)?;

        Ok(FigureModelEntry {
            _bounds: bounds,
            models: vertex_range.map(|range| {
                assert!(
                    range.start <= range.end && range.end <= model_len,
                    "The provided vertex range for figure mesh {:?} does not fit in the model, \
                     which is of size {:?}!",
                    range,
                    model_len
                );
                FigureModel {
                    opaque: model.submodel(range),
                }
            }),
            col_lights,
            allocation,
        })
    }

    fn make_atlas(renderer: &mut Renderer) -> Result<AtlasAllocator, RenderError> {
        let max_texture_size = renderer.max_texture_size();
        let atlas_size =
            guillotiere::Size::new(i32::from(max_texture_size), i32::from(max_texture_size));
        let atlas = AtlasAllocator::with_options(atlas_size, &guillotiere::AllocatorOptions {
            // TODO: Verify some good empirical constants.
            small_size_threshold: 32,
            large_size_threshold: 256,
            ..guillotiere::AllocatorOptions::default()
        });
        // TODO: Consider using a single texture atlas to store all figures, much like
        // we do for terrain chunks.  We previously avoided this due to
        // perceived performance degradation for the figure use case, but with a
        // smaller atlas size this may be less likely.
        /* let texture = renderer.create_texture_raw(
            gfx::texture::Kind::D2(
                max_texture_size,
                max_texture_size,
                gfx::texture::AaMode::Single,
            ),
            1 as gfx::texture::Level,
            gfx::memory::Bind::SHADER_RESOURCE,
            gfx::memory::Usage::Dynamic,
            (0, 0),
            gfx::format::Swizzle::new(),
            gfx::texture::SamplerInfo::new(
                gfx::texture::FilterMethod::Bilinear,
                gfx::texture::WrapMode::Clamp,
            ),
        )?;
        Ok((atlas, texture)) */
        Ok(atlas)
    }
}

pub struct FigureStateMeta {
    bone_consts: Consts<FigureBoneData>,
    locals: Consts<FigureLocals>,
    lantern_offset: anim::vek::Vec3<f32>,
    state_time: f64,
    last_ori: anim::vek::Vec3<f32>,
    lpindex: u8,
    can_shadow_sun: bool,
    visible: bool,
    last_pos: Option<anim::vek::Vec3<f32>>,
    avg_vel: anim::vek::Vec3<f32>,
    last_light: f32,
    last_glow: f32,
}

impl FigureStateMeta {
    pub fn visible(&self) -> bool { self.visible }

    pub fn can_shadow_sun(&self) -> bool {
        // Either visible, or explicitly a shadow caster.
        self.visible || self.can_shadow_sun
    }
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
        let mut buf = [Default::default(); anim::MAX_BONE_COUNT];
        let lantern_offset =
            anim::compute_matrices(&skeleton, anim::vek::Mat4::identity(), &mut buf);
        let bone_consts = figure_bone_data_from_anim(&buf);
        Self {
            meta: FigureStateMeta {
                bone_consts: renderer.create_consts(bone_consts).unwrap(),
                locals: renderer.create_consts(&[FigureLocals::default()]).unwrap(),
                lantern_offset,
                state_time: 0.0,
                last_ori: anim::vek::Vec3::zero(),
                lpindex: 0,
                visible: false,
                can_shadow_sun: false,
                last_pos: None,
                avg_vel: anim::vek::Vec3::zero(),
                last_light: 1.0,
                last_glow: 0.0,
            },
            skeleton,
        }
    }

    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn update<const N: usize>(
        &mut self,
        renderer: &mut Renderer,
        pos: anim::vek::Vec3<f32>,
        ori: anim::vek::Vec3<f32>,
        scale: f32,
        col: vek::Rgba<f32>,
        dt: f32,
        state_animation_rate: f32,
        model: Option<&FigureModelEntry<N>>,
        _lpindex: u8,
        _visible: bool,
        is_player: bool,
        _camera: &Camera,
        buf: &mut [anim::FigureBoneData; anim::MAX_BONE_COUNT],
        terrain: Option<&Terrain>,
    ) {
        // NOTE: As long as update() always gets called after get_or_create_model(), and
        // visibility is not set again until after the model is rendered, we
        // know we don't pair the character model with invalid model state.
        //
        // Currently, the only exception to this during normal gameplay is in the very
        // first tick after a model is created (so there's no `last_character`
        // state).  So in theory, we could have incorrect model data during this
        // tick.  It is possible to resolve this in a few ways, but since
        // currently we don't actually use the model state for anything, we
        // currently ignore this potential issue.
        //
        // FIXME: Address the above at some point.
        let model = if let Some(model) = model {
            model
        } else {
            self.visible = false;
            return;
        };

        // Approximate as a sphere with radius equal to the
        // largest dimension (if we were exact, it should just be half the largest
        // dimension, but we're not, so we double it and use size() instead of
        // half_size()).
        /* let radius = vek::Extent3::<f32>::from(model.bounds.half_size()).reduce_partial_max();
        let _bounds = BoundingSphere::new(pos.into_array(), scale * 0.8 * radius); */

        self.last_ori = vek::Lerp::lerp(self.last_ori, ori, 15.0 * dt);

        self.state_time += (dt * state_animation_rate) as f64;

        let mat = anim::vek::Mat4::rotation_z(-ori.x.atan2(ori.y))
            * anim::vek::Mat4::rotation_x(ori.z.atan2(anim::vek::Vec2::from(ori).magnitude()))
            * anim::vek::Mat4::scaling_3d(anim::vek::Vec3::from(0.8 * scale));

        let atlas_offs = model.allocation.rectangle.min;

        let (light, glow) = terrain
            .map(|t| {
                // Sample the location a little above to avoid clipping into terrain
                // TODO: Try to make this faster? It might be fine though
                let wpos = Vec3::from(pos.into_array()) + Vec3::unit_z();

                let wposi = wpos.map(|e: f32| e.floor() as i32);

                // TODO: Fix this up enough to make it work
                /*
                let sample = |off| {
                    let off = off * wpos.map(|e| (e.fract() - 0.5).signum() as i32);
                    Vec2::new(t.light_at_wpos(wposi + off), t.glow_at_wpos(wposi + off))
                };

                let s_000 = sample(Vec3::new(0, 0, 0));
                let s_100 = sample(Vec3::new(1, 0, 0));
                let s_010 = sample(Vec3::new(0, 1, 0));
                let s_110 = sample(Vec3::new(1, 1, 0));
                let s_001 = sample(Vec3::new(0, 0, 1));
                let s_101 = sample(Vec3::new(1, 0, 1));
                let s_011 = sample(Vec3::new(0, 1, 1));
                let s_111 = sample(Vec3::new(1, 1, 1));
                let s_00 = Lerp::lerp(s_000, s_001, (wpos.z.fract() - 0.5).abs() * 2.0);
                let s_10 = Lerp::lerp(s_100, s_101, (wpos.z.fract() - 0.5).abs() * 2.0);
                let s_01 = Lerp::lerp(s_010, s_011, (wpos.z.fract() - 0.5).abs() * 2.0);
                let s_11 = Lerp::lerp(s_110, s_111, (wpos.z.fract() - 0.5).abs() * 2.0);
                let s_0 = Lerp::lerp(s_00, s_01, (wpos.y.fract() - 0.5).abs() * 2.0);
                let s_1 = Lerp::lerp(s_10, s_11, (wpos.y.fract() - 0.5).abs() * 2.0);
                let s = Lerp::lerp(s_10, s_11, (wpos.x.fract() - 0.5).abs() * 2.0);
                */

                Vec2::new(t.light_at_wpos(wposi), t.glow_at_wpos(wposi)).into_tuple()
            })
            .unwrap_or((1.0, 0.0));
        // Fade between light and glow levels
        // TODO: Making this temporal rather than spatial is a bit dumb but it's a very
        // subtle difference
        self.last_light = vek::Lerp::lerp(self.last_light, light, 16.0 * dt);
        self.last_glow = vek::Lerp::lerp(self.last_glow, glow, 16.0 * dt);

        let locals = FigureLocals::new(
            mat,
            col.rgb(),
            pos,
            vek::Vec2::new(atlas_offs.x, atlas_offs.y),
            is_player,
            self.last_light,
            self.last_glow,
        );
        renderer.update_consts(&mut self.locals, &[locals]).unwrap();

        let lantern_offset = anim::compute_matrices(&self.skeleton, mat, buf);

        let new_bone_consts = figure_bone_data_from_anim(buf);

        renderer
            .update_consts(
                &mut self.meta.bone_consts,
                &new_bone_consts[0..S::BONE_COUNT],
            )
            .unwrap();
        self.lantern_offset = lantern_offset;

        let smoothing = (5.0 * dt).min(1.0);
        if let Some(last_pos) = self.last_pos {
            self.avg_vel = (1.0 - smoothing) * self.avg_vel + smoothing * (pos - last_pos) / dt;
        }
        self.last_pos = Some(pos);
    }

    pub fn locals(&self) -> &Consts<FigureLocals> { &self.locals }

    pub fn bone_consts(&self) -> &Consts<FigureBoneData> { &self.bone_consts }

    pub fn skeleton_mut(&mut self) -> &mut S { &mut self.skeleton }
}

fn figure_bone_data_from_anim(
    mats: &[anim::FigureBoneData; anim::MAX_BONE_COUNT],
) -> &[FigureBoneData] {
    gfx::memory::cast_slice(mats)
}
