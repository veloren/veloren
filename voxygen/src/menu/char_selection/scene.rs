use crate::{
    anim::{
        character::{CharacterSkeleton, IdleAnimation},
        fixture::FixtureSkeleton,
        Animation, Skeleton, SkeletonAttr,
    },
    render::{
        create_pp_mesh, create_skybox_mesh, Consts, FigurePipeline, Globals, Light, Model,
        PostProcessLocals, PostProcessPipeline, Renderer, Shadow, SkyboxLocals, SkyboxPipeline,
    },
    scene::{
        camera::{Camera, CameraMode},
        figure::{load_mesh, FigureModelCache, FigureState},
    },
    window::{Event, PressState},
};
use client::Client;
use common::{
    comp::{humanoid, Body, Equipment},
    state::DeltaTime,
    terrain::BlockKind,
};
use log::error;
use specs::WorldExt;
use vek::*;

struct Skybox {
    model: Model<SkyboxPipeline>,
    locals: Consts<SkyboxLocals>,
}

struct PostProcess {
    model: Model<PostProcessPipeline>,
    locals: Consts<PostProcessLocals>,
}

pub struct Scene {
    globals: Consts<Globals>,
    lights: Consts<Light>,
    shadows: Consts<Shadow>,
    camera: Camera,

    skybox: Skybox,
    postprocess: PostProcess,
    backdrop_model: Model<FigurePipeline>,
    backdrop_state: FigureState<FixtureSkeleton>,

    figure_model_cache: FigureModelCache,
    figure_state: FigureState<CharacterSkeleton>,

    turning: bool,
    char_ori: f32,
}

impl Scene {
    pub fn new(renderer: &mut Renderer) -> Self {
        let resolution = renderer.get_resolution().map(|e| e as f32);

        Self {
            globals: renderer.create_consts(&[Globals::default()]).unwrap(),
            lights: renderer.create_consts(&[Light::default(); 32]).unwrap(),
            shadows: renderer.create_consts(&[Shadow::default(); 32]).unwrap(),
            camera: Camera::new(resolution.x / resolution.y, CameraMode::ThirdPerson),

            skybox: Skybox {
                model: renderer.create_model(&create_skybox_mesh()).unwrap(),
                locals: renderer.create_consts(&[SkyboxLocals::default()]).unwrap(),
            },
            postprocess: PostProcess {
                model: renderer.create_model(&create_pp_mesh()).unwrap(),
                locals: renderer
                    .create_consts(&[PostProcessLocals::default()])
                    .unwrap(),
            },
            figure_model_cache: FigureModelCache::new(),
            figure_state: FigureState::new(renderer, CharacterSkeleton::new()),

            backdrop_model: renderer
                .create_model(&load_mesh(
                    "fixture.selection_bg",
                    Vec3::new(-55.0, -49.5, -2.0),
                ))
                .unwrap(),
            backdrop_state: FigureState::new(renderer, FixtureSkeleton::new()),

            turning: false,
            char_ori: 0.0,
        }
    }

    pub fn globals(&self) -> &Consts<Globals> {
        &self.globals
    }

    /// Handle an incoming user input event (e.g.: cursor moved, key pressed, window closed).
    ///
    /// If the event is handled, return true.
    pub fn handle_input_event(&mut self, event: Event) -> bool {
        match event {
            // When the window is resized, change the camera's aspect ratio
            Event::Resize(dims) => {
                self.camera.set_aspect_ratio(dims.x as f32 / dims.y as f32);
                true
            }
            Event::MouseButton(_, state) => {
                self.turning = state == PressState::Pressed;
                true
            }
            Event::CursorMove(delta) if self.turning => {
                self.char_ori += delta.x * 0.01;
                true
            }
            // All other events are unhandled
            _ => false,
        }
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, client: &Client, body: humanoid::Body) {
        self.camera.set_focus_pos(Vec3::unit_z() * 1.5);
        self.camera.update(client.state().get_time());
        self.camera.set_distance(3.0); // 4.2
        self.camera
            .set_orientation(Vec3::new(client.state().get_time() as f32 * 0.0, 0.0, 0.0));

        let (view_mat, proj_mat, cam_pos) = self.camera.compute_dependents(client);
        const VD: f32 = 115.0; //View Distance
        const TIME: f64 = 43200.0; // hours*3600 seconds
        if let Err(err) = renderer.update_consts(
            &mut self.globals,
            &[Globals::new(
                view_mat,
                proj_mat,
                cam_pos,
                self.camera.get_focus_pos(),
                VD,
                TIME,
                client.state().get_time(),
                renderer.get_resolution(),
                0,
                0,
                BlockKind::Air,
                None,
            )],
        ) {
            error!("Renderer failed to update: {:?}", err);
        }

        self.figure_model_cache.clean(client.get_tick());

        let tgt_skeleton = IdleAnimation::update_skeleton(
            self.figure_state.skeleton_mut(),
            client.state().get_time(),
            client.state().get_time(),
            &mut 0.0,
            &SkeletonAttr::from(&body),
        );
        self.figure_state.skeleton_mut().interpolate(
            &tgt_skeleton,
            client.state().ecs().read_resource::<DeltaTime>().0,
        );

        self.figure_state.update(
            renderer,
            Vec3::zero(),
            Vec3::zero(),
            Vec3::new(self.char_ori.sin(), -self.char_ori.cos(), 0.0),
            1.0,
            Rgba::broadcast(1.0),
            1.0 / 60.0, // TODO: Use actual deltatime here?
            1.0,
            1.0,
            0,
            true,
        );
    }

    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        client: &Client,
        body: humanoid::Body,
        equipment: &Equipment,
    ) {
        renderer.render_skybox(&self.skybox.model, &self.globals, &self.skybox.locals);

        let model = &self
            .figure_model_cache
            .get_or_create_model(
                renderer,
                Body::Humanoid(body),
                Some(equipment),
                client.get_tick(),
                CameraMode::default(),
                None,
            )
            .0;

        renderer.render_figure(
            model,
            &self.globals,
            self.figure_state.locals(),
            self.figure_state.bone_consts(),
            &self.lights,
            &self.shadows,
        );

        renderer.render_figure(
            &self.backdrop_model,
            &self.globals,
            self.backdrop_state.locals(),
            self.backdrop_state.bone_consts(),
            &self.lights,
            &self.shadows,
        );

        renderer.render_post_process(
            &self.postprocess.model,
            &self.globals,
            &self.postprocess.locals,
        );
    }
}
