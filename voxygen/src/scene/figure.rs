use std::{
    collections::HashMap,
    f32,
};
use specs::{Entity as EcsEntity, Component, VecStorage, Join};
use vek::*;
use client::Client;
use common::{
    comp::{
        self,
        character::{
            Character,
            Head,
            Chest,
            Belt,
            Pants,
            Hand,
            Foot,
            Weapon,
        }
    },
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

pub struct FigureCache {
    models: HashMap<Character, (Model<FigurePipeline>, u64)>,
    states: HashMap<EcsEntity, FigureState<CharacterSkeleton>>,
}

impl FigureCache {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            states: HashMap::new(),
        }
    }

    pub fn get_or_create_model<'a>(
        models: &'a mut HashMap<Character, (Model<FigurePipeline>, u64)>,
        renderer: &mut Renderer,
        tick: u64,
        character: Character)
    -> &'a (Model<FigurePipeline>, u64) {
        match models.get_mut(&character) {
            Some((model, last_used)) => {
                *last_used = tick;
            }
            None => {
                models.insert(character, ({
                    let bone_meshes = [
                        Some(Self::load_head(character.head)),
                        Some(Self::load_chest(character.chest)),
                        Some(Self::load_belt(character.belt)),
                        Some(Self::load_pants(character.pants)),
                        Some(Self::load_left_hand(character.hand)),
                        Some(Self::load_right_hand(character.hand)),
                        Some(Self::load_left_foot(character.foot)),
                        Some(Self::load_right_foot(character.foot)),
                        Some(Self::load_weapon(character.weapon)),
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

                    renderer.create_model(&mesh).unwrap()
                }, tick));
            }
        }

        &models[&character]
    }

    pub fn clean(&mut self, tick: u64) {
        // TODO: Don't hard-code this
        self.models.retain(|_, (_, last_used)| *last_used + 60 > tick);
    }

    fn load_mesh(filename: &'static str, position: Vec3<f32>) -> Mesh<FigurePipeline> {
        let fullpath: String = ["/voxygen/voxel/", filename].concat();
        Segment::from(dot_vox::load_bytes(
            assets::load(fullpath.as_str())
                .expect("Error loading file")
                .as_slice(),
        ).unwrap())
            .generate_mesh(position)
    }

    fn load_head(head: Head) -> Mesh<FigurePipeline> {
        Self::load_mesh(match head {
            Head::DefaultHead => "head.vox",
        }, Vec3::new(-5.5, -7.0, -6.0))
    }

    fn load_chest(chest: Chest) -> Mesh<FigurePipeline> {
        Self::load_mesh(match chest {
            Chest::DefaultChest => "chest.vox",
        }, Vec3::new(-2.5, -6.0, 0.0))
    }

    fn load_belt(belt: Belt) -> Mesh<FigurePipeline> {
        Self::load_mesh(match belt {
            Belt::DefaultBelt => "belt.vox",
        }, Vec3::new(-2.5, -5.0, 0.0))
    }

    fn load_pants(pants: Pants) -> Mesh<FigurePipeline> {
        Self::load_mesh(match pants {
            Pants::DefaultPants => "pants.vox",
        }, Vec3::new(-2.5, -5.0, 0.0))
    }

    fn load_left_hand(hand: Hand) -> Mesh<FigurePipeline> {
        Self::load_mesh(match hand {
            Hand::DefaultHand => "hand.vox",
        }, Vec3::new(0.0, -2.0, -7.0))
    }

    fn load_right_hand(hand: Hand) -> Mesh<FigurePipeline> {
        Self::load_mesh(match hand {
            Hand::DefaultHand => "hand.vox",
        }, Vec3::new(0.0, -2.0, -7.0))
    }

    fn load_left_foot(foot: Foot) -> Mesh<FigurePipeline> {
        Self::load_mesh(match foot {
            Foot::DefaultFoot => "foot.vox",
        }, Vec3::new(-3.5, -2.5, -8.0))
    }

    fn load_right_foot(foot: Foot) -> Mesh<FigurePipeline> {
        Self::load_mesh(match foot {
            Foot::DefaultFoot => "foot.vox",
        }, Vec3::new(-3.5, -2.5, -8.0))
    }

    fn load_weapon(weapon: Weapon) -> Mesh<FigurePipeline> {
        Self::load_mesh(match weapon {
            Weapon::Sword => "sword.vox",
            // TODO actually match against other weapons and set the right model
            _ => "sword.vox",
        }, Vec3::new(0.0, 0.0, -4.0))
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

    pub fn render(&mut self, renderer: &mut Renderer, client: &mut Client, globals: &Consts<Globals>) {
        let tick = client.get_tick();
        let ecs = client.state().ecs();
        let models = &mut self.models;

        for (entity, &character) in (
            &ecs.entities(),
            &ecs.read_storage::<comp::Character>(),
        ).join() {
            let model = Self::get_or_create_model(models, renderer, tick, character);
            let state = self.states.get(&entity).unwrap();
            renderer.render_figure(
                &model.0,
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
            Mat4::rotation_z(-dir.x.atan2(dir.y) + f32::consts::PI / 2.0);

        let locals = FigureLocals::new(mat);
        renderer.update_consts(&mut self.locals, &[locals]).unwrap();

        renderer.update_consts(&mut self.bone_consts, &self.skeleton.compute_matrices()).unwrap();
    }
}
