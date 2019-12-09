mod cache;
mod load;

pub use cache::FigureModelCache;
pub use load::load_mesh; // TODO: Don't make this public.

use crate::{
    anim::{
        self, biped_large::BipedLargeSkeleton, bird_medium::BirdMediumSkeleton,
        bird_small::BirdSmallSkeleton, character::CharacterSkeleton, dragon::DragonSkeleton,
        fish_medium::FishMediumSkeleton, fish_small::FishSmallSkeleton, object::ObjectSkeleton,
        quadruped_medium::QuadrupedMediumSkeleton, quadruped_small::QuadrupedSmallSkeleton,
        Animation, Skeleton,
    },
    render::{Consts, FigureBoneData, FigureLocals, Globals, Light, Renderer, Shadow},
    scene::camera::{Camera, CameraMode},
};
use client::Client;
use common::{
    comp::{
        ActionState::*, Body, CharacterState, ItemKind, Last, MovementState::*, Ori, Pos, Scale,
        Stats, Vel,
    },
    terrain::TerrainChunk,
    vol::RectRasterableVol,
};
use hashbrown::HashMap;
use log::trace;
use specs::{Entity as EcsEntity, Join};
use vek::*;

const DAMAGE_FADE_COEFFICIENT: f64 = 5.0;

pub struct FigureMgr {
    model_cache: FigureModelCache,
    character_states: HashMap<EcsEntity, FigureState<CharacterSkeleton>>,
    quadruped_small_states: HashMap<EcsEntity, FigureState<QuadrupedSmallSkeleton>>,
    quadruped_medium_states: HashMap<EcsEntity, FigureState<QuadrupedMediumSkeleton>>,
    bird_medium_states: HashMap<EcsEntity, FigureState<BirdMediumSkeleton>>,
    fish_medium_states: HashMap<EcsEntity, FigureState<FishMediumSkeleton>>,
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
            character_states: HashMap::new(),
            quadruped_small_states: HashMap::new(),
            quadruped_medium_states: HashMap::new(),
            bird_medium_states: HashMap::new(),
            fish_medium_states: HashMap::new(),
            dragon_states: HashMap::new(),
            bird_small_states: HashMap::new(),
            fish_small_states: HashMap::new(),
            biped_large_states: HashMap::new(),
            object_states: HashMap::new(),
        }
    }

    pub fn clean(&mut self, tick: u64) {
        self.model_cache.clean(tick);
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, client: &Client) {
        let time = client.state().get_time();
        let tick = client.get_tick();
        let ecs = client.state().ecs();
        let view_distance = client.view_distance().unwrap_or(1);
        let dt = client.state().get_delta_time();
        // Get player position.
        let player_pos = ecs
            .read_storage::<Pos>()
            .get(client.entity())
            .map_or(Vec3::zero(), |pos| pos.0);

        for (entity, pos, ori, scale, body, character, last_character, stats) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Ori>(),
            ecs.read_storage::<Scale>().maybe(),
            &ecs.read_storage::<Body>(),
            ecs.read_storage::<CharacterState>().maybe(),
            ecs.read_storage::<Last<CharacterState>>().maybe(),
            ecs.read_storage::<Stats>().maybe(),
        )
            .join()
        {
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
                    }
                    Body::QuadrupedSmall(_) => {
                        self.quadruped_small_states.remove(&entity);
                    }
                    Body::QuadrupedMedium(_) => {
                        self.quadruped_medium_states.remove(&entity);
                    }
                    Body::BirdMedium(_) => {
                        self.bird_medium_states.remove(&entity);
                    }
                    Body::FishMedium(_) => {
                        self.fish_medium_states.remove(&entity);
                    }
                    Body::Dragon(_) => {
                        self.dragon_states.remove(&entity);
                    }
                    Body::BirdSmall(_) => {
                        self.bird_small_states.remove(&entity);
                    }
                    Body::FishSmall(_) => {
                        self.fish_small_states.remove(&entity);
                    }
                    Body::BipedLarge(_) => {
                        self.biped_large_states.remove(&entity);
                    }
                    Body::Object(_) => {
                        self.object_states.remove(&entity);
                    }
                }
                continue;
            } else if vd_frac > 1.0 {
                continue;
            }

            // Change in health as color!
            let col = stats
                .map(|s| {
                    Rgba::broadcast(1.0)
                        + Rgba::new(2.0, 2.0, 2.0, 0.0).map(|c| {
                            (c / (1.0 + DAMAGE_FADE_COEFFICIENT * s.health.last_change.0)) as f32
                        })
                })
                .unwrap_or(Rgba::broadcast(1.0));

            let scale = scale.map(|s| s.0).unwrap_or(1.0);

            let skeleton_attr = &self
                .model_cache
                .get_or_create_model(
                    renderer,
                    *body,
                    stats.map(|s| &s.equipment),
                    tick,
                    CameraMode::default(),
                    None,
                )
                .1;

            let mut movement_animation_rate = 1.0;
            let mut action_animation_rate = 1.0;

            let vel = ecs
                .read_storage::<Vel>()
                .get(entity)
                .cloned()
                .unwrap_or_default();

            let active_tool_kind = if let Some(ItemKind::Tool { kind, .. }) = stats
                .and_then(|s| s.equipment.main.as_ref())
                .map(|i| &i.kind)
            {
                Some(*kind)
            } else {
                None
            };

            match body {
                Body::Humanoid(_) => {
                    let state = self
                        .character_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, CharacterSkeleton::new()));
                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.is_same_movement(&last_character.0) {
                        state.movement_time = 0.0;
                    }
                    if !character.is_same_action(&last_character.0) {
                        state.action_time = 0.0;
                    }

                    let target_base = match &character.movement {
                        Stand => anim::character::StandAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (active_tool_kind, time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Run => anim::character::RunAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (active_tool_kind, vel.0, ori.0, state.last_ori, time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Jump | Fall => anim::character::JumpAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (active_tool_kind, time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Glide => anim::character::GlidingAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (active_tool_kind, vel.0, ori.0, state.last_ori, time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Swim => anim::character::SwimAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (active_tool_kind, vel.0.magnitude(), ori.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Climb => anim::character::ClimbAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (active_tool_kind, vel.0, ori.0, time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Sit => anim::character::SitAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (active_tool_kind, time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                    };
                    let target_bones = match (&character.movement, &character.action) {
                        (Stand, Wield { .. }) => anim::character::CidleAnimation::update_skeleton(
                            &target_base,
                            (active_tool_kind, time),
                            state.action_time,
                            &mut action_animation_rate,
                            skeleton_attr,
                        ),
                        (Stand, Block { .. }) => {
                            anim::character::BlockIdleAnimation::update_skeleton(
                                &target_base,
                                (active_tool_kind, time),
                                state.action_time,
                                &mut action_animation_rate,
                                skeleton_attr,
                            )
                        }
                        (_, Attack { .. }) => anim::character::AttackAnimation::update_skeleton(
                            &target_base,
                            (active_tool_kind, time),
                            state.action_time,
                            &mut action_animation_rate,
                            skeleton_attr,
                        ),
                        (_, Wield { .. }) => anim::character::WieldAnimation::update_skeleton(
                            &target_base,
                            (active_tool_kind, vel.0.magnitude(), time),
                            state.action_time,
                            &mut action_animation_rate,
                            skeleton_attr,
                        ),
                        (_, Roll { .. }) => anim::character::RollAnimation::update_skeleton(
                            &target_base,
                            (active_tool_kind, time),
                            state.action_time,
                            &mut action_animation_rate,
                            skeleton_attr,
                        ),
                        (_, Block { .. }) => anim::character::BlockAnimation::update_skeleton(
                            &target_base,
                            (active_tool_kind, time),
                            state.action_time,
                            &mut action_animation_rate,
                            skeleton_attr,
                        ),
                        (_, Charge { .. }) => anim::character::ChargeAnimation::update_skeleton(
                            &target_base,
                            (active_tool_kind, time),
                            state.action_time,
                            &mut action_animation_rate,
                            skeleton_attr,
                        ),
                        _ => target_base,
                    };

                    state.skeleton.interpolate(&target_bones, dt);
                    state.update(
                        renderer,
                        pos.0,
                        vel.0,
                        ori.0,
                        scale,
                        col,
                        dt,
                        movement_animation_rate,
                        action_animation_rate,
                    );
                }
                Body::QuadrupedSmall(_) => {
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

                    if !character.is_same_movement(&last_character.0) {
                        state.movement_time = 0.0;
                    }

                    let target_base = match character.movement {
                        Stand => anim::quadruped_small::IdleAnimation::update_skeleton(
                            &QuadrupedSmallSkeleton::new(),
                            time,
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Run => anim::quadruped_small::RunAnimation::update_skeleton(
                            &QuadrupedSmallSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Jump => anim::quadruped_small::JumpAnimation::update_skeleton(
                            &QuadrupedSmallSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        vel.0,
                        ori.0,
                        scale,
                        col,
                        dt,
                        movement_animation_rate,
                        action_animation_rate,
                    );
                }
                Body::QuadrupedMedium(_) => {
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

                    if !character.is_same_movement(&last_character.0) {
                        state.movement_time = 0.0;
                    }

                    let target_base = match character.movement {
                        Stand => anim::quadruped_medium::IdleAnimation::update_skeleton(
                            &QuadrupedMediumSkeleton::new(),
                            time,
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Run => anim::quadruped_medium::RunAnimation::update_skeleton(
                            &QuadrupedMediumSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Jump => anim::quadruped_medium::JumpAnimation::update_skeleton(
                            &QuadrupedMediumSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        vel.0,
                        ori.0,
                        scale,
                        col,
                        dt,
                        movement_animation_rate,
                        action_animation_rate,
                    );
                }
                Body::BirdMedium(_) => {
                    let state = self
                        .bird_medium_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, BirdMediumSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.is_same_movement(&last_character.0) {
                        state.movement_time = 0.0;
                    }

                    let target_base = match character.movement {
                        Stand => anim::bird_medium::IdleAnimation::update_skeleton(
                            &BirdMediumSkeleton::new(),
                            time,
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Run => anim::bird_medium::RunAnimation::update_skeleton(
                            &BirdMediumSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Jump => anim::bird_medium::JumpAnimation::update_skeleton(
                            &BirdMediumSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        vel.0,
                        ori.0,
                        scale,
                        col,
                        dt,
                        movement_animation_rate,
                        action_animation_rate,
                    );
                }
                Body::FishMedium(_) => {
                    let state = self
                        .fish_medium_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, FishMediumSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.is_same_movement(&last_character.0) {
                        state.movement_time = 0.0;
                    }

                    let target_base = match character.movement {
                        Stand => anim::fish_medium::IdleAnimation::update_skeleton(
                            &FishMediumSkeleton::new(),
                            time,
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Run => anim::fish_medium::RunAnimation::update_skeleton(
                            &FishMediumSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Jump => anim::fish_medium::JumpAnimation::update_skeleton(
                            &FishMediumSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        vel.0,
                        ori.0,
                        scale,
                        col,
                        dt,
                        movement_animation_rate,
                        action_animation_rate,
                    );
                }
                Body::Dragon(_) => {
                    let state = self
                        .dragon_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, DragonSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.is_same_movement(&last_character.0) {
                        state.movement_time = 0.0;
                    }

                    let target_base = match character.movement {
                        Stand => anim::dragon::IdleAnimation::update_skeleton(
                            &DragonSkeleton::new(),
                            time,
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Run => anim::dragon::RunAnimation::update_skeleton(
                            &DragonSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Jump => anim::dragon::JumpAnimation::update_skeleton(
                            &DragonSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        vel.0,
                        ori.0,
                        scale,
                        col,
                        dt,
                        movement_animation_rate,
                        action_animation_rate,
                    );
                }
                Body::BirdSmall(_) => {
                    let state = self
                        .bird_small_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, BirdSmallSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.is_same_movement(&last_character.0) {
                        state.movement_time = 0.0;
                    }

                    let target_base = match character.movement {
                        Stand => anim::bird_small::IdleAnimation::update_skeleton(
                            &BirdSmallSkeleton::new(),
                            time,
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Run => anim::bird_small::RunAnimation::update_skeleton(
                            &BirdSmallSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Jump => anim::bird_small::JumpAnimation::update_skeleton(
                            &BirdSmallSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        vel.0,
                        ori.0,
                        scale,
                        col,
                        dt,
                        movement_animation_rate,
                        action_animation_rate,
                    );
                }
                Body::FishSmall(_) => {
                    let state = self
                        .fish_small_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, FishSmallSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.is_same_movement(&last_character.0) {
                        state.movement_time = 0.0;
                    }

                    let target_base = match character.movement {
                        Stand => anim::fish_small::IdleAnimation::update_skeleton(
                            &FishSmallSkeleton::new(),
                            time,
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Run => anim::fish_small::RunAnimation::update_skeleton(
                            &FishSmallSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Jump => anim::fish_small::JumpAnimation::update_skeleton(
                            &FishSmallSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        vel.0,
                        ori.0,
                        scale,
                        col,
                        dt,
                        movement_animation_rate,
                        action_animation_rate,
                    );
                }
                Body::BipedLarge(_) => {
                    let state = self
                        .biped_large_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, BipedLargeSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.is_same_movement(&last_character.0) {
                        state.movement_time = 0.0;
                    }

                    let target_base = match character.movement {
                        Stand => anim::biped_large::IdleAnimation::update_skeleton(
                            &BipedLargeSkeleton::new(),
                            time,
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Run => anim::biped_large::RunAnimation::update_skeleton(
                            &BipedLargeSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),
                        Jump => anim::biped_large::JumpAnimation::update_skeleton(
                            &BipedLargeSkeleton::new(),
                            (vel.0.magnitude(), time),
                            state.movement_time,
                            &mut movement_animation_rate,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(
                        renderer,
                        pos.0,
                        vel.0,
                        ori.0,
                        scale,
                        col,
                        dt,
                        movement_animation_rate,
                        action_animation_rate,
                    );
                }
                Body::Object(_) => {
                    let state = self
                        .object_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, ObjectSkeleton::new()));

                    state.skeleton = state.skeleton_mut().clone();
                    state.update(
                        renderer,
                        pos.0,
                        vel.0,
                        ori.0,
                        scale,
                        col,
                        dt,
                        movement_animation_rate,
                        action_animation_rate,
                    );
                }
            }
        }

        // Clear states that have dead entities.
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
        client: &mut Client,
        globals: &Consts<Globals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
        camera: &Camera,
    ) {
        let tick = client.get_tick();
        let ecs = client.state().ecs();

        let frustum = camera.frustum(client);

        let character_state_storage = client
            .state()
            .read_storage::<common::comp::CharacterState>();
        let character_state = character_state_storage.get(client.entity());

        for (entity, _, _, body, stats, _) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Ori>(),
            &ecs.read_storage::<Body>(),
            ecs.read_storage::<Stats>().maybe(),
            ecs.read_storage::<Scale>().maybe(),
        )
            .join()
            // Don't render figures outside of frustum (camera viewport, max draw distance is farplane)
            .filter(|(_, pos, _, _, _, scale)| {
                frustum.sphere_intersecting(
                    &pos.0.x,
                    &pos.0.y,
                    &pos.0.z,
                    &(scale.unwrap_or(&Scale(1.0)).0 * 2.0),
                )
            })
            // Don't render dead entities
            .filter(|(_, _, _, _, stats, _)| stats.map_or(true, |s| !s.is_dead))
        {
            if let Some((locals, bone_consts)) = match body {
                Body::Humanoid(_) => self
                    .character_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::QuadrupedSmall(_) => self
                    .quadruped_small_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::QuadrupedMedium(_) => self
                    .quadruped_medium_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::BirdMedium(_) => self
                    .bird_medium_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::FishMedium(_) => self
                    .fish_medium_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::Dragon(_) => self
                    .dragon_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::BirdSmall(_) => self
                    .bird_small_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::FishSmall(_) => self
                    .fish_small_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::BipedLarge(_) => self
                    .biped_large_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::Object(_) => self
                    .object_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
            } {
                let is_player = entity == client.entity();

                let player_camera_mode = if is_player {
                    camera.get_mode()
                } else {
                    CameraMode::default()
                };

                let model = &self
                    .model_cache
                    .get_or_create_model(
                        renderer,
                        *body,
                        stats.map(|s| &s.equipment),
                        tick,
                        player_camera_mode,
                        if is_player { character_state } else { None },
                    )
                    .0;

                renderer.render_figure(model, globals, locals, bone_consts, lights, shadows);
            } else {
                trace!("Body has no saved figure");
            }
        }
    }
}

pub struct FigureState<S: Skeleton> {
    bone_consts: Consts<FigureBoneData>,
    locals: Consts<FigureLocals>,
    movement_time: f64,
    action_time: f64,
    skeleton: S,
    pos: Vec3<f32>,
    ori: Vec3<f32>,
    last_ori: Vec3<f32>,
}

impl<S: Skeleton> FigureState<S> {
    pub fn new(renderer: &mut Renderer, skeleton: S) -> Self {
        Self {
            bone_consts: renderer
                .create_consts(&skeleton.compute_matrices())
                .unwrap(),
            locals: renderer.create_consts(&[FigureLocals::default()]).unwrap(),
            movement_time: 0.0,
            action_time: 0.0,
            skeleton,
            pos: Vec3::zero(),
            ori: Vec3::zero(),
            last_ori: Vec3::zero(),
        }
    }

    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        ori: Vec3<f32>,
        scale: f32,
        col: Rgba<f32>,
        dt: f32,
        movement_rate: f32,
        action_rate: f32,
    ) {
        self.last_ori = Lerp::lerp(self.last_ori, ori, 15.0 * dt);

        // Update interpolation values
        if self.pos.distance_squared(pos) < 64.0 * 64.0 {
            self.pos = Lerp::lerp(self.pos, pos + vel * 0.03, 10.0 * dt);
            self.ori = Slerp::slerp(self.ori, ori, 5.0 * dt);
        } else {
            self.pos = pos;
            self.ori = ori;
        }

        self.movement_time += (dt * movement_rate) as f64;
        self.action_time += (dt * action_rate) as f64;

        let mat = Mat4::<f32>::identity()
            * Mat4::translation_3d(self.pos)
            * Mat4::rotation_z(-ori.x.atan2(ori.y))
            * Mat4::rotation_x(ori.z.atan2(Vec2::from(ori).magnitude()))
            * Mat4::scaling_3d(Vec3::from(0.8 * scale));

        let locals = FigureLocals::new(mat, col);
        renderer.update_consts(&mut self.locals, &[locals]).unwrap();

        renderer
            .update_consts(&mut self.bone_consts, &self.skeleton.compute_matrices())
            .unwrap();
    }

    pub fn locals(&self) -> &Consts<FigureLocals> {
        &self.locals
    }

    pub fn bone_consts(&self) -> &Consts<FigureBoneData> {
        &self.bone_consts
    }

    pub fn skeleton_mut(&mut self) -> &mut S {
        &mut self.skeleton
    }
}
