pub mod camera;
pub mod figure;
pub mod lod;
pub mod math;
pub mod simple;
pub mod terrain;

use self::{
    camera::{Camera, CameraMode},
    figure::FigureMgr,
    lod::{Lod, LodData},
    terrain::Terrain,
};
use crate::{
    audio::{music::MusicMgr, sfx::SfxMgr, AudioFrontend},
    render::{
        self, create_pp_mesh, create_skybox_mesh, Consts, Globals, Light, Model, PostProcessLocals,
        PostProcessPipeline, Renderer, Shadow, ShadowLocals, SkyboxLocals, SkyboxPipeline,
    },
    settings::Settings,
    window::{AnalogGameInput, Event},
};
use anim::character::SkeletonAttr;
use client::Client;
use common::{
    comp,
    state::State,
    terrain::{BlockKind, TerrainChunk},
    vol::ReadVol,
};
use num::traits::{Float, FloatConst};
use specs::{Entity as EcsEntity, Join, WorldExt};
use vek::*;

// TODO: Don't hard-code this.
const CURSOR_PAN_SCALE: f32 = 0.005;

const MAX_LIGHT_COUNT: usize = 31;
const MAX_SHADOW_COUNT: usize = 24;
const NUM_DIRECTED_LIGHTS: usize = 1;
const LIGHT_DIST_RADIUS: f32 = 64.0; // The distance beyond which lights may not emit light from their origin
const SHADOW_DIST_RADIUS: f32 = 8.0;
const SHADOW_MAX_DIST: f32 = 96.0; // The distance beyond which shadows may not be visible
/* /// The minimum sin γ we will use before switching to uniform mapping.
const EPSILON_GAMMA: f64 = 0.25; */

// const NEAR_PLANE: f32 = 0.5;
// const FAR_PLANE: f32 = 100000.0;

const SHADOW_NEAR: f32 = 0.25; //1.0; //0.5;//1.0; // Near plane for shadow map rendering.
const SHADOW_FAR: f32 = 128.0; //100000.0;//128.0; //25.0; //100000.0;//25.0; // Far plane for shadow map rendering.

/// Above this speed is considered running
/// Used for first person camera effects
const RUNNING_THRESHOLD: f32 = 0.7;

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
    shadow_mats: Consts<ShadowLocals>,
    shadows: Consts<Shadow>,
    camera: Camera,
    camera_input_state: Vec2<f32>,

    skybox: Skybox,
    postprocess: PostProcess,
    terrain: Terrain<TerrainChunk>,
    pub lod: Lod,
    loaded_distance: f32,
    /// x coordinate is sea level (minimum height for any land chunk), and y
    /// coordinate is the maximum height above the mnimimum for any land
    /// chunk.
    map_bounds: Vec2<f32>,
    select_pos: Option<Vec3<i32>>,
    light_data: Vec<Light>,

    figure_mgr: FigureMgr,
    sfx_mgr: SfxMgr,
    music_mgr: MusicMgr,
}

pub struct SceneData<'a> {
    pub state: &'a State,
    pub player_entity: specs::Entity,
    pub loaded_distance: f32,
    pub view_distance: u32,
    pub tick: u64,
    pub thread_pool: &'a uvth::ThreadPool,
    pub gamma: f32,
    pub mouse_smoothing: bool,
    pub sprite_render_distance: f32,
    pub figure_lod_render_distance: f32,
    pub is_aiming: bool,
}

impl<'a> SceneData<'a> {
    pub fn get_sun_dir(&self) -> Vec3<f32> { Globals::get_sun_dir(self.state.get_time_of_day()) }

    pub fn get_moon_dir(&self) -> Vec3<f32> { Globals::get_moon_dir(self.state.get_time_of_day()) }
}

/// Approximte a scalar field of view angle using the parameterization from
/// section 4.3 of Lloyd's thesis:
///
/// W_e = 2 n_e tan θ
///
/// where
///
/// W_e = 2 is the width of the image plane (for our projections, since they go
/// from -1 to 1) n_e = near_plane is the near plane for the view frustum
/// θ = (fov / 2) is the half-angle of the FOV (the one passed to
/// Mat4::projection_rh_no).
///
/// Although the widths for the x and y image planes are the same, they are
/// different in this framework due to the introduction of an aspect ratio:
///
/// y'(p) = 1.0 / tan(fov / 2) * p.y / -p.z
/// x'(p) = 1.0 / (aspect * tan(fov / 2)) * p.x / -p.z
///
/// i.e.
///
/// y'(x, y, -near, w) = 1 / tan(fov / 2) p.y / near
/// x'(x, y, -near, w) = 1 / (aspect * tan(fov / 2)) p.x / near
///
/// W_e,y = 2 * near_plane * tan(fov / 2)
/// W_e,x = 2 * near_plane * aspect * W_e,y
///
/// Θ_x = atan(W_e_y / 2 / near_plane) = atanfov / t()
///
/// i.e. we have an "effective" W_e_x of
///
/// 2 = 2 * near_plane * tan Θ
///
/// atan(1 / near_plane) = θ
///
/// y'
/// x(-near)
/// W_e = 2 * near_plane *
///
/// W_e_y / n_e = tan (fov / 2)
/// W_e_x = 2 n
fn compute_scalar_fov<F: Float>(_near_plane: F, fov: F, aspect: F) -> F {
    let two = F::one() + F::one();
    let theta_y = fov / two;
    let theta_x = (aspect * theta_y.tan()).atan();
    /* let h = (fov / two).tan().recip();
    let w = h / aspect;
    let theta_y = (h / two).atan();
    let theta_x = (w / two).atan(); */
    /* // let theta_x = ((aspect * (fov / two).tan()).recip()/* / (two * near_plane)*/).atan();
    // let theta_y  = ((fov / two).tan().recip()/* / (two * near_plane)*/).atan();
    let theta_x = ((aspect * (fov / two).tan()) / ).atan();
    let theta_y  = ((fov / two).tan().recip()/* / (two * near_plane)*/).atan(); */
    theta_x.min(theta_y)
    // near_plane.recip().atan()
    /* fov / two */
}

/// Compute a near-optimal warping parameter that helps minimize error in a
/// shadow map.
///
/// See section 5.2 of Brandon Lloyd's thesis:
///
/// [http://gamma.cs.unc.edu/papers/documents/dissertations/lloyd07.pdf](Logarithmic Perspective Shadow Maps).
///
/// η =
///     0                                                         γ < γ_a
///     -1 + (η_b + 1)(1 + cos(90 (γ - γ_a)/(γ_b - γ_a)))   γ_a ≤ γ < γ_b
///     η_b + (η_c - η_b)  sin(90 (γ - γ_b)/(γ_c - γ_b))    γ_b ≤ γ < γ_c
///     η_c                                                 γ_c ≤ γ
fn compute_warping_parameter<F: Float + FloatConst>(
    gamma: F,
    (gamma_a, gamma_b, gamma_c): (F, F, F),
    (eta_b, eta_c): (F, F),
) -> F {
    if gamma < gamma_a {
        F::zero()
    } else if gamma_a <= gamma && gamma < gamma_b {
        -F::one() + (eta_b + F::one()) * (F::one() + (F::FRAC_PI_2() * (gamma - gamma_a) / (gamma_b - gamma_a)).cos())
    } else if gamma_b <= gamma && gamma < gamma_c {
        eta_b + (eta_c - eta_b) * (F::FRAC_PI_2() * (gamma - gamma_b) / (gamma_c - gamma_b)).sin()
    } else {
        eta_c
    }
    // NOTE: Just in case we go out of range due to floating point imprecision.
    .max(-F::one()).min(F::one())
}

/// Compute a near-optimal warping parameter that falls off quickly enough
/// when the warp angle goes past the minimum field of view angle, for
/// perspective projections.
///
/// For F_p (perspective warping) and view fov angle θ,the parameters are:
///
/// γ_a = θ / 3
/// γ_b = θ
/// γ_c = θ + 0.3(90 - θ)
///
/// η_b = -0.2
/// η_c = 0
///
/// See compute_warping_parameter.
fn compute_warping_parameter_perspective<F: Float + FloatConst>(
    gamma: F,
    near_plane: F,
    fov: F,
    aspect: F,
) -> F {
    let theta = compute_scalar_fov(near_plane, fov, aspect);
    let two = F::one() + F::one();
    let three = two + F::one();
    let ten = three + three + three + F::one();
    compute_warping_parameter(
        gamma,
        (
            theta / three,
            theta,
            theta + (three / ten) * (F::FRAC_PI_2() - theta),
        ),
        (-two / ten, F::zero()),
    )
}

impl Scene {
    /// Create a new `Scene` with default parameters.
    pub fn new(renderer: &mut Renderer, client: &Client, settings: &Settings) -> Self {
        let resolution = renderer.get_resolution().map(|e| e as f32);

        Self {
            globals: renderer.create_consts(&[Globals::default()]).unwrap(),
            lights: renderer
                .create_consts(&[Light::default(); MAX_LIGHT_COUNT])
                .unwrap(),
            shadows: renderer
                .create_consts(&[Shadow::default(); MAX_SHADOW_COUNT])
                .unwrap(),
            shadow_mats: renderer
                .create_consts(&[ShadowLocals::default(); MAX_LIGHT_COUNT * 6 + 6])
                .unwrap(),
            camera: Camera::new(resolution.x / resolution.y, CameraMode::ThirdPerson),
            camera_input_state: Vec2::zero(),

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
            terrain: Terrain::new(renderer),
            lod: Lod::new(renderer, client, settings),
            loaded_distance: 0.0,
            map_bounds: client.world_map.2,
            select_pos: None,
            light_data: Vec::new(),

            figure_mgr: FigureMgr::new(renderer),
            sfx_mgr: SfxMgr::new(),
            music_mgr: MusicMgr::new(),
        }
    }

    /// Get a reference to the scene's globals.
    pub fn globals(&self) -> &Consts<Globals> { &self.globals }

    /// Get a reference to the scene's camera.
    pub fn camera(&self) -> &Camera { &self.camera }

    /// Get a reference to the scene's terrain.
    pub fn terrain(&self) -> &Terrain<TerrainChunk> { &self.terrain }

    /// Get a reference to the scene's figure manager.
    pub fn figure_mgr(&self) -> &FigureMgr { &self.figure_mgr }

    /// Get a mutable reference to the scene's camera.
    pub fn camera_mut(&mut self) -> &mut Camera { &mut self.camera }

    /// Set the block position that the player is interacting with
    pub fn set_select_pos(&mut self, pos: Option<Vec3<i32>>) { self.select_pos = pos; }

    pub fn select_pos(&self) -> Option<Vec3<i32>> { self.select_pos }

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
            // Panning the cursor makes the camera rotate
            Event::CursorPan(delta) => {
                self.camera.rotate_by(Vec3::from(delta) * CURSOR_PAN_SCALE);
                true
            },
            // Zoom the camera when a zoom event occurs
            Event::Zoom(delta) => {
                self.camera
                    .zoom_switch(delta * (0.05 + self.camera.get_distance() * 0.01));
                true
            },
            Event::AnalogGameInput(input) => match input {
                AnalogGameInput::CameraX(d) => {
                    self.camera_input_state.x = d;
                    true
                },
                AnalogGameInput::CameraY(d) => {
                    self.camera_input_state.y = d;
                    true
                },
                _ => false,
            },
            // All other events are unhandled
            _ => false,
        }
    }

    /// Maintain data such as GPU constant buffers, models, etc. To be called
    /// once per tick.
    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        audio: &mut AudioFrontend,
        scene_data: &SceneData,
    ) {
        // Get player position.
        let ecs = scene_data.state.ecs();

        let player_pos = ecs
            .read_storage::<comp::Pos>()
            .get(scene_data.player_entity)
            .map_or(Vec3::zero(), |pos| pos.0);

        let player_rolling = ecs
            .read_storage::<comp::CharacterState>()
            .get(scene_data.player_entity)
            .map_or(false, |cs| cs.is_dodge());

        let is_running = ecs
            .read_storage::<comp::Vel>()
            .get(scene_data.player_entity)
            .map(|v| v.0.magnitude_squared() > RUNNING_THRESHOLD.powi(2))
            .unwrap_or(false);

        let on_ground = ecs
            .read_storage::<comp::PhysicsState>()
            .get(scene_data.player_entity)
            .map(|p| p.on_ground);

        let player_scale = match scene_data
            .state
            .ecs()
            .read_storage::<comp::Body>()
            .get(scene_data.player_entity)
        {
            Some(comp::Body::Humanoid(body)) => SkeletonAttr::calculate_scale(body),
            _ => 1_f32,
        };

        // Add the analog input to camera
        self.camera
            .rotate_by(Vec3::from([self.camera_input_state.x, 0.0, 0.0]));
        self.camera
            .rotate_by(Vec3::from([0.0, self.camera_input_state.y, 0.0]));

        // Alter camera position to match player.
        let tilt = self.camera.get_orientation().y;
        let dist = self.camera.get_distance();

        let up = match self.camera.get_mode() {
            CameraMode::FirstPerson => {
                if player_rolling {
                    player_scale * 0.8
                } else if is_running && on_ground.unwrap_or(false) {
                    player_scale * 1.65 + (scene_data.state.get_time() as f32 * 17.0).sin() * 0.05
                } else {
                    player_scale * 1.65
                }
            },
            CameraMode::ThirdPerson if scene_data.is_aiming => player_scale * 2.1,
            CameraMode::ThirdPerson => player_scale * 1.65,
        };

        self.camera
            .set_focus_pos(player_pos + Vec3::unit_z() * (up - tilt.min(0.0).sin() * dist * 0.6));

        // Tick camera for interpolation.
        self.camera.update(
            scene_data.state.get_time(),
            scene_data.state.get_delta_time(),
            scene_data.mouse_smoothing,
        );

        // Compute camera matrices.
        self.camera.compute_dependents(&*scene_data.state.terrain());
        let camera::Dependents {
            view_mat,
            proj_mat,
            cam_pos,
        } = self.camera.dependents();

        // Update chunk loaded distance smoothly for nice shader fog
        let loaded_distance =
            (0.98 * self.loaded_distance + 0.02 * scene_data.loaded_distance).max(0.01);

        // Update light constants
        let lights = &mut self.light_data;
        lights.clear();
        lights.extend(
            (
                &scene_data.state.ecs().read_storage::<comp::Pos>(),
                scene_data.state.ecs().read_storage::<comp::Ori>().maybe(),
                scene_data
                    .state
                    .ecs()
                    .read_storage::<crate::ecs::comp::Interpolated>()
                    .maybe(),
                &scene_data
                    .state
                    .ecs()
                    .read_storage::<comp::LightAnimation>(),
            )
                .join()
                .filter(|(pos, _, _, _)| {
                    (pos.0.distance_squared(player_pos) as f32)
                        < loaded_distance.powf(2.0) + LIGHT_DIST_RADIUS
                })
                .map(|(pos, ori, interpolated, light_anim)| {
                    // Use interpolated values if they are available
                    let (pos, ori) =
                        interpolated.map_or((pos.0, ori.map(|o| o.0)), |i| (i.pos, Some(i.ori)));
                    let rot = {
                        if let Some(o) = ori {
                            Mat3::rotation_z(-o.x.atan2(o.y))
                        } else {
                            Mat3::identity()
                        }
                    };
                    Light::new(
                        pos + (rot * light_anim.offset),
                        light_anim.col,
                        light_anim.strength,
                    )
                }),
        );
        lights.sort_by_key(|light| light.get_pos().distance_squared(player_pos) as i32);
        lights.truncate(MAX_LIGHT_COUNT);
        renderer
            .update_consts(&mut self.lights, &lights)
            .expect("Failed to update light constants");

        // Update shadow constants
        let mut shadows = (
            &scene_data.state.ecs().read_storage::<comp::Pos>(),
            scene_data
                .state
                .ecs()
                .read_storage::<crate::ecs::comp::Interpolated>()
                .maybe(),
            scene_data.state.ecs().read_storage::<comp::Scale>().maybe(),
            &scene_data.state.ecs().read_storage::<comp::Body>(),
            &scene_data.state.ecs().read_storage::<comp::Stats>(),
        )
            .join()
            .filter(|(_, _, _, _, stats)| !stats.is_dead)
            .filter(|(pos, _, _, _, _)| {
                (pos.0.distance_squared(player_pos) as f32)
                    < (loaded_distance.min(SHADOW_MAX_DIST) + SHADOW_DIST_RADIUS).powf(2.0)
            })
            .map(|(pos, interpolated, scale, _, _)| {
                Shadow::new(
                    // Use interpolated values pos if it is available
                    interpolated.map_or(pos.0, |i| i.pos),
                    scale.map_or(1.0, |s| s.0),
                )
            })
            .collect::<Vec<_>>();
        shadows.sort_by_key(|shadow| shadow.get_pos().distance_squared(player_pos) as i32);
        shadows.truncate(MAX_SHADOW_COUNT);
        renderer
            .update_consts(&mut self.shadows, &shadows)
            .expect("Failed to update light constants");

        // Remember to put the new loaded distance back in the scene.
        self.loaded_distance = loaded_distance;

        // Update light projection matrices for the shadow map.
        let time_of_day = scene_data.state.get_time_of_day();
        let focus_pos = self.camera.get_focus_pos();
        let focus_off = focus_pos.map(|e| e.trunc());

        // Update global constants.
        renderer
            .update_consts(&mut self.globals, &[Globals::new(
                view_mat,
                proj_mat,
                cam_pos,
                focus_pos,
                self.loaded_distance,
                self.lod.get_data().tgt_detail as f32,
                self.map_bounds,
                time_of_day,
                scene_data.state.get_time(),
                renderer.get_resolution(),
                Vec2::new(SHADOW_NEAR, SHADOW_FAR),
                lights.len(),
                shadows.len(),
                NUM_DIRECTED_LIGHTS,
                scene_data
                    .state
                    .terrain()
                    .get((cam_pos + focus_off).map(|e| e.floor() as i32))
                    .map(|b| b.kind())
                    .unwrap_or(BlockKind::Air),
                self.select_pos.map(|e| e - focus_off.map(|e| e as i32)),
                scene_data.gamma,
                self.camera.get_mode(),
                scene_data.sprite_render_distance as f32 - 20.0,
            )])
            .expect("Failed to update global constants");

        // Maintain LoD.
        self.lod.maintain(renderer, time_of_day);

        // Maintain the terrain.
        let (_scene_bounds, visible_bounds, _psc_bounds) = self.terrain.maintain(
            renderer,
            &scene_data,
            focus_pos,
            self.loaded_distance,
            view_mat,
            proj_mat,
        );

        // Maintain the figures.
        let _figure_bounds = self.figure_mgr.maintain(renderer, scene_data, &self.camera);

        let sun_dir = scene_data.get_sun_dir();
        let is_daylight = sun_dir.z < 0.0/*0.6*/;
        if renderer.render_mode().shadow == render::ShadowMode::Map
            && (is_daylight || !lights.is_empty())
        {
            /* // We treat the actual scene bounds as being clipped by the horizontal terrain bounds, but
            // expanded to contain the z values of all NPCs.  This is potentially important to make
            // sure we don't clip out figures in front of the camera.
            let visible_bounds = Aabb {
                min: Vec3::new(visible_bounds.min.x, visible_bounds.min.y, visible_bounds.min.z.min(figure_bounds.min.z)),
                max: Vec3::new(visible_bounds.max.x, visible_bounds.max.y, visible_bounds.max.z.max(figure_bounds.max.z)),
            }; */

            // let focus_frac = focus_pos.map(|e| e.fract());
            let visible_bounds = math::Aabb::<f32> {
                min: math::Vec3::from(visible_bounds.min - focus_off),
                max: math::Vec3::from(visible_bounds.max - focus_off),
            };
            let visible_bounds_fine = math::Aabb {
                min: visible_bounds.min.map(f64::from),
                max: visible_bounds.max.map(f64::from),
            };
            /* let visible_bounds = fit_psr(proj_mat * view_mat, visible_bounds, |p| (Vec3::from(p) / p.w)/*.map(|e| e.clamped(-1.0, 1.0))*/);
            // clip bounding box points to positions that are actually visible.
            // let visible_bounds_projected: aabb = fit_psr(proj_mat * view_mat, visible_bounds);
            let inverse_visible: Mat4<f32> = (proj_mat * view_mat
                // .scaled_3d(vec3::new(proj_mat[(0, 0)], proj_mat[(1, 1)], 1.0))
            ).inverted();/* Mat4::identity();*/
            let visible_bounds = fit_psr(inverse_visible, visible_bounds, |p| Vec3::from(p) / p.w); */
            // let visible_pts = aabb_to_points(visible_bounds);
            /* let scene_bounds = Aabb {
                min: (scene_bounds.min - focus_off),
                max: (scene_bounds.max - focus_off),
            };
            let scene_bounds_fine = Aabb {
                min: scene_bounds.min.map(f64::from),
                max: scene_bounds.max.map(f64::from),
            }; */
            let inv_proj_view = math::Mat4::from_col_arrays(
                (proj_mat * view_mat/* * Mat4::translation_3d(-focus_off)*/).into_col_arrays(),
            )
            .map(f64::from)
            .inverted();

            let fov = self.camera.get_fov();
            let aspect_ratio = self.camera.get_aspect_ratio();

            /* println!("view_mat: {:?}", view_mat);
            println!("scene_bounds: {:?} visible_bounds: {:?}", scene_bounds, visible_bounds); */
            let view_dir = ((focus_pos.map(f32::fract)) - cam_pos).normalized();
            let (point_shadow_res, _directed_shadow_res) = renderer.get_shadow_resolution();
            // NOTE: The aspect ratio is currently always 1 for our cube maps, since they
            // are equal on all sides.
            let point_shadow_aspect = point_shadow_res.x as f32 / point_shadow_res.y as f32;
            // Construct matrices to transform from world space to light space for the sun
            // and moon.
            let directed_light_dir = math::Vec3::from(sun_dir);
            /* let light_volume = calc_focused_light_volume_points(inv_proj_view, directed_light_dir.map(f64::from), scene_bounds_fine, 1e-3)
                // .map(|e| e - focus_off)
                // NOTE: Hopefully not out of bounds.
                .map(|v| v.map(|e| e as f32))
                .collect::<Vec<_>>();
            // println!("light_volume: {:?}", light_volume); */
            // let visible_light_volume = light_volume.clone();
            let visible_light_volume = math::calc_focused_light_volume_points(inv_proj_view, directed_light_dir.map(f64::from), visible_bounds_fine, 1e-6)
                // .map(|e| e - focus_off)
                // NOTE: Hopefully not out of bounds.
                .map(|v| v.map(|e| e as f32))
                .collect::<Vec<_>>();
            // println!("visible_light_volume: {:?}", visible_light_volume);
            // let bounds0 = fit_psr(Mat4::identity()/* * inverse_visible*/,
            // light_volume.iter().copied(), |p| Vec3::from(p) / p.w);
            /* let light_volume = calc_focused_light_volume_points(inv_proj_view, directed_light_dir.map(f64::from), Aabb {
                min: visible_bounds.min.map(f64::from),
                max: visible_bounds.max.map(f64::from),
            }, 1e-3)
                // .map(|e| e - focus_off)
                // NOTE: Hopefully not out of bounds.
                .map(|v| v.map(|e| e as f32))
                .collect::<Vec<_>>(); */
            // First, add a projected matrix for our directed hard lights.
            // NOTE: This can be hard, so we should probably look at techniques for
            // restricting what's in the light frustum for things like sunlight
            // (i.e. potential shadow receivers and potential shadow casters, as
            // well as other culling). The sun position is currently scaled so
            // that the focus is halfway between the near plane and far plane;
            // however, there is probably a much smarter way to do this.
            // NOTE: Multiplying by 1.5 as an approxmiation for √(2)/2, to make sure we
            // capture all chunks.
            let radius = /*loaded_distance;// *//*/*scene_bounds*/scene_bounds.half_size().reduce_partial_max() * 1.5*/0.75/*bounds0/*scene_bounds*/.half_size().reduce_partial_max()*/;

            // Optimal warping for directed lights:
            //
            // n_opt = 1 / sin y (z_n + √(z_n + (f - n) sin y))
            //
            // where n is near plane, f is far plane, y is the tilt angle between view and
            // light directon, and n_opt is the optimal near plane.
            let directed_near = 1.0/*0.5*/;
            let _directed_far = /*128.0*/directed_near + /*loaded_distance * 2.0*/2.0 * radius;
            /* let directed_proj_mat = Mat4::orthographic_rh_no/*orthographic_without_depth_planes*/(FrustumPlanes {
                // TODO: Consider adjusting resolution based on view distance.
                left: -/*loaded_distance*/radius,
                // left: -(directed_shadow_res.x as f32) / 2.0,
                right: /*loaded_distance*/radius,
                // right: directed_shadow_res.x as f32 / 2.0,
                bottom: -/*loaded_distance*/radius,
                // bottom: -(directed_shadow_res.y as f32) / 2.0,
                top: /*loaded_distance*/radius,
                // top: directed_shadow_res.y as f32 / 2.0,
                // TODO: Swap fixed near and far planes for something dependent on the height of the
                // current scene.
                near: directed_near,
                far: directed_far,
            }); */
            // let directed_proj_mat = Mat4::identity();
            // We also want a way to transform and scale this matrix (* 0.5 + 0.5) in order
            // to transform it correctly into texture coordinates, as well as
            // OpenGL coordinates.  Note that the matrix for directional light
            // is *already* linear in the depth buffer.
            let texture_mat = Mat4::scaling_3d(0.5f32) * Mat4::translation_3d(1.0f32);
            // We need to compute these offset matrices to tranform world space coordinates
            // to the translated ones we use when multiplying by the light space
            // matrix; this helps avoid precision loss during the
            // multiplication.

            // let moon_dir = scene_data.get_moon_dir();
            // let moon_dir = Vec3::new(-angle_rad.sin(), 0.0, angle_rad.cos() - 0.5);
            // Parallel light is aimed dead at the nearest integer to our focus position; if
            // we were to offset by focus_off, it would be *at* our focus
            // position, but using zero may result in less precision loss
            // overall. NOTE: We could also try to use the offset of the
            // *camera* position from the focus spot, to make shadows near the
            // eye as sharp as possible. NOTE: If there's precision loss during
            // the matrix *calcuation*, how much could be resolved by just using
            // f64 in Rust for the computation, and switching to f32 afterwards
            // just for the GPU?
            // let look_at = bounds0.center();//Vec3::zero();//
            // scene_bounds.center();//Vec3::zero(); let look_at =
            // bounds0.center();
            let look_at = math::Vec3::from(cam_pos); // /*Vec3::zero()*/scene_bounds.center()/*cam_pos*/;// - focus_off;// focus_off;
            let _light_scale = 1.5 * /*(directed_near + directed_far) / 2.0*/radius;
            // We upload view matrices as well, to assist in linearizing vertex positions.
            // (only for directional lights, so far).
            let mut directed_shadow_mats = Vec::with_capacity(6);
            let new_dir = math::Vec3::from(view_dir);
            // let new_dir: Vec3<f32> = light_volume/*visible_light_volume*/.iter().map(|p|
            // p - cam_pos).sum();
            let new_dir = new_dir.normalized();
            /* let dot_prod = f64::from(directed_light_dir.dot(new_dir));
            let sin_gamma = (1.0 - dot_prod * dot_prod).sqrt();
            // let sin_gamma = 0.0;
            let new_dir = if /*sin_gamma > EPISLON_GAMMA*/factor != -1.0 {
                new_dir
            } else {
                // For uniform mapping, align shadow map t axis with viewer's y axis to maximize
                // utilization of the shadow map.
                Vec3::from(view_mat * Vec4::from_direction(Vec3::up()))
                    .normalized()
            }; */
            let up: math::Vec3<f32> = {
                /* (directed_light_dir)
                .cross(new_dir)
                .cross(directed_light_dir)
                .normalized() */
                math::Vec3::up()
            };
            // let up = Vec3::up();
            // let up: Vec3<f32> = Vec3::from(Mat4::<f32>::look_at_rh(look_at - sun_dir,
            // look_at, -Vec3::from(view_dir)) * Vec4::<f32>::forward_rh());
            // println!("bounds0: {:?}, scene_bounds: {:?}", bounds0, scene_bounds);
            directed_shadow_mats.push(math::Mat4::look_at_rh(
                look_at,
                look_at + directed_light_dir,
                /* Vec3::up()*//*Vec3::from(view_dir)*//*up*//*Vec3::down() */ up,
            ));
            // directed_shadow_mats.push(Mat4::look_at_rh(look_at - sun_dir * light_scale,
            // look_at, /*Vec3::up()*//*Vec3::from(view_dir)*//*up*//*Vec3::down()*/up));
            // directed_shadow_mats.push(Mat4::look_at_rh(look_at - moon_dir * light_scale,
            // look_at, Vec3::up())); This leaves us with four dummy slots,
            // which we push as defaults.
            directed_shadow_mats
                .extend_from_slice(&[math::Mat4::default(); 6 - NUM_DIRECTED_LIGHTS] as _);
            // Now, construct the full projection matrices in the first two directed light
            // slots.
            let mut shadow_mats = Vec::with_capacity(6 * (lights.len() + 1));
            // let cam_pos = self.camera.dependents().cam_pos - focus_off;
            /* let all_mat = /*proj_mat * */view_mat
                .scaled_3d(Vec3::new(proj_mat[(0, 0)], proj_mat[(1, 1)], 1.0));
            let focus_off = focus_pos.map(|e| e.trunc()); */
            let z_n = 1.0; //f64::from(camera::NEAR_PLANE);
            let _z_f = f64::from(camera::FAR_PLANE);
            let _scalar_fov = f64::from(fov / 2.0); // compute_scalar_fov(z_n, f64::from(fov), f64::from(aspect_ratio));
            shadow_mats.extend(directed_shadow_mats.iter().map(move |&light_view_mat| {
                /* let visible_light_volume = {
                    let light_view_mat = light_view_mat.map(f64::from);
                    // (See http://www.songho.ca/opengl/gl_normaltransform.html)
                    // NOTE: What we really need here is the transpose of the matrix inverse:
                    // (M⁻¹)ᵀ
                    //
                    // where M is the light projection-view matrix.
                    //
                    // However, since we (hopefully) only have rotational transformations and
                    // transpositions for directional lights, and transpositions can be ignored by
                    // setting the w component of a vector to 0 (which is what we do when multiplying
                    // by the normal vector), we can treat M as an orthogonal matrix when multiplying
                    // by the normal.  Thus the matrix inverse M⁻¹ can be treated as equal to its
                    // transpose Mᵀ, so the transpose of the inverse can be treated as equal to
                    // (Mᵀ)ᵀ = M for this purpose.
                    let inv_light_view_mat_transpose = light_view_mat;
                    let world_pts = calc_view_frustum_world_coord(light_view_mat * inv_proj_view);
                    // println!("world_pts: {:?}", world_pts);
                    let mut world_frust_object = calc_view_frust_object(&world_pts);
                    // println!("world_frust_object: {:?}", world_frust_object);
                    // clip_object_by_aabb(&mut world_frust_object, scene_bounding_box, tolerance);
                    {
                        let mut planes = aabb_to_planes(Aabb {
                            min: visible_bounds.min.map(f64::from),
                            max: visible_bounds.max.map(f64::from),
                        });
                        /* let new_origin = light_view_mat * Vec4::unit_w();
                        let new_origin = Vec3::from(new_origin) / new_origin.w; */
                        planes.iter_mut().for_each(|plane| {
                            println!("old plane: {:?}", plane);
                            // NOTE: We may be able to simplify this to one matrix multiplication in
                            // this case (avoiding handling w separately) using the adjunction, but
                            // it's not clear whether it would be a performance win if it requires
                            // calculating the full matrix inverse.
                            let new_plane = inv_light_view_mat_transpose * Vec4::from_direction(plane.0);
                            /* let new_plane = light_view_mat * Vec4::new(plane.0.x, plane.0.y, plane.0.z, plane.1); */
                            /* let new_plane = light_view_mat * Vec4::new(plane.0.x * plane.1, plane.0.y * plane.1, plane.0.z * plane.1, /*1.0*/0.0); */
                            // We can easily find a point on the plane by multiplying the normal by the
                            // distance, and of course we only need to transform this point using the
                            // original matrix to find its new position.
                            let new_point = light_view_mat * Vec4::from_point(plane.0 * plane.1);
                            // NOTE: We currently assume no scaling, so length is 1.0.
                            let length: f64 = 1.0/*Vec3::from(new_plane).magnitude()*/;
                            let new_norm = Vec3::from(new_plane) / length;
                            // The new distance to the origin is the dot product of the transformed
                            // point on the plane's 3D coordinates, and the vector normal to the plane;
                            // this is because we have
                            //  cos θ_new_point,new_norm = new_point ⋅ new_norm / (||origin|| ||new_norm||)
                            //                           = new_point ⋅ new_norm / ||origin||
                            //  ||origin|| cos θ_new_point,new_norm = new_point ⋅ new_norm
                            // which is exactly the projection of the vector from the origin to
                            // new_point onto the plane normal new_norm, i.e. the plane's distance
                            // from the origin.
                            *plane = (new_norm, Vec3::from(new_point).dot(new_norm));
                            /* *plane = (Vec3::from(new_plane) / length, length); */
                            /* let sgn = new_plane.w.signum();
                            *plane = (sgn * Vec3::from(new_plane) / length, sgn * new_plane.w * length); */
                            println!("new plane: {:?}", plane);
                            /* let new_plane = Vec3::from(light_view_mat * Vec4::from_direction(plane.0));
                            *plane = (new_plane / new_plane.w, plane.1 / new_plane.w); */
                        });
                        // println!("planes@clip_object_by_aabb: {:?}", planes);
                        planes.iter().for_each(|&plane| {
                            clip_object_by_plane(&mut world_frust_object, plane, 1e-3);
                            // println!("polys@clip_object_by_aabb (after clipping by {:?}): {:?}", plane, polys);
                        });
                    }
                    world_frust_object.into_iter().flat_map(|e| e.into_iter())
                        .map(|v| v.map(|e| e as f32))
                        .collect::<Vec<_>>()
                    //
                    // println!("world_frust_object@clip_object_by_aabb: {:?}", world_frust_object);
                    // include_object_light_volume(world_frust_object.into_iter().flat_map(|e| e.into_iter()), Vec3::forward_rh(), scene_bounding_box)
                };
                println!("visible_light_volume: {:?}", visible_light_volume); */

                // let mut e_p: Vec4<f32> = light_view_mat * Vec4::new(cam_pos.x, cam_pos.y, cam_pos.z, 1.0);
                /* let mut v_p: Vec4<f32> = /*directed_proj_mat * */light_view_mat * Vec4::from_direction(/*up*/new_dir);// Vec4::new(view_dir.x, view_dir.y, view_dir.z, 1.0);
                //
                // gluLookAt(e, p, y') /
                //  Mat4::look_at_rh(eye, target, up) /
                //  MathStuff::look(output, pos, dir, up) ~ Mat4::look_at_rh(pos, pos + dir, -up)
                //
                // eye point e = eye
                // point p to look at = target
                // up vector y' = up
                //
                // Let
                //  c = normalize(e - p)
                //  a = (y' × c) / ||y'|| = normalize(y' × c)
                //  b = c × a
                //
                // Then M_v =
                // (a_x a_y a_z -(a⋅e)
                //  b_x b_y b_z -(b⋅e)
                //  c_x c_y c_z -(c⋅e)
                //  0   0   0   1)
                //
                //  c = -lightDir
                //  y' = -viewDir
                //
                //  MathStuff::look(output, pos, dir, up) ~ Mat4::look_at_rh(pos, pos + dir, up):
                //    e = pos
                //    c = normalize(pos - (pos + dir)) = normalize(-dir) = -normalize(dir) = -dirN
                //    a = normalize(-up × c) = normalize(up × -normalize(dir)) = normalize(-(up × dir))
                //      = normalize(dir × up) = lftN
                //    b = c × a = -normalize(dir) × lftN = normalize(-(dir × lftN))
                //      = normalize(lftN × dir) = upN
                //    output =
                //    (lftN_x   lftN_y  lftN_z  -(lftN⋅pos)
                //     upN_x    upN_y   upN_z   -(upN⋅pos)
                //     -dirN_x  -dirN_y -dirN_z dirN⋅pos
                //     0        0       0       1) =
                //   (a_x       a_y     a_z     -(a⋅e)
                //    b_x       b_y     b_z     -(b⋅e)
                //    -(-c)_x   -(-c)_y -(-c)_z (-c)⋅e
                //    0         0       0       1) =
                //   (a_x a_y a_z -(a⋅e)
                //    b_x b_y b_z -(b⋅e)
                //    c_x c_y c_z -(c⋅e)
                //    0   0   0   1)
                //
                let mut e_p: Vec3<f32> = Vec3::zero();
                v_p.z = 0.0; */
                let mut v_p = math::Vec3::from(light_view_mat * math::Vec4::from_direction(new_dir));
                v_p.normalize();
                // let dot_prod = f64::from(v_p.z);
                let dot_prod = new_dir.map(f64::from).dot(directed_light_dir.map(f64::from));
                let sin_gamma = (1.0 - dot_prod * dot_prod).sqrt();
                let gamma = sin_gamma.asin();
                let factor = compute_warping_parameter_perspective(gamma, f64::from(camera::NEAR_PLANE), f64::from(fov), f64::from(aspect_ratio));
                /* let factor = if factor > 0.0 {
                    -1.0
                } else {
                    factor
                };*/

                v_p.z = 0.0;
                v_p.normalize();
                let l_r: math::Mat4<f32> = if /*v_p.magnitude_squared() > 1e-3*//*sin_gamma > EPISLON_GAMMA*/factor != -1.0 {
                    math::Mat4::look_at_rh(math::Vec3::zero(), math::Vec3::forward_rh(), v_p)
                } else {
                    math::Mat4::identity()
                };
                // let factor = -1.0;
                // let l_r: Mat4<f32> = Mat4::look_at_rh(/*Vec3::from(e_p) - Vec3::from(v_p)*//*Vec3::up()*/e_p, /*Vec3::from(e_p)*//*Vec3::zero()*/e_p + Vec3::forward_rh(), Vec3::from(v_p));
                // let l_r: Mat4<f32> = Mat4::look_at_rh(/*Vec3::from(e_p) - Vec3::from(v_p)*//*Vec3::up()*/-Vec3::from(v_p), /*Vec3::from(e_p)*/Vec3::zero(), Vec3::back_rh());
                // let l_r: Mat4<f32> = Mat4::identity();//Mat4::look_at_rh(/*Vec3::from(e_p) - Vec3::from(v_p)*//*Vec3::up()*/-Vec3::from(v_p), /*Vec3::from(e_p)*/Vec3::zero(), Vec3::back_rh());
                // let l_r: Mat4<f32> = Mat4::look_at_rh(/*Vec3::from(e_p) - Vec3::from(v_p)*//*Vec3::up()*/-Vec3::from(v_p), /*Vec3::from(e_p)*/Vec3::zero(), Vec3::back_rh());
                // let l_r: Mat4<f32> = Mat4::look_at_rh(Vec3::from(e_p) - Vec3::from(v_p), Vec3::from(e_p), Vec3::forward_rh());
                // let l_r: Mat4<f32> = Mat4::look_at_rh(/*Vec3::from(e_p) - Vec3::from(v_p)*/Vec3::zero(), /*Vec3::from(e_p)*/-Vec3::forward_rh(), /*Vec3::up()*/-Vec3::from(v_p));
                // let l_r: Mat4<f32> = Mat4::look_at_rh(/*Vec3::from(e_p) - Vec3::from(v_p)*/Vec3::back_rh(), /*Vec3::from(e_p)*/Vec3::zero(), /*Vec3::up()*/Vec3::from(v_p));
                // let l_r: Mat4<f32> = Mat4::identity();
                let bounds0 = math::fit_psr(light_view_mat, visible_light_volume.iter().copied(), /*|p| math::Vec3::from(p) / p.w*/math::Vec4::homogenized);
                let directed_proj_mat = math::Mat4::orthographic_rh_no(FrustumPlanes {
                    // TODO: Consider adjusting resolution based on view distance.
                    left: bounds0.min.x,
                    right: bounds0.max.x,
                    bottom: bounds0.min.y,
                    top: bounds0.max.y,
                    near: bounds0.min.z,
                    far: bounds0.max.z,
                })/* /Mat4::identity() */;

                let light_all_mat = l_r * directed_proj_mat * light_view_mat;
                // let bounds1 = fit_psr(light_all_mat/* * inverse_visible*/, light_volume.iter().copied(), |p| Vec3::from(p) / p.w);
                let bounds0 = math::fit_psr(/*l_r*/light_all_mat/* * inverse_visible*/, visible_light_volume.iter().copied(), /*|p| math::Vec3::from(p) / p.w*/math::Vec4::homogenized);
                // let bounds1 = fit_psr(light_all_mat/* * inverse_visible*/, aabb_to_points(visible_bounds).iter().copied(), |p| Vec3::from(p) / p.w);
                // let mut light_focus_pos: Vec3<f32> = Vec3::from(light_all_mat * Vec4::from_point(focus_pos.map(f32::fract)));
                let mut light_focus_pos: math::Vec3<f32> = math::Vec3::zero();//bounds0.center();// l_r * directed_proj_mat * light_view_mat * Vec4::from_point(focus_pos.map(|e| e.fract()));
                // let mut light_focus_pos: Vec3<f32> = bounds0.center();// l_r * directed_proj_mat * light_view_mat * Vec4::from_point(focus_pos.map(|e| e.fract()));
                // println!("cam_pos: {:?}, focus_pos: {:?}, light_focus_pos: {:?}, v_p: {:?} bounds: {:?}, l_r: {:?}, light_view_mat: {:?}, light_all_mat: {:?}", cam_pos, focus_pos - focus_off, light_focus_pos, v_p, /*bounds1*/bounds0, l_r, light_view_mat, light_all_mat);
                // let w_v = Mat4::translation_3d(-Vec3::new(xmax + xmin, ymax + ymin, /*zmax + zmin*/0.0) / 2.0);

                // let dot_prod = /*new_dir*//*up_dir*/view_dir.map(f64::from).dot(directed_light_dir.map(f64::from));
                // let sin_gamma = (1.0 - dot_prod * dot_prod).sqrt();//.clamped(1e-1, 1.0);
                // let sin_gamma = 0.0;
                // let factor = -1.0;//1.0 / sin_gamma;
                // println!("Warp factor for γ (sin γ = {:?}, γ = {:?}, near_plane = {:?}, fov = {:?}, scalar fov = {:?}, aspect ratio = {:?}): η = {:?}", sin_gamma, gamma.to_degrees(), camera::NEAR_PLANE, fov.to_degrees(), scalar_fov.to_degrees(), aspect_ratio, factor);
               /* v ---l
                \ Θ| 
                  \| */

                // let directed_near = /*0.5*//*0.25*/f64::from(camera::NEAR_PLANE);/*1.0*/;//bounds0.min.y.max(1.0);
                // let z_n = /*f64::from(bounds0.min.y)*//*factor * *//*f64::from(*/directed_near/*)*/;// / /*sin_gamma*/scalar_fov.cos();// / sin_gamma; //often 1
                let d = f64::from(bounds0.max.y - bounds0.min.y/*directed_near*/).abs(); //perspective transform depth //light space y extents
                // let z_f = z_n + d * camera::FAR_PLANE/* / scalar_fov.cos()*/;
                // let z_0 = f64::from(bounds0.min.y);

                // Vague idea: project z_n from the camera view to the light view (where it's
                // tilted by γ).
                let z_0 = z_n;// / sin_gamma;// / sin_gamma;
                // let z_1 = z_0 + d;
                // Vague idea: project d from the light view back to the camera view (undoing the
                // tilt by γ).
                let z_1 = /*z_n*/z_0 + d * sin_gamma;
                let w_l_y = /* z_f - z_n */d;/*/*f64::from(camera::FAR_PLANE - camera::NEAR_PLANE)*//*(z_f - z_n)*/d * scalar_fov.cos();*/
                // let z_f = z_n + d;
                // let near_dist = directed_near;
                // let factor = -1.0;
                /* let factor = if factor == -1.0 {
                    -1.0
                } else {
                    0.0
                }; */

                // NOTE: See section 5.1.2.2 of Lloyd's thesis.
                let alpha = z_1 / z_0/*z_f / z_n*/;
                let alpha_sqrt = alpha.sqrt();
                let directed_near_normal = if factor < 0.0 {
                    // Standard shadow map to LiSPSM
                    (1.0 + alpha_sqrt - factor * (alpha - 1.0)) / ((alpha - 1.0) * (factor + 1.0))
                    // 1+sqrt(z_f/z_n)/((z_f/z_n - 1)*2)
                } else {
                    // LiSPSM to PSM
                    ((alpha_sqrt - 1.0) * (factor * alpha_sqrt + 1.0)).recip()
                    // LiSPSM: 1 / ((√α - 1) * (η√α + 1))
                    //      = 1 / ((√α - 1)(1))
                    //      = 1 / (√α - 1)
                    //      = (1 + √α) / (α - 1)
                    //      = (a + √(z_f/z_n)) / (z_f/z_n - 1)
                };
                // let factor = -1.0;

                // Equation 5.14 - 5.16
                // let directed_near_normal = 1.0 / d * (z_0 + (z_0 * z_1).sqrt());
                // let directed_near = w_l_y / d * (z_0 + (z_0 * z_1).sqrt());
                /* let directed_near = directed_near_normal as f32;
                let directed_far = (directed_near_normal + d) as f32; */
                let directed_near = (w_l_y * directed_near_normal).abs() as f32;
                let directed_far = (w_l_y * (directed_near_normal + 1.0)).abs() as f32;
                let (directed_near, directed_far) = (directed_near.min(directed_far), directed_near.max(directed_far));
                // let directed_near = w_l_y / d * (z_0 + (z_0 * z_1).sqrt());
                // println!("θ = {:?} η = {:?} z_n = {:?} z_f = {:?} γ = {:?} d = {:?} z_0 = {:?} z_1 = {:?} w_l_y: {:?} α = {:?} √α = {:?} n'₀ = {:?} n' = {:?} f' = {:?}", scalar_fov.to_degrees(), factor, z_n, z_f, gamma.to_degrees(), d, z_0, z_1, w_l_y, alpha, alpha_sqrt, directed_near_normal, directed_near, directed_far);

                // let directed_near = /*camera::NEAR_PLANE / sin_gamma*/camera::NEAR_PLANE;
                //let near_dist = directed_near as f32;
                // let directed_far = directed_near + (camera::FAR_PLANE - camera::NEAR_PLANE);
                /* // let directed_near = 1.0;
                let directed_near = ((z_n + (z_f * z_n).sqrt()) / /*sin_gamma*/factor) as f32; //1.0; */
                // let directed_far = directed_near + d as f32;
                // println!("view_dir: {:?}, new_dir: {:?}, directed_light_dir: {:?}, dot_prod: {:?}, sin_gamma: {:?}, near_dist: {:?}, d: {:?}, z_n: {:?}, z_f: {:?}, directed_near: {:?}, directed_far: {:?}", view_dir, new_dir, directed_light_dir, dot_prod, sin_gamma, near_dist, d, z_n, z_f, directed_near, directed_far);
                /* let size1 = bounds1.half_size();
                let center1 = bounds1.center(); */
                /* let look_at = cam_pos - (directed_near - near_dist) * up;
                let light_all_mat: Mat4<f32> = Mat4::look_at_rh(look_at, look_at + directed_light_dir, /*Vec3::up()*//*Vec3::from(view_dir)*//*up*//*Vec3::down()*/up); */
                // let look_at = look_at - (directed_near - near_dist) * up;
                // let light_view_mat = l_r * Mat4::look_at_rh(look_at - sun_dir * light_scale, look_at, /*Vec3::up()*//*Vec3::from(view_dir)*/up);
                // let w_v: Mat4<f32> = Mat4::identity();
                // let w_v: Mat4<f32> = Mat4::translation_3d(/*-bounds1.center()*/-center1);
		        //new observer point n-1 behind eye position
		        //pos = eyePos-up*(n-nearDist)
                // let directed_near = if /*sin_gamma > EPISLON_GAMMA*/factor != -1.0 { directed_near } else { near_dist/*0.0*//*-(near_dist *//*- light_focus_pos.y)*/ };
                light_focus_pos.y = if factor != -1.0 {
                    /*near_dist*/z_n as f32 - directed_near
                } else {
                    light_focus_pos.y
                };
                let w_v: math::Mat4<f32> = math::Mat4::translation_3d(/*-bounds1.center()*/-math::Vec3::new(light_focus_pos.x, light_focus_pos.y/* + (directed_near - near_dist)*/,/* - /*(directed_near - near_dist)*/directed_near*//*bounds1.center().z*//*directed_near*//*bounds1.min.z - *//*(directed_near - near_dist)*//*focus_pos.z*//*light_focus_pos.z*//*light_focus_pos.z*//*center1.z*//*center1.z.max(0.0)*/light_focus_pos.z));
                // let w_v: Mat4<f32> = Mat4::translation_3d(/*-bounds1.center()*/-Vec3::new(light_focus_pos.x, light_focus_pos.y,/* - /*(directed_near - near_dist)*/directed_near*//*bounds1.center().z*//*directed_near*//*bounds1.min.z - *//*(directed_near - near_dist)*//*focus_pos.z*//*light_focus_pos.z*//*light_focus_pos.z*/center1.z + directed_near - near_dist));
                // let w_v: Mat4<f32> = Mat4::translation_3d(/*-bounds1.center()*/-Vec3::new(0.0, 0.0,/* - /*(directed_near - near_dist)*/directed_near*//*bounds1.center().z*//*directed_near*//*bounds1.min.z - *//*(directed_near - near_dist)*//*focus_pos.z*//*light_focus_pos.z*/directed_near - near_dist));
                /* let w_p: Mat4<f32> = Mat4::orthographic_rh_no/*frustum_rh_no*/(FrustumPlanes {
                // TODO: Consider adjusting resolution based on view distance.
                    left: -1.0// + (center1.x - focus_pos.x) / size1.w,
                    // left: -(directed_shadow_resx as f32) / 2.0,
                    right: 1.0// + (center1.x - focus_pos.x) / size1.w,
                    // right: directed_shadow_res.x as f32 / 2.0,
                    bottom: -1.0// + (center1.y - focus_pos.y) / size1.h,
                    // bottom: -(directed_shadow_res.y as f32) / 2.0,
                    top: 1.0// + (center1.y - focus_pos.y) / size1.h,
                    // top: directed_shadow_res.y as f32 / 2.0,
                    // TODO: Swap fixed near and far planes for something dependent on the height of the
                    // current scene.
                    near: directed_near,
                    far: directed_far,// directed_near + /*zmax - zmin*/bounds1.max.z - bounds1.min.z,//directed_far,
                }); */
                let shadow_view_mat: math::Mat4<f32> = w_v * light_all_mat;
                let _bounds0 = math::fit_psr(/*l_r*/shadow_view_mat/* * inverse_visible*/, visible_light_volume.iter().copied(), /*|p| math::Vec3::from(p) / p.w*/math::Vec4::homogenized);
                // let factor = -1.0;
                let w_p: math::Mat4<f32> = {
                    if /*sin_gamma > EPISLON_GAMMA*/factor != -1.0 {
                        // Projection for y
                        let n = directed_near;// - near_dist;
                        let f = directed_far;
                        let l = -1.0;// bounds0.min.x;//-1.0;// bounds0.min.x - light_focus_pos.x;
                        let r = 1.0;// bounds0.max.x;//1.0;// bounds0.max.x - light_focus_pos.x;
                        let b = -1.0;// bounds0.max.z;// bounds0.max.z - light_focus_pos.z;
                        let t = 1.0;// bounds0.min.z;// bounds0.min.z - light_focus_pos.z;
                        let s_x = 2.0 * n / (r - l);
                        let o_x = (r + l) / (r - l);
                        let s_z = 2.0 * n / (t - b);
                        let o_z = (t + b) / (t - b);

                        let s_y = (f + n) / (f - n);
                        let o_y = -2.0 * f * n / (f - n);
                        // y(y₀) = s_y y₀ + o_y
                        //      = ((f + n)y₀ - 2fn) / (f - n)
                        // y(f) = s_y f + o_y
                        //      = ((f + n)f - 2fn) / (f - n)
                        //      = (f² + fn - 2fn) / (f - n)
                        //      = (f² - fn) / (f - n)
                        //      = f(f - n) / (f - n)
                        //      = f
                        //
                        // y(n) = s_y n + o_y
                        //      = ((f + n)n - 2fn) / (f - n)
                        //      = (fn + n² - 2fn) / (f - n)
                        //      = (n² - fn) / (f - n)
                        //      = n(n - f) / (f - n)
                        //      = -n
                        //
                        // x(y₀) = s_x x₀ + o_x y₀
                        //      = (2n x₀ + (r + l) y₀) / (r - l)
                        //      = (2n x₀ + 2ly₀ + (r - l) y₀) / (r - l)
                        //      = 2(n x₀ + l y₀) / (r - l) + y₀
                        //      = (2(n l + l n) + 2(n (x₀ - n) + l (y₀ - l))) / (r - l) + y₀
                        //      = (2(n l + l n) + 2(n (x₀ - n) + l (y₀ - l))) / (r - l) + y₀
                        //
                        //      = 2n(x₀ - l) / (r - l) + 2n l / (r - l) + (r + l) / (r - l)y₀
                        //
                        //      = 2
                        //
                        //      = (2 (x₀ n + l x₀) / (r - l) + y₀
                        //
                        //      = (2n x₀ - (r + l) y₀) / (r - l)
                        //      = (2 (x₀ n - l y₀) - (r - l) y₀) / (r - l)
                        //      = 2 (x₀ n - l y₀) / (r - l) - y₀
                        //
                        //      ~ 2(x₀ n / y₀ - l) / (r - l) - 1
                        //
                        //      = 2 (x₀ (y₀ + n - y₀) - l y₀) / (r - l) - y₀
                        //      = 2 (x₀ - l) y₀ / (r - l) - x₀(y₀ - n) / (r - l) - y₀
                        //
                        // x(n) = 2 (x₀ n - l n) / (r - l) - n
                        //      = n  (2(x₀ - l) / (r - l) - 1)
                        //
                        // x(f) = 2 (x₀ n - l f) / (r - l) - f
                        //      = f (2(x₀ (n / f) - l) / (r - l) - 1)
                        //
                        // x(f) = 2 (x₀ f + l y₀) / (r - l) - f
                        math::Mat4::new(
                            s_x,    o_x,    0.0,    0.0,
                            0.0,    s_y,    0.0,    o_y,
                            0.0,    o_z,    s_z,    0.0,
                            0.0,    1.0,    0.0,    0.0,
                        )/*
                        Mat4::new(
                            n/*1.0*/,      0.0,    0.0,    0.0,
                            0.0,    s_y,    0.0,    o_y,
                            0.0,    0.0,    n,      0.0,
                            0.0,    1.0,    0.0,    0.0,
                        )*/
                    } else {
                        /* Mat4::new(
                            1.0,    0.0,    0.0,    0.0,
                            0.0,    1.0,    0.0,    0.0,
                            0.0,    0.0,    s_y,    o_y,
                            0.0,    0.0,    1.0,    0.0,
                        ) */
                        math::Mat4::identity()
                    }
                    // Mat4::identity()
                    /* let a = (n + f) / (n - f);
                    let b = 2.0 * n * f / (n - f);
                    Mat4::new(
                        n,      0.0,    0.0,    0.0,
                        0.0,    n,      0.0,    0.0,
                        0.0,    0.0,    a,      b,
                        0.0,    0.0,    -1.0,   0.0,
                    ) */
                };
                /* let a = (directed_far + directed_near) / (directed_far - directed_near);
                let b = -2.0 * directed_far * directed_near / (directed_far - directed_near);
                let w_p: Mat4<f32> = Mat4::new(
                    1.0, 0.0, 0.0, 0.0,
                    0.0, a,   0.0, b,
                    0.0, 0.0, 1.0, 0.0,
                    0.0, 1.0, 0.0, 0.0,
                ); */
                let _w_p_arr = w_p.cols.iter().map(|e| (e.x, e.y, e.z, e.w)).collect::<Vec<_>>();
                // println!("mat4 w_p = mat4(vec4{:?}, vec4{:?}, vec4{:?}, vec4{:?});", w_p_arr[0], w_p_arr[1], w_p_arr[2], w_p_arr[3]);
                // let w_p: Mat4<f32> = Mat4::identity();
                // let zmin = p1.z.min(p4.z);
                // let zmax = p1.z.max(p4.z);
                // println!("zmin: {:?}, zmax: {:?}", zmin, zmax);

                // let directed_near = 1.0;
                // let directed_far = /*loaded_distance * 2.0*/(zmax - zmin) * 2.0 + directed_near;

                /* let directed_proj_mat = Mat4::orthographic_rh_no(FrustumPlanes {
                // TODO: Consider adjusting resolution based on view distance.
                    left: xmin,
                    // left: -(directed_shadow_res.x as f32) / 2.0,
                    right: xmax,
                    // right: directed_shadow_res.x as f32 / 2.0,
                    bottom: ymin,
                    // bottom: -(directed_shadow_res.y as f32) / 2.0,
                    top: ymax,
                    // top: directed_shadow_res.y as f32 / 2.0,
                    // TODO: Swap fixed near and far planes for something dependent on the height of the
                    // current scene.
                    near: zmin,//directed_near,
                    far: zmax,//directed_far,
                }); */
                let shadow_all_mat: math::Mat4<f32> = w_p * shadow_view_mat/*w_v * light_all_mat*/;
                let _w_p_arr = shadow_all_mat.cols.iter().map(|e| (e.x, e.y, e.z, e.w)).collect::<Vec<_>>();
                // println!("mat4 shadow_all_mat = mat4(vec4{:?}, vec4{:?}, vec4{:?}, vec4{:?});", w_p_arr[0], w_p_arr[1], w_p_arr[2], w_p_arr[3]);
                let math::Aabb::<f32> { min: math::Vec3 { x: xmin, y: ymin, z: zmin }, max: math::Vec3 { x: xmax, y: ymax, z: zmax } } =
                    math::fit_psr(/*light_all_mat*/shadow_all_mat/*shadow_view_mat*//* * inverse_visible*/, visible_light_volume.iter().copied(), /*|p| math::Vec3::from(p) / p.w*/math::Vec4::homogenized);
                    // fit_psr(light_all_mat/* * inverse_visible*/, aabb_to_points(visible_bounds).iter().copied(), |p| Vec3::from(p) / p.w);
                /* let Aabb { min: Vec3 { z: zmin, .. }, max: Vec3 { z: zmax, .. } } =
                    fit_psr(/*light_all_mat*/shadow_all_mat/* * inverse_visible*/, light_volume.iter().copied(), |p| Vec3::from(p) / p.w);
                    // fit_psr(light_all_mat/* * inverse_visible*/, light_volume.iter().copied(), |p| Vec3::from(p) / p.w);
                    // fit_psr(light_all_mat/* * inverse_visible*/, aabb_to_points(visible_bounds).iter().copied(), |p| Vec3::from(p) / p.w); */
                // println!("xmin: {:?} ymin: {:?} zmin: {:?}, xmax: {:?}, ymax: {:?}, zmax: {:?}", xmin, ymin, zmin, xmax, ymax, zmax);
                let s_x = 2.0 / (xmax - xmin);
                let s_y = 2.0 / (ymax - ymin);
                let s_z = 2.0 / (zmax - zmin);
                /* let o_x = -(s_x * (xmax + xmin)) / 2.0;
                let o_y = -(s_y * (ymax + ymin)) / 2.0;
                let o_z = -(s_z * (zmax + zmin)) / 2.0; */
                let o_x = -(xmax + xmin) / (xmax - xmin);
                let o_y = -(ymax + ymin) / (ymax - ymin);
                let o_z = -(zmax + zmin) / (zmax - zmin);
                let directed_proj_mat = if /*sin_gamma > EPISLON_GAMMA*/factor != -1.0 {
                    // Mat4::identity()
                    Mat4::new(
                        s_x, 0.0, 0.0, o_x,
                        0.0, s_y, 0.0, o_y,
                        0.0, 0.0, /*-*/s_z, /*-*/o_z,
                        0.0, 0.0, 0.0, 1.0,
                    )/*.scaled_3d(Vec3::new(1.0, 1.0, -1.0))*/
                } else {
                    Mat4::new(
                        s_x, 0.0, 0.0, o_x,
                        0.0, s_y, 0.0, o_y,
                        0.0, 0.0, s_z, o_z,
                        0.0, 0.0, 0.0, 1.0,
                    )/*.scaled_3d(Vec3::new(1.0, 1.0, -1.0))*/
                }/*.scaled_3d(Vec3::new(1.0, 1.0, -1.0))*//* * w_p * w_v*//* * l_r*/;//Mat4::identity();
                // println!("proj_mat: {:?}", directed_proj_mat);
                // println!("all_mat: {:?}", directed_proj_mat * view_mat);
                let _w_p_arr = directed_proj_mat.cols.iter().map(|e| (e.x, e.y, e.z, e.w)).collect::<Vec<_>>();
                // println!("mat4 directed_proj_mat = mat4(vec4{:?}, vec4{:?}, vec4{:?}, vec4{:?});", w_p_arr[0], w_p_arr[1], w_p_arr[2], w_p_arr[3]);

                let shadow_all_mat: Mat4<f32> = Mat4::from_col_arrays(shadow_all_mat.into_col_arrays());
                let _w_p_arr = (directed_proj_mat * shadow_all_mat).cols.iter().map(|e| (e.x, e.y, e.z, e.w)).collect::<Vec<_>>();
                // println!("mat4 final_mat = mat4(vec4{:?}, vec4{:?}, vec4{:?}, vec4{:?});", w_p_arr[0], w_p_arr[1], w_p_arr[2], w_p_arr[3]);

                let directed_texture_proj_mat = texture_mat * directed_proj_mat;
                ShadowLocals::new(directed_proj_mat * shadow_all_mat, directed_texture_proj_mat * shadow_all_mat)
            }));
            // Now, we tackle point lights.
            // First, create a perspective projection matrix at 90 degrees (to cover a whole
            // face of the cube map we're using).
            let shadow_proj = Mat4::perspective_rh_no(
                90.0f32.to_radians(),
                point_shadow_aspect,
                SHADOW_NEAR,
                SHADOW_FAR,
            );
            // Next, construct the 6 orientations we'll use for the six faces, in terms of
            // their (forward, up) vectors.
            let orientations = [
                (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)),
                (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)),
                (Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 1.0)),
                (Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 0.0, -1.0)),
                (Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, -1.0, 0.0)),
                (Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, -1.0, 0.0)),
            ];
            // NOTE: We could create the shadow map collection at the same time as the
            // lights, but then we'd have to sort them both, which wastes time.  Plus, we
            // want to prepend our directed lights.
            shadow_mats.extend(lights.iter().flat_map(|light| {
                // Now, construct the full projection matrix by making the light look at each
                // cube face.
                let eye = Vec3::new(light.pos[0], light.pos[1], light.pos[2]) - focus_off;
                orientations.iter().map(move |&(forward, up)| {
                    // NOTE: We don't currently try to linearize point lights or need a separate
                    // transform for them.
                    ShadowLocals::new(
                        shadow_proj * Mat4::look_at_rh(eye, eye + forward, up),
                        Mat4::identity(),
                    )
                })
            }));

            /* shadow_mats.push(
                        Mat4::orthographic_rh_no
            float near_plane = 1.0f, far_plane = 7.5f;
            glm::mat4 lightProjection = glm::ortho(-10.0f, 10.0f, -10.0f, 10.0f, near_plane, far_plane);

                    ); */
            renderer
                .update_consts(&mut self.shadow_mats, &shadow_mats)
                .expect("Failed to update light constants");
            // renderer
            //     .update_shadow_consts(&mut self.shadow_mats, &shadow_mats, 0,
            // 6)     .expect("Failed to update light constants");
        }

        // Remove unused figures.
        self.figure_mgr.clean(scene_data.tick);

        // Maintain audio
        self.sfx_mgr
            .maintain(audio, scene_data.state, scene_data.player_entity);
        self.music_mgr.maintain(audio, scene_data.state);
    }

    /// Render the scene using the provided `Renderer`.
    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        state: &State,
        player_entity: EcsEntity,
        tick: u64,
        scene_data: &SceneData,
    ) {
        let sun_dir = scene_data.get_sun_dir();
        let is_daylight = sun_dir.z < 0.0/*0.6*/;
        let focus_pos = self.camera.get_focus_pos();
        let cam_pos = self.camera.dependents().cam_pos + focus_pos.map(|e| e.trunc());

        // would instead have this as an extension.
        if renderer.render_mode().shadow == render::ShadowMode::Map
            && (is_daylight || self.light_data.len() > 0)
        {
            // Set up shadow mapping.
            renderer.start_shadows();

            // Render terrain shadows.
            self.terrain.render_shadows(
                renderer,
                &self.globals,
                &self.shadow_mats,
                &self.light_data,
                is_daylight,
                focus_pos,
            );

            // Render figure shadows.
            self.figure_mgr.render_shadows(
                renderer,
                state,
                tick,
                &self.globals,
                &self.shadow_mats,
                is_daylight,
                &self.light_data,
                &self.camera,
                scene_data.figure_lod_render_distance,
            );

            // Flush shadows.
            renderer.flush_shadows();
        }
        let lod = self.lod.get_data();

        self.figure_mgr.render_player(
            renderer,
            state,
            player_entity,
            tick,
            &self.globals,
            &self.lights,
            &self.shadows,
            &self.shadow_mats,
            lod,
            &self.camera,
            scene_data.figure_lod_render_distance,
        );

        // Render terrain and figures.
        self.terrain.render(
            renderer,
            &self.globals,
            &self.lights,
            &self.shadows,
            &self.shadow_mats,
            lod,
            focus_pos,
        );

        self.figure_mgr.render(
            renderer,
            state,
            player_entity,
            tick,
            &self.globals,
            &self.lights,
            &self.shadows,
            &self.shadow_mats,
            lod,
            &self.camera,
            scene_data.figure_lod_render_distance,
        );
        self.lod.render(renderer, &self.globals);

        // Render the skybox.
        renderer.render_skybox(
            &self.skybox.model,
            &self.globals,
            &self.skybox.locals,
            &lod.map,
            &lod.horizon,
        );

        self.terrain.render_translucent(
            renderer,
            &self.globals,
            &self.lights,
            &self.shadows,
            &self.shadow_mats,
            lod,
            focus_pos,
            cam_pos,
            scene_data.sprite_render_distance,
        );

        renderer.render_post_process(
            &self.postprocess.model,
            &self.globals,
            &self.postprocess.locals,
        );
    }
}
