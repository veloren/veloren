use crate::{
    anim::{
        character::{self, CharacterSkeleton},
        quadruped::{self, QuadrupedSkeleton},
        Animation, Skeleton,
    },
    mesh::Meshable,
    render::{
        Consts, FigureBoneData, FigureLocals, FigurePipeline, Globals, Mesh, Model, Renderer,
    },
    Error,
};
use client::Client;
use common::{
    assets,
    comp::{
        self,
        actor::{
            Belt, Chest, Draw, Foot, Hand, Head, Pants, PigChest, PigHead, PigLegL, PigLegR,
            Shoulder, Weapon,
        },
        Body, HumanoidBody, QuadrupedBody,
    },
    figure::Segment,
    msg,
};
use dot_vox::DotVoxData;
use specs::{Component, Entity as EcsEntity, Join, VecStorage};
use std::{collections::HashMap, f32};
use vek::*;

pub struct FigureModelCache {
    models: HashMap<Body, (Model<FigurePipeline>, u64)>,
}

impl FigureModelCache {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    pub fn get_or_create_model(
        &mut self,
        renderer: &mut Renderer,
        body: Body,
        tick: u64,
    ) -> &Model<FigurePipeline> {
        match self.models.get_mut(&body) {
            Some((model, last_used)) => {
                *last_used = tick;
            }
            None => {
                self.models.insert(
                    body,
                    (
                        {
                            let bone_meshes = match body {
                                Body::Humanoid(body) => [
                                    Some(Self::load_head(body.head)),
                                    Some(Self::load_chest(body.chest)),
                                    Some(Self::load_belt(body.belt)),
                                    Some(Self::load_pants(body.pants)),
                                    Some(Self::load_left_hand(body.hand)),
                                    Some(Self::load_right_hand(body.hand)),
                                    Some(Self::load_left_foot(body.foot)),
                                    Some(Self::load_right_foot(body.foot)),
                                    Some(Self::load_weapon(body.weapon)),
                                    Some(Self::load_left_shoulder(body.shoulder)),
                                    Some(Self::load_right_shoulder(body.shoulder)),
                                    Some(Self::load_draw(body.draw)),
                                    None,
                                    None,
                                    None,
                                    None,
                                ],
                                Body::Quadruped(body) => [
                                    Some(Self::load_pig_head(body.pig_head)),
                                    Some(Self::load_pig_chest(body.pig_chest)),
                                    Some(Self::load_pig_leg_lf(body.pig_leg_l)),
                                    Some(Self::load_pig_leg_rf(body.pig_leg_r)),
                                    Some(Self::load_pig_leg_lb(body.pig_leg_l)),
                                    Some(Self::load_pig_leg_rb(body.pig_leg_r)),
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                ],
                            };

                            let mut mesh = Mesh::new();
                            bone_meshes
                                .iter()
                                .enumerate()
                                .filter_map(|(i, bm)| bm.as_ref().map(|bm| (i, bm)))
                                .for_each(|(i, bone_mesh)| {
                                    mesh.push_mesh_map(bone_mesh, |vert| {
                                        vert.with_bone_idx(i as u8)
                                    })
                                });

                            renderer.create_model(&mesh).unwrap()
                        },
                        tick,
                    ),
                );
            }
        }

        &self.models[&body].0
    }

    pub fn clean(&mut self, tick: u64) {
        // TODO: Don't hard-code this.
        self.models
            .retain(|_, (_, last_used)| *last_used + 60 > tick);
    }

    // TODO: Don't make this public.
    pub fn load_mesh(filename: &str, position: Vec3<f32>) -> Mesh<FigurePipeline> {
        let full_path: String = ["/voxygen/voxel/", filename].concat();
        Segment::from(assets::load_expect::<DotVoxData>(full_path.as_str()).as_ref())
            .generate_mesh(position)
    }

    fn load_head(head: Head) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match head {
                Head::Default => "figure/head.vox",
            },
            Vec3::new(-7.0, -5.5, -6.0),
        )
    }

    fn load_chest(chest: Chest) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match chest {
                Chest::Default => "figure/body/chest_male.vox",
                Chest::Blue => "armor/chest/chest_blue.vox",
                Chest::Brown => "armor/chest/chest_brown.vox",
                Chest::Dark => "armor/chest/chest_dark.vox",
                Chest::Green => "armor/chest/chest_green.vox",
                Chest::Orange => "armor/chest/chest_orange.vox",
            },
            Vec3::new(-6.0, -3.5, 0.0),
        )
    }

    fn load_belt(belt: Belt) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match belt {
                //Belt::Default => "figure/body/belt_male.vox",
                Belt::Dark => "armor/belt/belt_dark.vox",
            },
            Vec3::new(-5.0, -3.5, 0.0),
        )
    }

    fn load_pants(pants: Pants) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match pants {
                Pants::Default => "figure/body/pants_male.vox",
                Pants::Blue => "armor/pants/pants_blue.vox",
                Pants::Brown => "armor/pants/pants_brown.vox",
                Pants::Dark => "armor/pants/pants_dark.vox",
                Pants::Green => "armor/pants/pants_green.vox",
                Pants::Orange => "armor/pants/pants_orange.vox",
            },
            Vec3::new(-5.0, -3.5, 0.0),
        )
    }

    fn load_left_hand(hand: Hand) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match hand {
                Hand::Default => "figure/body/hand.vox",
            },
            Vec3::new(2.0, 0.0, -7.0),
        )
    }

    fn load_right_hand(hand: Hand) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match hand {
                Hand::Default => "figure/body/hand.vox",
            },
            Vec3::new(2.0, 0.0, -7.0),
        )
    }

    fn load_left_foot(foot: Foot) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match foot {
                Foot::Default => "figure/body/foot.vox",
                Foot::Dark => "armor/foot/foot_dark.vox",
            },
            Vec3::new(2.5, -3.5, -9.0),
        )
    }

    fn load_right_foot(foot: Foot) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match foot {
                Foot::Default => "figure/body/foot.vox",
                Foot::Dark => "armor/foot/foot_dark.vox",
            },
            Vec3::new(2.5, -3.5, -9.0),
        )
    }

    fn load_weapon(weapon: Weapon) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match weapon {
                Weapon::Sword => "weapon/sword/sword_wood_2h.vox",
                // TODO actually match against other weapons and set the right model
                _ => "weapon/sword/sword_wood_2h.vox",               
            },
            Vec3::new(0.0, 0.0, -4.0),
        )
    }

    fn load_left_shoulder(shoulder: Shoulder) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match shoulder {
                Shoulder::Default => "armor/shoulder/shoulder_l_brown.vox",
            },
            Vec3::new(2.5, -0.5, 0.0),
        )
    }

    fn load_right_shoulder(shoulder: Shoulder) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match shoulder {
                Shoulder::Default => "armor/shoulder/shoulder_r_brown.vox",
            },
            Vec3::new(2.5, -0.5, 0.0),
        )
    }
    fn load_draw(draw: Draw) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match draw {
                Draw::Default => "object/glider.vox",
            },
            Vec3::new(-26.0, -26.0, -5.0),
        )
    }

    fn load_pig_head(pig_head: PigHead) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match pig_head {
                PigHead::Default => "npc/pig_purple/pighead.vox",
            },
            Vec3::new(-6.0, 4.5, 3.0),
        )
    }

    fn load_pig_chest(pig_chest: PigChest) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match pig_chest {
                PigChest::Default => "npc/pig_purple/pigchest.vox",
            },
            Vec3::new(-5.0, 4.5, 0.0),
        )
    }

    fn load_pig_leg_lf(pig_leg_l: PigLegL) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match pig_leg_l {
                PigLegL::Default => "npc/pig_purple/pigleg_l.vox",
            },
            Vec3::new(0.0, -1.0, -1.5),
        )
    }

    fn load_pig_leg_rf(pig_leg_r: PigLegR) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match pig_leg_r {
                PigLegR::Default => "npc/pig_purple/pigleg_r.vox",
            },
            Vec3::new(0.0, -1.0, -1.5),
        )
    }

    fn load_pig_leg_lb(pigleg_l: PigLegL) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match pigleg_l {
                PigLegL::Default => "npc/pig_purple/pigleg_l.vox",
            },
            Vec3::new(0.0, -1.0, -1.5),
        )
    }

    fn load_pig_leg_rb(pig_leg_r: PigLegR) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match pig_leg_r {
                PigLegR::Default => "npc/pig_purple/pigleg_r.vox",
            },
            Vec3::new(0.0, -1.0, -1.5),
        )
    }
}

pub struct FigureMgr {
    model_cache: FigureModelCache,
    character_states: HashMap<EcsEntity, FigureState<CharacterSkeleton>>,
    quadruped_states: HashMap<EcsEntity, FigureState<QuadrupedSkeleton>>,
}

impl FigureMgr {
    pub fn new() -> Self {
        Self {
            model_cache: FigureModelCache::new(),
            character_states: HashMap::new(),
            quadruped_states: HashMap::new(),
        }
    }

    pub fn clean(&mut self, tick: u64) {
        self.model_cache.clean(tick);
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, client: &Client) {
        let time = client.state().get_time();
        let ecs = client.state().ecs();
        for (entity, pos, vel, dir, actor, animation_history, stats) in (
            &ecs.entities(),
            &ecs.read_storage::<comp::phys::Pos>(),
            &ecs.read_storage::<comp::phys::Vel>(),
            &ecs.read_storage::<comp::phys::Dir>(),
            &ecs.read_storage::<comp::Actor>(),
            &ecs.read_storage::<comp::AnimationHistory>(),
            ecs.read_storage::<comp::Stats>().maybe(),
        )
            .join()
        {
            match actor {
                comp::Actor::Character { body, .. } => match body {
                    Body::Humanoid(body) => {
                        let state = self.character_states.entry(entity).or_insert_with(|| {
                            FigureState::new(renderer, CharacterSkeleton::new())
                        });

                        let target_skeleton = match animation_history.current {
                            comp::Animation::Idle => character::IdleAnimation::update_skeleton(
                                state.skeleton_mut(),
                                time,
                                animation_history.time,
                            ),
                            comp::Animation::Run => character::RunAnimation::update_skeleton(
                                state.skeleton_mut(),
                                (vel.0.magnitude(), time),
                                animation_history.time,
                            ),
                            comp::Animation::Jump => character::JumpAnimation::update_skeleton(
                                state.skeleton_mut(),
                                time,
                                animation_history.time,
                            ),
                            comp::Animation::Gliding => {
                                character::GlidingAnimation::update_skeleton(
                                    state.skeleton_mut(),
                                    time,
                                    animation_history.time,
                                )
                            }
                        };

                        state.skeleton.interpolate(&target_skeleton);

                        state.update(renderer, pos.0, dir.0, Rgba::white());
                    }
                    Body::Quadruped(body) => {
                        let state = self.quadruped_states.entry(entity).or_insert_with(|| {
                            FigureState::new(renderer, QuadrupedSkeleton::new())
                        });

                        let target_skeleton = match animation_history.current {
                            comp::Animation::Run => quadruped::RunAnimation::update_skeleton(
                                state.skeleton_mut(),
                                (vel.0.magnitude(), time),
                                animation_history.time,
                            ),
                            comp::Animation::Idle => quadruped::IdleAnimation::update_skeleton(
                                state.skeleton_mut(),
                                time,
                                animation_history.time,
                            ),
                            comp::Animation::Jump => quadruped::JumpAnimation::update_skeleton(
                                state.skeleton_mut(),
                                (vel.0.magnitude(), time),
                                animation_history.time,
                            ),

                            // TODO!
                            _ => state.skeleton_mut().clone(),
                        };

                        state.skeleton.interpolate(&target_skeleton);

                        // Change in health as color!
                        let col = stats
                            .and_then(|stats| stats.hp.last_change)
                            .map(|(change_by, change_time)| Rgba::new(1.0, 0.7, 0.7, 1.0))
                            .unwrap_or(Rgba::broadcast(1.0));

                        // Change in health as color!
                        let col = stats
                            .and_then(|stats| stats.hp.last_change)
                            .map(|(change_by, change_time)| Rgba::new(1.0, 0.7, 0.7, 1.0))
                            .unwrap_or(Rgba::broadcast(1.0));

                        state.update(renderer, pos.0, dir.0, col);

                        state.update(renderer, pos.0, dir.0, col);
                    }
                },
                // TODO: Non-character actors
            }
        }

        // Clear states that have dead entities.
        self.character_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
        self.quadruped_states
            .retain(|entity, _| ecs.entities().is_alive(*entity));
    }

    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        client: &mut Client,
        globals: &Consts<Globals>,
    ) {
        let tick = client.get_tick();
        let ecs = client.state().ecs();

        for (entity, actor) in (&ecs.entities(), &ecs.read_storage::<comp::Actor>()).join() {
            match actor {
                comp::Actor::Character { body, .. } => {
                    if let Some((locals, bone_consts)) = match body {
                        Body::Humanoid(_) => self
                            .character_states
                            .get(&entity)
                            .map(|state| (state.locals(), state.bone_consts())),
                        Body::Quadruped(_) => self
                            .quadruped_states
                            .get(&entity)
                            .map(|state| (state.locals(), state.bone_consts())),
                    } {
                        let model = self.model_cache.get_or_create_model(renderer, *body, tick);

                        renderer.render_figure(model, globals, locals, bone_consts);
                    }
                }
            }
        }
    }
}

pub struct FigureState<S: Skeleton> {
    bone_consts: Consts<FigureBoneData>,
    locals: Consts<FigureLocals>,
    skeleton: S,
}

impl<S: Skeleton> FigureState<S> {
    pub fn new(renderer: &mut Renderer, skeleton: S) -> Self {
        Self {
            bone_consts: renderer
                .create_consts(&skeleton.compute_matrices())
                .unwrap(),
            locals: renderer.create_consts(&[FigureLocals::default()]).unwrap(),
            skeleton,
        }
    }

    pub fn update(
        &mut self,
        renderer: &mut Renderer,
        pos: Vec3<f32>,
        dir: Vec3<f32>,
        col: Rgba<f32>,
    ) {
        let mat = Mat4::<f32>::identity()
            * Mat4::translation_3d(pos)
            * Mat4::rotation_z(-dir.x.atan2(dir.y)); // + f32::consts::PI / 2.0);

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
