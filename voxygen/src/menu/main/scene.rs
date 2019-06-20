use crate::{
    anim::{
        character::{CharacterSkeleton, IdleAnimation},
        fixture::FixtureSkeleton,
        Animation, Skeleton,
    },
    render::{
        create_pp_mesh, create_skybox_mesh, Consts, FigurePipeline, Globals, Model,
        PostProcessLocals, PostProcessPipeline, Renderer, SkyboxLocals, SkyboxPipeline,
    },
    scene::{
        camera::Camera,
        figure::{FigureModelCache, FigureState},
    },
};
use client::Client;
use common::comp;
use log::error;
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
    camera: Camera,

    skybox: Skybox,
    postprocess: PostProcess,
    // TODO: Uncomment this once the model will be ready
    // backdrop_model: Model<FigurePipeline>,
    backdrop_state: FigureState<FixtureSkeleton>,
}

impl Scene {
    pub fn new(renderer: &mut Renderer) -> Self {
        let resolution = renderer.get_resolution().map(|e| e as f32);

        Self {
            globals: renderer.create_consts(&[Globals::default()]).unwrap(),
            camera: Camera::new(resolution.x / resolution.y),

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

            // TODO: Ditto, but also add the model in load_mesh()
            /* backdrop_model: renderer
            .create_model(&FigureModelCache::load_mesh(

            ))
            .unwrap(), */
            backdrop_state: FigureState::new(renderer, FixtureSkeleton::new()),
        }
    }

    pub fn globals(&self) -> &Consts<Globals> {
        &self.globals
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, client: &Client) {
        self.camera.set_focus_pos(Vec3::unit_z() * 2.0);
        self.camera.update(client.state().get_time());
        self.camera.set_distance(4.2);
        self.camera
            .set_orientation(Vec3::new(client.state().get_time() as f32 * 0.0, 0.0, 0.0));

        let (view_mat, proj_mat, cam_pos) = self.camera.compute_dependents(client);

        if let Err(err) = renderer.update_consts(
            &mut self.globals,
            // TODO: replace client.state().get_time_of_day() with a const so the time will always
            // be day
            &[Globals::new(
                view_mat,
                proj_mat,
                cam_pos,
                self.camera.get_focus_pos(),
                100.0,
                client.state().get_time_of_day(),
                client.state().get_time(),
                renderer.get_resolution(),
            )],
        ) {
            error!("Renderer failed to update: {:?}", err);
        }
    }

    pub fn render(&mut self, renderer: &mut Renderer, client: &Client) {
        renderer.render_skybox(&self.skybox.model, &self.globals, &self.skybox.locals);

        // TODO: Uncomment this once backdrop model will be ready
        /* renderer.render_figure(
            &self.backdrop_model,
            &self.globals,
            self.backdrop_state.locals(),
            self.backdrop_state.bone_consts(),
        ); */

        renderer.render_post_process(
            &self.postprocess.model,
            &self.globals,
            &self.postprocess.locals,
        );
    }
}
