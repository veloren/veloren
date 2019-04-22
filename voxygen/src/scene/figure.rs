use std::{
    collections::HashMap,
    f32,
};
use specs::{Entity as EcsEntity, Component, VecStorage, Join};
use vek::*;
use client::Client;
use common::{
    comp,
    figure::Segment,
    msg,
    assets,
};
use crate::{
    Error,
    render::{
        Consts,
        Globals,
        Mesh,
        Model,
        Renderer,
        FigurePipeline,
        FigureBoneData,
        FigureLocals,
    },
    anim::{
        Animation,
        Skeleton,
        character::{
            CharacterSkeleton,
            RunAnimation,
            IdleAnimation,
        },
    },
    mesh::Meshable,
};

pub struct Figures {
    test_model: Model<FigurePipeline>,
    states: HashMap<EcsEntity, FigureState<CharacterSkeleton>>,
}

impl Figures {
    pub fn new(renderer: &mut Renderer) -> Self {
        // TODO: Make a proper asset loading system
        fn load_segment(filename: &'static str) -> Segment {
            let fullpath: String = ["/voxygen/voxel/", filename].concat();
            Segment::from(dot_vox::load_bytes(
                assets::load(fullpath.as_str())
                    .expect("Error loading file")
                    .as_slice(),
            ).unwrap())
        }

        let bone_meshes = [

            Some(load_segment("head.vox").generate_mesh(Vec3::new(-3.5, -7.0, -6.0))),
            Some(load_segment("chest.vox").generate_mesh(Vec3::new(-3.0, -6.0, 0.0))),
            Some(load_segment("belt.vox").generate_mesh(Vec3::new(-3.0, -5.0, 0.0))),
            Some(load_segment("pants.vox").generate_mesh(Vec3::new(-3.0, -5.0, 0.0))),
            Some(load_segment("hand.vox").generate_mesh(Vec3::new(0.0, -2.0, -6.0))),
            Some(load_segment("hand.vox").generate_mesh(Vec3::new(0.0, -2.0, -6.0))),
            Some(load_segment("foot.vox").generate_mesh(Vec3::new(-4.0, -2.5, -6.0))),
            Some(load_segment("foot.vox").generate_mesh(Vec3::new(-4.0, -2.5, -6.0))),
            Some(load_segment("sword.vox").generate_mesh(Vec3::new(0.0, -0.0, 0.0))),
            None,
            None,
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
                mesh.push_mesh_map(bone_mesh, |vert| vert.with_bone_idx(i as u8))
            });

        Self {
            test_model: renderer.create_model(&mesh).unwrap(),
            states: HashMap::new(),
        }
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, client: &mut Client) {
        let time = client.state().get_time();
        let ecs = client.state_mut().ecs_mut();
        for (entity, pos, dir, character, animation_history) in (
            &ecs.entities(),
            &ecs.read_storage::<comp::phys::Pos>(),
            &ecs.read_storage::<comp::phys::Dir>(),
            &ecs.read_storage::<comp::Character>(),
            &ecs.read_storage::<comp::AnimationHistory>(),
        ).join() {
            let state = self.states
                .entry(entity)
                .or_insert_with(|| FigureState::new(renderer, CharacterSkeleton::new()));

            let target_skeleton = match animation_history.current {
                comp::character::Animation::Idle => IdleAnimation::update_skeleton(&mut state.skeleton, time),
                comp::character::Animation::Run => RunAnimation::update_skeleton(&mut state.skeleton, time),
            };

            state.skeleton.interpolate(&target_skeleton);

            state.update(renderer, pos.0, dir.0);
        }

        self.states.retain(|entity, _| ecs.entities().is_alive(*entity));
    }

    pub fn render(&self, renderer: &mut Renderer, client: &Client, globals: &Consts<Globals>) {
        for state in self.states.values() {
            renderer.render_figure(
                &self.test_model,
                globals,
                &state.locals,
                &state.bone_consts,
            );
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
            bone_consts: renderer.create_consts(&skeleton.compute_matrices()).unwrap(),
            locals: renderer.create_consts(&[FigureLocals::default()]).unwrap(),
            skeleton,
        }
    }

    fn update(&mut self, renderer: &mut Renderer, pos: Vec3<f32>, dir: Vec3<f32>) {
        let mat =
            Mat4::<f32>::identity() *
            Mat4::translation_3d(pos) *
            Mat4::rotation_z(dir.y.atan2(dir.x));// + f32//::consts)::PI / 2.0);

        let locals = FigureLocals::new(mat);
        renderer.update_consts(&mut self.locals, &[locals]).unwrap();

        renderer.update_consts(&mut self.bone_consts, &self.skeleton.compute_matrices()).unwrap();
    }
}
