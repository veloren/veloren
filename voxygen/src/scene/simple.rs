use crate::{
    mesh::{greedy::GreedyMesh, segment::generate_mesh_base_vol_terrain},
    render::{
        create_skybox_mesh, BoneMeshes, Consts, FigureModel, FirstPassDrawer, GlobalModel, Globals,
        GlobalsBindGroup, Light, LodData, Mesh, Model, PointLightMatrix, RainOcclusionLocals,
        Renderer, Shadow, ShadowLocals, SkyboxVertex, TerrainVertex,
    },
    scene::{
        camera::{self, Camera, CameraMode},
        figure::{
            load_mesh, FigureColLights, FigureModelCache, FigureModelEntry, FigureState,
            FigureUpdateCommonParameters,
        },
    },
    window::{Event, PressState},
};
use anim::{
    character::{CharacterSkeleton, IdleAnimation, SkeletonAttr},
    fixture::FixtureSkeleton,
    Animation,
};
use client::Client;
use common::{
    comp::{
        humanoid,
        inventory::{slot::EquipSlot, Inventory},
        item::ItemKind,
    },
    figure::Segment,
    slowjob::SlowJobPool,
    terrain::BlockKind,
    vol::{BaseVol, ReadVol},
};
use vek::*;
use winit::event::MouseButton;

struct VoidVol;
impl BaseVol for VoidVol {
    type Error = ();
    type Vox = ();
}
impl ReadVol for VoidVol {
    fn get(&self, _pos: Vec3<i32>) -> Result<&'_ Self::Vox, Self::Error> { Ok(&()) }
}

fn generate_mesh(
    greedy: &mut GreedyMesh<'_>,
    mesh: &mut Mesh<TerrainVertex>,
    segment: Segment,
    offset: Vec3<f32>,
    bone_idx: u8,
) -> BoneMeshes {
    let (opaque, _, /* shadow */ _, bounds) =
        generate_mesh_base_vol_terrain(segment, (greedy, mesh, offset, Vec3::one(), bone_idx));
    (opaque /* , shadow */, bounds)
}

struct Skybox {
    model: Model<SkyboxVertex>,
}

pub struct Scene {
    data: GlobalModel,
    globals_bind_group: GlobalsBindGroup,
    camera: Camera,

    skybox: Skybox,
    lod: LodData,
    map_bounds: Vec2<f32>,

    col_lights: FigureColLights,
    backdrop: Option<(FigureModelEntry<1>, FigureState<FixtureSkeleton>)>,
    figure_model_cache: FigureModelCache,
    figure_state: Option<FigureState<CharacterSkeleton>>,

    //turning_camera: bool,
    turning_character: bool,
    char_ori: f32,
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
    pub fn new(renderer: &mut Renderer, backdrop: Option<&str>, client: &Client) -> Self {
        let start_angle = 90.0f32.to_radians();
        let resolution = renderer.resolution().map(|e| e as f32);

        let map_bounds = Vec2::new(
            client.world_data().min_chunk_alt(),
            client.world_data().max_chunk_alt(),
        );

        let mut camera = Camera::new(resolution.x / resolution.y, CameraMode::ThirdPerson);
        camera.set_focus_pos(Vec3::unit_z() * 1.5);
        camera.set_distance(3.4);
        camera.set_orientation(Vec3::new(start_angle, 0.0, 0.0));

        let mut col_lights = FigureColLights::new(renderer);

        let data = GlobalModel {
            globals: renderer.create_consts(&[Globals::default()]),
            lights: renderer.create_consts(&[Light::default(); 20]),
            shadows: renderer.create_consts(&[Shadow::default(); 24]),
            shadow_mats: renderer.create_shadow_bound_locals(&[ShadowLocals::default()]),
            rain_occlusion_mats: renderer
                .create_rain_occlusion_bound_locals(&[RainOcclusionLocals::default()]),
            point_light_matrices: Box::new([PointLightMatrix::default(); 126]),
        };
        let lod = LodData::dummy(renderer);

        let globals_bind_group = renderer.bind_globals(&data, &lod);

        Self {
            data,
            globals_bind_group,
            skybox: Skybox {
                model: renderer.create_model(&create_skybox_mesh()).unwrap(),
            },
            lod,
            map_bounds,

            figure_model_cache: FigureModelCache::new(),
            figure_state: None,

            backdrop: backdrop.map(|specifier| {
                let mut state = FigureState::new(renderer, FixtureSkeleton::default(), ());
                let mut greedy = FigureModel::make_greedy();
                let mut opaque_mesh = Mesh::new();
                let (segment, offset) = load_mesh(specifier, Vec3::new(-55.0, -49.5, -2.0));
                let (_opaque_mesh, bounds) =
                    generate_mesh(&mut greedy, &mut opaque_mesh, segment, offset, 0);
                // NOTE: Since MagicaVoxel sizes are limited to 256 × 256 × 256, and there are
                // at most 3 meshed vertices per unique vertex, we know the
                // total size is bounded by 2^24 * 3 * 1.5 which is bounded by
                // 2^27, which fits in a u32.
                let range = 0..opaque_mesh.vertices().len() as u32;
                let model =
                    col_lights
                        .create_figure(renderer, greedy.finalize(), (opaque_mesh, bounds), [range]);
                let mut buf = [Default::default(); anim::MAX_BONE_COUNT];
                let common_params = FigureUpdateCommonParameters {
                    entity: None,
                    pos: anim::vek::Vec3::zero(),
                    ori: anim::vek::Quaternion::rotation_from_to_3d(
                        anim::vek::Vec3::unit_y(),
                        anim::vek::Vec3::new(start_angle.sin(), -start_angle.cos(), 0.0),
                    ),
                    scale: 1.0,
                    mount_transform_pos: None,
                    body: None,
                    tools: (None, None),
                    col: Rgba::broadcast(1.0),
                    dt: 15.0, // Want to get there immediately.
                    _lpindex: 0,
                    _visible: true,
                    is_player: false,
                    _camera: &camera,
                    terrain: None,
                    ground_vel: Vec3::zero(),
                };
                state.update(
                    renderer,
                    None,
                    &mut buf,
                    &common_params,
                    1.0,
                    Some(&model),
                    (),
                );
                (model, state)
            }),
            col_lights,

            camera,

            //turning_camera: false,
            turning_character: false,
            char_ori: -start_angle,
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
                    //self.turning_camera = button == MouseButton::Right;
                    self.turning_character = button == MouseButton::Left;
                } else {
                    //self.turning_camera = false;
                    self.turning_character = false;
                }
                true
            },
            Event::CursorMove(delta) => {
                /*if self.turning_camera {
                    self.camera.rotate_by(Vec3::new(delta.x * 0.01, 0.0, 0.0))
                }*/
                if self.turning_character {
                    self.char_ori += delta.x * 0.01;
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
    ) {
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
            ..
        } = self.camera.dependents();
        const VD: f32 = 115.0; // View Distance

        const TIME: f64 = 8.6 * 60.0 * 60.0;
        const SHADOW_NEAR: f32 = 1.0;
        const SHADOW_FAR: f32 = 25.0;

        renderer.update_consts(&mut self.data.globals, &[Globals::new(
            view_mat,
            proj_mat,
            cam_pos,
            self.camera.get_focus_pos(),
            VD,
            self.lod.tgt_detail as f32,
            self.map_bounds,
            TIME,
            scene_data.time,
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

        self.figure_model_cache
            .clean(&mut self.col_lights, scene_data.tick);

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

        if let Some(body) = scene_data.body {
            let figure_state = self.figure_state.get_or_insert_with(|| {
                FigureState::new(renderer, CharacterSkeleton::default(), body)
            });
            let tgt_skeleton = IdleAnimation::update_skeleton(
                figure_state.skeleton_mut(),
                (
                    active_tool_kind,
                    second_tool_kind,
                    hands,
                    scene_data.time as f32,
                ),
                scene_data.time as f32,
                &mut 0.0,
                &SkeletonAttr::from(&body),
            );
            let dt_lerp = (scene_data.delta_time * 15.0).min(1.0);
            *figure_state.skeleton_mut() =
                Lerp::lerp(&*figure_state.skeleton_mut(), &tgt_skeleton, dt_lerp);

            let model = self
                .figure_model_cache
                .get_or_create_model(
                    renderer,
                    &mut self.col_lights,
                    body,
                    inventory,
                    (),
                    scene_data.tick,
                    CameraMode::default(),
                    None,
                    scene_data.slow_job_pool,
                    None,
                )
                .0;
            let mut buf = [Default::default(); anim::MAX_BONE_COUNT];
            let common_params = FigureUpdateCommonParameters {
                entity: None,
                pos: anim::vek::Vec3::zero(),
                ori: anim::vek::Quaternion::rotation_from_to_3d(
                    anim::vek::Vec3::unit_y(),
                    anim::vek::Vec3::new(self.char_ori.sin(), -self.char_ori.cos(), 0.0),
                ),
                scale: 1.0,
                mount_transform_pos: None,
                body: None,
                tools: (None, None),
                col: Rgba::broadcast(1.0),
                dt: scene_data.delta_time,
                _lpindex: 0,
                _visible: true,
                is_player: false,
                _camera: &self.camera,
                terrain: None,
                ground_vel: Vec3::zero(),
            };

            figure_state.update(renderer, None, &mut buf, &common_params, 1.0, model, body);
        }
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
            let model = &self.figure_model_cache.get_model(
                &self.col_lights,
                body,
                inventory,
                tick,
                CameraMode::default(),
                None,
                None,
            );

            if let Some((model, figure_state)) = model.zip(self.figure_state.as_ref()) {
                if let Some(lod) = model.lod_model(0) {
                    figure_drawer.draw(lod, figure_state.bound(), self.col_lights.texture(model));
                }
            }
        }

        if let Some((model, state)) = &self.backdrop {
            if let Some(lod) = model.lod_model(0) {
                figure_drawer.draw(lod, state.bound(), self.col_lights.texture(model));
            }
        }
        drop(figure_drawer);

        drawer.draw_skybox(&self.skybox.model);
    }
}
