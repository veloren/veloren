use crate::{
    render::{
        create_skybox_mesh, pipelines::terrain::BoundLocals as BoundTerrainLocals, AltIndices,
        Consts, FirstPassDrawer, GlobalModel, Globals, GlobalsBindGroup, Light, Model,
        PointLightMatrix, RainOcclusionLocals, Renderer, Shadow, ShadowLocals, SkyboxVertex,
        SpriteGlobalsBindGroup,
    },
    scene::{
        camera::{self, Camera, CameraMode},
        figure::{FigureAtlas, FigureModelCache, FigureState, FigureUpdateCommonParameters},
        terrain::{SpriteRenderContext, SpriteRenderState},
        CloudsLocals, CullingMode, Lod, PostProcessLocals,
    },
    window::{Event, PressState},
    Settings,
};
use anim::{character::CharacterSkeleton, ship::ShipSkeleton, Animation};
use client::Client;
use common::{
    comp::{
        humanoid,
        inventory::{slot::EquipSlot, Inventory},
        item::ItemKind,
        ship,
    },
    slowjob::SlowJobPool,
    terrain::{BlockKind, CoordinateConversions},
    vol::{BaseVol, ReadVol},
};
use vek::*;
use winit::event::MouseButton;

use super::figure::{ModelEntry, ModelEntryRef};

struct VoidVol;
impl BaseVol for VoidVol {
    type Error = ();
    type Vox = ();
}
impl ReadVol for VoidVol {
    fn get(&self, _pos: Vec3<i32>) -> Result<&'_ Self::Vox, Self::Error> { Ok(&()) }
}

struct Skybox {
    model: Model<SkyboxVertex>,
}

pub struct Scene {
    data: GlobalModel,
    globals_bind_group: GlobalsBindGroup,
    camera: Camera,

    skybox: Skybox,
    lod: Lod,
    map_bounds: Vec2<f32>,

    figure_atlas: FigureAtlas,
    sprite_render_state: SpriteRenderState,
    sprite_globals: SpriteGlobalsBindGroup,

    turning_camera: bool,

    char_pos: Vec3<f32>,
    char_state: Option<FigureState<CharacterSkeleton>>,
    char_model_cache: FigureModelCache<CharacterSkeleton>,

    airship_pos: Vec3<f32>,
    airship_state: Option<FigureState<ShipSkeleton, BoundTerrainLocals>>,
    airship_model_cache: FigureModelCache<ShipSkeleton>,
}

pub struct SceneData<'a> {
    pub time: f64,
    pub delta_time: f32,
    pub tick: u64,
    pub slow_job_pool: &'a SlowJobPool,
    pub body: Option<humanoid::Body>,
    pub gamma: f32,
    pub exposure: f32,
    pub ambiance: f32,
    pub figure_lod_render_distance: f32,
    pub mouse_smoothing: bool,
}

impl Scene {
    pub fn new(
        renderer: &mut Renderer,
        client: &mut Client,
        settings: &Settings,
        sprite_render_context: SpriteRenderContext,
    ) -> Self {
        let start_angle = -90.0f32.to_radians();
        let resolution = renderer.resolution().map(|e| e as f32);

        let map_bounds = Vec2::new(
            client.world_data().min_chunk_alt(),
            client.world_data().max_chunk_alt(),
        );

        let mut camera = Camera::new(resolution.x / resolution.y, CameraMode::ThirdPerson);
        camera.set_distance(3.4);
        camera.set_orientation(Vec3::new(start_angle, 0.1, 0.0));

        let figure_atlas = FigureAtlas::new(renderer);

        let data = GlobalModel {
            globals: renderer.create_consts(&[Globals::default()]),
            lights: renderer.create_consts(&[Light::default(); crate::scene::MAX_LIGHT_COUNT]),
            shadows: renderer.create_consts(&[Shadow::default(); crate::scene::MAX_SHADOW_COUNT]),
            shadow_mats: renderer.create_shadow_bound_locals(&[ShadowLocals::default()]),
            rain_occlusion_mats: renderer
                .create_rain_occlusion_bound_locals(&[RainOcclusionLocals::default()]),
            point_light_matrices: Box::new(
                [PointLightMatrix::default(); crate::scene::MAX_POINT_LIGHT_MATRICES_COUNT],
            ),
        };
        let lod = Lod::new(renderer, client, settings);

        let globals_bind_group = renderer.bind_globals(&data, lod.get_data());

        let world = client.world_data();
        let char_chunk = world.chunk_size().map(|e| e as i32 / 2);
        let char_pos = char_chunk.cpos_to_wpos().map(|e| e as f32).with_z(
            world
                .lod_alt
                .get(char_chunk)
                .map_or(0.0, |z| *z as f32 + 48.0),
        );
        client.set_lod_pos_fallback(char_pos.xy());
        client.set_lod_distance(settings.graphics.lod_distance);

        Self {
            globals_bind_group,
            skybox: Skybox {
                model: renderer.create_model(&create_skybox_mesh()).unwrap(),
            },
            map_bounds,

            figure_atlas,
            sprite_render_state: sprite_render_context.state,
            sprite_globals: renderer.bind_sprite_globals(
                &data,
                lod.get_data(),
                &sprite_render_context.sprite_verts_buffer,
            ),
            lod,
            data,

            camera,

            turning_camera: false,
            char_pos,
            char_state: None,
            char_model_cache: FigureModelCache::new(),

            airship_pos: char_pos - Vec3::unit_z() * 10.0,
            airship_state: None,
            airship_model_cache: FigureModelCache::new(),
        }
    }

    pub fn globals(&self) -> &Consts<Globals> { &self.data.globals }

    pub fn camera_mut(&mut self) -> &mut Camera { &mut self.camera }

    /// Handle an incoming user input event (e.g.: cursor moved, key pressed,
    /// window closed).
    ///
    /// If the event is handled, return true.
    pub fn handle_input_event(&mut self, event: Event) -> bool {
        match event {
            // When the window is resized, change the camera's aspect ratio
            Event::Resize(dims) => {
                self.camera.set_aspect_ratio(dims.x as f32 / dims.y as f32);
                true
            },
            Event::MouseButton(button, state) => {
                if state == PressState::Pressed {
                    self.turning_camera = button == MouseButton::Left;
                } else {
                    self.turning_camera = false;
                }
                true
            },
            Event::CursorMove(delta) => {
                if self.turning_camera {
                    self.camera.rotate_by(delta.with_z(0.0) * 0.01);
                }
                true
            },
            // All other events are unhandled
            _ => false,
        }
    }

    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        scene_data: SceneData,
        inventory: Option<&Inventory>,
        client: &Client,
    ) {
        self.camera
            .force_focus_pos(self.char_pos + Vec3::unit_z() * 1.5);
        let ori = self.camera.get_tgt_orientation();
        self.camera
            .set_orientation(Vec3::new(ori.x, ori.y.max(-0.25), ori.z));
        self.camera.update(
            scene_data.time,
            /* 1.0 / 60.0 */ scene_data.delta_time,
            scene_data.mouse_smoothing,
        );

        self.camera.compute_dependents_full(&VoidVol, |_| false);
        let camera::Dependents {
            view_mat,
            proj_mat,
            cam_pos,
            proj_mat_inv,
            view_mat_inv,
            ..
        } = self.camera.dependents();
        const VD: f32 = 0.0; // View Distance

        const TIME: f64 = 8.6 * 60.0 * 60.0;
        const SHADOW_NEAR: f32 = 0.25;
        const SHADOW_FAR: f32 = 1.0;

        self.lod
            .maintain(renderer, client, self.camera.get_focus_pos(), &self.camera);

        renderer.update_consts(&mut self.data.globals, &[Globals::new(
            view_mat,
            proj_mat,
            cam_pos,
            self.camera.get_focus_pos(),
            VD,
            self.lod.get_data().tgt_detail as f32,
            self.map_bounds,
            TIME,
            scene_data.time,
            0.0,
            renderer.resolution().as_(),
            Vec2::new(SHADOW_NEAR, SHADOW_FAR),
            0,
            0,
            0,
            BlockKind::Air,
            None,
            scene_data.gamma,
            scene_data.exposure,
            (Vec3::zero(), -1000.0),
            Vec2::zero(),
            scene_data.ambiance,
            self.camera.get_mode(),
            250.0,
        )]);
        renderer.update_clouds_locals(CloudsLocals::new(proj_mat_inv, view_mat_inv));
        renderer.update_postprocess_locals(PostProcessLocals::new(proj_mat_inv, view_mat_inv));

        self.char_model_cache
            .clean(&mut self.figure_atlas, scene_data.tick);
        self.airship_model_cache
            .clean(&mut self.figure_atlas, scene_data.tick);

        let item_info = |equip_slot| {
            inventory
                .and_then(|inv| inv.equipped(equip_slot))
                .and_then(|i| {
                    if let ItemKind::Tool(tool) = &*i.kind() {
                        Some((Some(tool.kind), Some(tool.hands)))
                    } else {
                        None
                    }
                })
                .unwrap_or((None, None))
        };

        let (active_tool_kind, active_tool_hand) = item_info(EquipSlot::ActiveMainhand);
        let (second_tool_kind, second_tool_hand) = item_info(EquipSlot::ActiveOffhand);

        let hands = (active_tool_hand, second_tool_hand);

        fn figure_params(
            camera: &Camera,
            dt: f32,
            pos: Vec3<f32>,
        ) -> FigureUpdateCommonParameters<'_> {
            FigureUpdateCommonParameters {
                entity: None,
                pos: pos.into(),
                ori: anim::vek::Quaternion::identity().rotated_z(std::f32::consts::PI * -0.5),
                scale: 1.0,
                mount_transform_pos: None,
                body: None,
                tools: (None, None),
                col: Rgba::broadcast(1.0),
                dt,
                _lpindex: 0,
                _visible: true,
                is_player: false,
                _camera: camera,
                terrain: None,
                ground_vel: Vec3::zero(),
            }
        }

        if let Some(body) = scene_data.body {
            let char_state = self.char_state.get_or_insert_with(|| {
                FigureState::new(renderer, CharacterSkeleton::default(), body)
            });
            let params = figure_params(&self.camera, scene_data.delta_time, self.char_pos);
            let tgt_skeleton = anim::character::IdleAnimation::update_skeleton(
                char_state.skeleton_mut(),
                (
                    active_tool_kind,
                    second_tool_kind,
                    hands,
                    scene_data.time as f32,
                ),
                scene_data.time as f32,
                &mut 0.0,
                &anim::character::SkeletonAttr::from(&body),
            );
            let dt_lerp = (scene_data.delta_time * 15.0).min(1.0);
            *char_state.skeleton_mut() =
                Lerp::lerp(&*char_state.skeleton_mut(), &tgt_skeleton, dt_lerp);
            let (model, _) = self.char_model_cache.get_or_create_model(
                renderer,
                &mut self.figure_atlas,
                body,
                inventory,
                (),
                scene_data.tick,
                CameraMode::default(),
                None,
                scene_data.slow_job_pool,
                None,
            );
            char_state.update(
                renderer,
                None,
                &mut [Default::default(); anim::MAX_BONE_COUNT],
                &params,
                1.0,
                model,
                body,
            );
        }

        let airship_body = ship::Body::DefaultAirship;
        let airship_state = self.airship_state.get_or_insert_with(|| {
            FigureState::new(renderer, ShipSkeleton::default(), airship_body)
        });
        let params = figure_params(&self.camera, scene_data.delta_time, self.airship_pos);
        let tgt_skeleton = anim::ship::IdleAnimation::update_skeleton(
            airship_state.skeleton_mut(),
            (
                None,
                None,
                scene_data.time as f32,
                scene_data.time as f32,
                (params.ori * Vec3::unit_y()).into(),
                (params.ori * Vec3::unit_y()).into(),
            ),
            scene_data.time as f32,
            &mut 0.0,
            &anim::ship::SkeletonAttr::from(&airship_body),
        );
        let dt_lerp = (scene_data.delta_time * 15.0).min(1.0);
        *airship_state.skeleton_mut() =
            Lerp::lerp(&*airship_state.skeleton_mut(), &tgt_skeleton, dt_lerp);
        let (model, _) = self.airship_model_cache.get_or_create_terrain_model(
            renderer,
            &mut self.figure_atlas,
            airship_body,
            (),
            scene_data.tick,
            scene_data.slow_job_pool,
            &self.sprite_render_state,
        );
        airship_state.update(
            renderer,
            None,
            &mut [Default::default(); anim::MAX_BONE_COUNT],
            &params,
            1.0,
            model,
            airship_body,
        );
    }

    pub fn global_bind_group(&self) -> &GlobalsBindGroup { &self.globals_bind_group }

    pub fn render<'a>(
        &'a self,
        drawer: &mut FirstPassDrawer<'a>,
        tick: u64,
        body: Option<humanoid::Body>,
        inventory: Option<&Inventory>,
    ) {
        let mut figure_drawer = drawer.draw_figures();
        if let Some(body) = body {
            let model = &self.char_model_cache.get_model(
                &self.figure_atlas,
                body,
                inventory,
                tick,
                CameraMode::default(),
                None,
                None,
            );

            if let Some((model, char_state)) = model.zip(self.char_state.as_ref()) {
                if let Some(lod) = model.lod_model(0) {
                    figure_drawer.draw(
                        lod,
                        char_state.bound(),
                        self.figure_atlas.texture(ModelEntryRef::Figure(model)),
                    );
                }
            }
        }

        let model = &self.airship_model_cache.get_model(
            &self.figure_atlas,
            ship::Body::DefaultAirship,
            Default::default(),
            tick,
            CameraMode::default(),
            None,
            None,
        );
        if let Some((model, airship_state)) = model.zip(self.airship_state.as_ref()) {
            if let Some(lod) = model.lod_model(0) {
                figure_drawer.draw(
                    lod,
                    airship_state.bound(),
                    self.figure_atlas.texture(ModelEntryRef::Terrain(model)),
                );
            }
        }

        drop(figure_drawer);

        let mut sprite_drawer = drawer.draw_sprites(
            &self.sprite_globals,
            &self.sprite_render_state.sprite_atlas_textures,
        );
        if let (Some(sprite_instances), Some(data)) = (
            self.airship_model_cache
                .get_sprites(ship::Body::DefaultAirship),
            self.airship_state.as_ref().map(|s| &s.extra),
        ) {
            sprite_drawer.draw(
                data,
                &sprite_instances[0],
                &AltIndices {
                    deep_end: 0,
                    underground_end: 0,
                },
                CullingMode::None,
            );
        }
        drop(sprite_drawer);

        self.lod.render(drawer, Default::default());

        drawer.draw_skybox(&self.skybox.model);
    }
}
