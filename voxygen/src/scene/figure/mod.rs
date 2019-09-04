mod cache;
mod load;

pub use cache::FigureModelCache;
pub use load::load_mesh; // TODO: Don't make this public.

use crate::{
    anim::{
        self, character::CharacterSkeleton, object::ObjectSkeleton, quadruped::QuadrupedSkeleton,
        quadrupedmedium::QuadrupedMediumSkeleton, Animation, Skeleton,
    },
    render::{Consts, FigureBoneData, FigureLocals, Globals, Light, Renderer},
    scene::camera::{Camera, CameraMode},
};
use client::Client;
use common::{
    comp::{
        ActionState::*, Body, CharacterState, Last, MovementState::*, Ori, Pos, Scale, Stats, Vel,
    },
    terrain::TerrainChunkSize,
    vol::VolSize,
};
use hashbrown::HashMap;
use log::debug;
use specs::{Entity as EcsEntity, Join};
use std::time::Instant;
use vek::*;

const DAMAGE_FADE_COEFFICIENT: f64 = 5.0;

pub struct FigureMgr {
    model_cache: FigureModelCache,
    character_states: HashMap<EcsEntity, FigureState<CharacterSkeleton>>,
    quadruped_states: HashMap<EcsEntity, FigureState<QuadrupedSkeleton>>,
    quadruped_medium_states: HashMap<EcsEntity, FigureState<QuadrupedMediumSkeleton>>,
    object_states: HashMap<EcsEntity, FigureState<ObjectSkeleton>>,
}

impl FigureMgr {
    pub fn new() -> Self {
        Self {
            model_cache: FigureModelCache::new(),
            character_states: HashMap::new(),
            quadruped_states: HashMap::new(),
            quadruped_medium_states: HashMap::new(),
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

        for (entity, pos, vel, ori, scale, body, character, last_character, stats) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Vel>(),
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
            let vd_frac = (pos.0 - player_pos)
                .map2(TerrainChunkSize::SIZE, |d, sz| d.abs() as f32 / sz as f32)
                .magnitude()
                / view_distance as f32;
            // Keep from re-adding/removing entities on the border of the vd
            if vd_frac > 1.2 {
                match body {
                    Body::Humanoid(_) => {
                        self.character_states.remove(&entity);
                    }
                    Body::Quadruped(_) => {
                        self.quadruped_states.remove(&entity);
                    }
                    Body::QuadrupedMedium(_) => {
                        self.quadruped_medium_states.remove(&entity);
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
                .and_then(|stats| stats.health.last_change)
                .map(|(_, time, _)| {
                    Rgba::broadcast(1.0)
                        + Rgba::new(0.0, -1.0, -1.0, 0.0)
                            .map(|c| (c / (1.0 + DAMAGE_FADE_COEFFICIENT * time)) as f32)
                })
                .unwrap_or(Rgba::broadcast(1.0));

            let scale = scale.map(|s| s.0).unwrap_or(1.0);

            let skeleton_attr = &self
                .model_cache
                .get_or_create_model(renderer, *body, stats.map(|s| &s.equipment), tick)
                .1;

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
                        state.last_movement_change = Instant::now();
                    }
                    if !character.is_same_action(&last_character.0) {
                        state.last_action_change = Instant::now();
                    }

                    let time_since_movement_change =
                        state.last_movement_change.elapsed().as_secs_f64();
                    let time_since_action_change = state.last_action_change.elapsed().as_secs_f64();

                    let target_base = match &character.movement {
                        Stand => anim::character::StandAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            time,
                            time_since_movement_change,
                            skeleton_attr,
                        ),
                        Run => anim::character::RunAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (vel.0.magnitude(), ori.0.magnitude(), time),
                            time_since_movement_change,
                            skeleton_attr,
                        ),
                        Jump => anim::character::JumpAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            time,
                            time_since_movement_change,
                            skeleton_attr,
                        ),
                        Roll { .. } => anim::character::RollAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            time,
                            time_since_movement_change,
                            skeleton_attr,
                        ),
                        Glide => anim::character::GlidingAnimation::update_skeleton(
                            &CharacterSkeleton::new(),
                            (vel.0.magnitude(), time),
                            time_since_movement_change,
                            skeleton_attr,
                        ),
                    };

                    let target_bones = match (&character.movement, &character.action) {
                        (Stand, Wield { .. }) => anim::character::CidleAnimation::update_skeleton(
                            &target_base,
                            time,
                            time_since_action_change,
                            skeleton_attr,
                        ),
                        (Stand, Block { .. }) => {
                            anim::character::BlockIdleAnimation::update_skeleton(
                                &target_base,
                                time,
                                time_since_action_change,
                                skeleton_attr,
                            )
                        }
                        (_, Attack { .. }) => anim::character::AttackAnimation::update_skeleton(
                            &target_base,
                            time,
                            time_since_action_change,
                            skeleton_attr,
                        ),
                        (_, Wield { .. }) => anim::character::WieldAnimation::update_skeleton(
                            &target_base,
                            (vel.0.magnitude(), time),
                            time_since_action_change,
                            skeleton_attr,
                        ),
                        (_, Block { .. }) => anim::character::BlockAnimation::update_skeleton(
                            &target_base,
                            time,
                            time_since_action_change,
                            skeleton_attr,
                        ),
                        _ => target_base,
                    };
                    state.skeleton.interpolate(&target_bones, dt);

                    state.update(renderer, pos.0, ori.0, scale, col, dt);
                }
                Body::Quadruped(_) => {
                    let state = self
                        .quadruped_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, QuadrupedSkeleton::new()));

                    let (character, last_character) = match (character, last_character) {
                        (Some(c), Some(l)) => (c, l),
                        _ => continue,
                    };

                    if !character.is_same_movement(&last_character.0) {
                        state.last_movement_change = Instant::now();
                    }

                    let time_since_movement_change =
                        state.last_movement_change.elapsed().as_secs_f64();

                    let target_base = match character.movement {
                        Stand => anim::quadruped::IdleAnimation::update_skeleton(
                            &QuadrupedSkeleton::new(),
                            time,
                            time_since_movement_change,
                            skeleton_attr,
                        ),
                        Run => anim::quadruped::RunAnimation::update_skeleton(
                            &QuadrupedSkeleton::new(),
                            (vel.0.magnitude(), time),
                            time_since_movement_change,
                            skeleton_attr,
                        ),
                        Jump => anim::quadruped::JumpAnimation::update_skeleton(
                            &QuadrupedSkeleton::new(),
                            (vel.0.magnitude(), time),
                            time_since_movement_change,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(renderer, pos.0, ori.0, scale, col, dt);
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
                        state.last_movement_change = Instant::now();
                    }

                    let time_since_movement_change =
                        state.last_movement_change.elapsed().as_secs_f64();

                    let target_base = match character.movement {
                        Stand => anim::quadrupedmedium::IdleAnimation::update_skeleton(
                            &QuadrupedMediumSkeleton::new(),
                            time,
                            time_since_movement_change,
                            skeleton_attr,
                        ),
                        Run => anim::quadrupedmedium::RunAnimation::update_skeleton(
                            &QuadrupedMediumSkeleton::new(),
                            (vel.0.magnitude(), time),
                            time_since_movement_change,
                            skeleton_attr,
                        ),
                        Jump => anim::quadrupedmedium::JumpAnimation::update_skeleton(
                            &QuadrupedMediumSkeleton::new(),
                            (vel.0.magnitude(), time),
                            time_since_movement_change,
                            skeleton_attr,
                        ),

                        // TODO!
                        _ => state.skeleton_mut().clone(),
                    };

                    state.skeleton.interpolate(&target_base, dt);
                    state.update(renderer, pos.0, ori.0, scale, col, dt);
                }
                Body::Object(_) => {
                    let state = self
                        .object_states
                        .entry(entity)
                        .or_insert_with(|| FigureState::new(renderer, ObjectSkeleton::new()));

                    state.skeleton = state.skeleton_mut().clone();
                    state.update(renderer, pos.0, ori.0, scale, col, dt);
                }
            }
        }

        // Clear states that have dead entities.
        self.character_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.quadruped_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.quadruped_medium_states
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
        camera: &Camera,
    ) {
        let tick = client.get_tick();
        let ecs = client.state().ecs();

        let frustum = camera.frustum(client);

        for (entity, _, _, _, body, stats, _) in (
            &ecs.entities(),
            &ecs.read_storage::<Pos>(),
            &ecs.read_storage::<Vel>(),
            &ecs.read_storage::<Ori>(),
            &ecs.read_storage::<Body>(),
            ecs.read_storage::<Stats>().maybe(),
            ecs.read_storage::<Scale>().maybe(),
        )
            .join()
            // Don't render figures outside of frustum (camera viewport, max draw distance is farplane)
            .filter(|(_, pos, _, _, _, _, scale)| {
                frustum.sphere_intersecting(
                    &pos.0.x,
                    &pos.0.y,
                    &pos.0.z,
                    &(scale.unwrap_or(&Scale(1.0)).0 * 2.0),
                )
            })
            // Don't render dead entities
            .filter(|(_, _, _, _, _, stats, _)| stats.map_or(true, |s| !s.is_dead))
        {
            if let Some((locals, bone_consts)) = match body {
                Body::Humanoid(_) => self
                    .character_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::Quadruped(_) => self
                    .quadruped_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::QuadrupedMedium(_) => self
                    .quadruped_medium_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
                Body::Object(_) => self
                    .object_states
                    .get(&entity)
                    .map(|state| (state.locals(), state.bone_consts())),
            } {
                let model = &self
                    .model_cache
                    .get_or_create_model(renderer, *body, stats.map(|s| &s.equipment), tick)
                    .0;

                // Don't render the player's body while in first person mode
                if camera.get_mode() == CameraMode::FirstPerson
                    && client
                        .state()
                        .read_storage::<Body>()
                        .get(client.entity())
                        .is_some()
                    && entity == client.entity()
                {
                    continue;
                }

                renderer.render_figure(model, globals, locals, bone_consts, lights);
            } else {
                debug!("Body has no saved figure");
            }
        }
    }
}

pub struct FigureState<S: Skeleton> {
    bone_consts: Consts<FigureBoneData>,
    locals: Consts<FigureLocals>,
    last_movement_change: Instant,
    last_action_change: Instant,
    skeleton: S,
    pos: Vec3<f32>,
    ori: Vec3<f32>,
}

impl<S: Skeleton> FigureState<S> {
    pub fn new(renderer: &mut Renderer, skeleton: S) -> Self {
        Self {
            bone_consts: renderer
                .create_consts(&skeleton.compute_matrices())
                .unwrap(),
            locals: renderer.create_consts(&[FigureLocals::default()]).unwrap(),
            last_movement_change: Instant::now(),
            last_action_change: Instant::now(),
            skeleton,
            pos: Vec3::zero(),
            ori: Vec3::zero(),
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
    ) {
        // Update interpolation values
        if self.pos.distance_squared(pos) < 64.0 * 64.0 {
            self.pos = Lerp::lerp(self.pos, pos, 15.0 * dt);
            self.ori = Slerp::slerp(self.ori, ori, 7.5 * dt);
        } else {
            self.pos = pos;
            self.ori = ori;
        }

        let mat = Mat4::<f32>::identity()
            * Mat4::translation_3d(self.pos)
            * Mat4::rotation_z(-ori.x.atan2(ori.y))
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
