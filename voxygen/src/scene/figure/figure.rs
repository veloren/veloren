use crate::{
    anim::{
        character::{CharacterSkeleton, IdleAnimation, JumpAnimation, RunAnimation},
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
        actor::{Belt, Chest, Foot, Hand, Head, Pants, Shoulder, Weapon},
        Body, HumanoidBody,
    },
    figure::Segment,
    msg,
};
use dot_vox::DotVoxData;
use specs::{Component, Entity as EcsEntity, Join, VecStorage};
use std::{collections::HashMap, f32};
use vek::*;

pub struct FigureModelCache {
    models: HashMap<HumanoidBody, (Model<FigurePipeline>, u64)>,
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
        body: HumanoidBody,
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
                            let bone_meshes = [
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
                                None,
                                None,
                                None,
                                None,
                                None,
                            ];

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
                Chest::Default => "chest.vox",
            },
            Vec3::new(-6.0, -3.5, 0.0),
        )
    }

    fn load_belt(belt: Belt) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match belt {
                Belt::Default => "belt.vox",
            },
            Vec3::new(-5.0, -3.5, 0.0),
        )
    }

    fn load_pants(pants: Pants) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match pants {
                Pants::Default => "pants.vox",
            },
            Vec3::new(-5.0, -3.5, 0.0),
        )
    }

    fn load_left_hand(hand: Hand) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match hand {
                Hand::Default => "hand.vox",
            },
            Vec3::new(2.0, 0.0, -7.0),
        )
    }

    fn load_right_hand(hand: Hand) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match hand {
                Hand::Default => "hand.vox",
            },
            Vec3::new(2.0, 0.0, -7.0),
        )
    }

    fn load_left_foot(foot: Foot) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match foot {
                Foot::Default => "foot.vox",
            },
            Vec3::new(2.5, -3.5, -9.0),
        )
    }

    fn load_right_foot(foot: Foot) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match foot {
                Foot::Default => "foot.vox",
            },
            Vec3::new(2.5, -3.5, -9.0),
        )
    }

    fn load_weapon(weapon: Weapon) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match weapon {
                Weapon::Sword => "sword.vox",
                // TODO actually match against other weapons and set the right model.
                _ => "sword.vox",
            },
            Vec3::new(0.0, 0.0, -4.0),
        )
    }

    fn load_left_shoulder(shoulder: Shoulder) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match shoulder {
                Shoulder::Default => "shoulder_l.vox",
            },
            Vec3::new(2.5, 0.0, 0.0),
        )
    }

    fn load_right_shoulder(shoulder: Shoulder) -> Mesh<FigurePipeline> {
        Self::load_mesh(
            match shoulder {
                Shoulder::Default => "shoulder_r.vox",
            },
            Vec3::new(2.5, 0.0, 0.0),
        )
    }
    //    fn load_draw(draw: Draw) -> Mesh<FigurePipeline> {
    //        Self::load_mesh(
    //            match draw {
    //                //Draw::DefaultDraw => "sword.vox",
    //
    //            },
    //            Vec3::new(0.0, 0.0, -2.0)
    //
    //
    //        )
    //    }
}

pub struct FigureMgr {
    model_cache: FigureModelCache,
    states: HashMap<EcsEntity, FigureState<CharacterSkeleton>>,
}

impl FigureMgr {
    pub fn new() -> Self {
        Self {
            model_cache: FigureModelCache::new(),
            states: HashMap::new(),
        }
    }

    pub fn clean(&mut self, tick: u64) {
        self.model_cache.clean(tick);
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, client: &Client) {
        let time = client.state().get_time();
        let ecs = client.state().ecs();
        for (entity, pos, vel, dir, actor, animation_history) in (
            &ecs.entities(),
            &ecs.read_storage::<comp::phys::Pos>(),
            &ecs.read_storage::<comp::phys::Vel>(),
            &ecs.read_storage::<comp::phys::Dir>(),
            &ecs.read_storage::<comp::Actor>(),
            &ecs.read_storage::<comp::AnimationHistory>(),
        )
            .join()
        {
            match actor {
                comp::Actor::Character { body, .. } => match body {
                    Body::Humanoid(body) => {
                        let state = self.states.entry(entity).or_insert_with(|| {
                            FigureState::new(renderer, CharacterSkeleton::new())
                        });

                        let target_skeleton = match animation_history.current {
                            comp::Animation::Idle => IdleAnimation::update_skeleton(
                                state.skeleton_mut(),
                                time,
                                animation_history.time,
                            ),
                            comp::Animation::Run => RunAnimation::update_skeleton(
                                state.skeleton_mut(),
                                (vel.0.magnitude(), time),
                                animation_history.time,
                            ),
                            comp::Animation::Jump => JumpAnimation::update_skeleton(
                                state.skeleton_mut(),
                                time,
                                animation_history.time,
                            ),
                        };

                        state.skeleton.interpolate(&target_skeleton);

                        state.update(renderer, pos.0, dir.0);
                    } // TODO: Non-humanoid bodies.
                },
                // TODO: Non-character actors.
            }
        }

        self.states
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
                comp::Actor::Character { body, .. } => match body {
                    Body::Humanoid(body) => {
                        if let Some(state) = self.states.get(&entity) {
                            let model = self.model_cache.get_or_create_model(renderer, *body, tick);
                            renderer.render_figure(
                                model,
                                globals,
                                &state.locals(),
                                state.bone_consts(),
                            );
                        }
                    } // TODO: Non-humanoid bodies.
                },
                // TODO: Non-character actors.
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

    pub fn update(&mut self, renderer: &mut Renderer, pos: Vec3<f32>, dir: Vec3<f32>) {
        let mat = Mat4::<f32>::identity()
            * Mat4::translation_3d(pos)
            * Mat4::rotation_z(-dir.x.atan2(dir.y)); // + f32::consts::PI / 2.0);

        let locals = FigureLocals::new(mat);
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
