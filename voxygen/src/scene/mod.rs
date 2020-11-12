pub mod camera;
pub mod figure;
pub mod lod;
pub mod math;
pub mod particle;
pub mod simple;
pub mod terrain;

pub use self::{
    camera::{Camera, CameraMode},
    figure::FigureMgr,
    lod::Lod,
    particle::ParticleMgr,
    terrain::Terrain,
};
use crate::{
    audio::{music::MusicMgr, sfx::SfxMgr, wind::WindMgr, AudioFrontend},
    render::{
        create_clouds_mesh, create_pp_mesh, create_skybox_mesh, CloudsLocals, CloudsPipeline,
        Consts, GlobalModel, Globals, Light, LodData, Model, PostProcessLocals,
        PostProcessPipeline, Renderer, Shadow, ShadowLocals, SkyboxLocals, SkyboxPipeline,
    },
    settings::Settings,
    window::{AnalogGameInput, Event},
};
use client::Client;
use common::{
    comp,
    outcome::Outcome,
    span,
    state::{DeltaTime, State},
    terrain::{BlockKind, TerrainChunk},
    vol::ReadVol,
};
use comp::item::Reagent;
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
/// The minimum sin γ we will use before switching to uniform mapping.
const EPSILON_UPSILON: f64 = -1.0;

const SHADOW_NEAR: f32 = 0.25; // Near plane for shadow map point light rendering.
const SHADOW_FAR: f32 = 128.0; // Far plane for shadow map point light rendering.

/// Above this speed is considered running
/// Used for first person camera effects
const RUNNING_THRESHOLD: f32 = 0.7;

/// is_daylight, array of active lights.
pub type LightData<'a> = (bool, &'a [Light]);

struct EventLight {
    light: Light,
    timeout: f32,
    fadeout: fn(f32) -> f32,
}

struct Skybox {
    model: Model<SkyboxPipeline>,
    locals: Consts<SkyboxLocals>,
}

struct Clouds {
    model: Model<CloudsPipeline>,
    locals: Consts<CloudsLocals>,
}

struct PostProcess {
    model: Model<PostProcessPipeline>,
    locals: Consts<PostProcessLocals>,
}

pub struct Scene {
    data: GlobalModel,
    camera: Camera,
    camera_input_state: Vec2<f32>,
    event_lights: Vec<EventLight>,

    skybox: Skybox,
    clouds: Clouds,
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

    particle_mgr: ParticleMgr,
    figure_mgr: FigureMgr,
    sfx_mgr: SfxMgr,
    music_mgr: MusicMgr,
    ambient_mgr: WindMgr,
}

pub struct SceneData<'a> {
    pub state: &'a State,
    pub player_entity: specs::Entity,
    pub target_entity: Option<specs::Entity>,
    pub loaded_distance: f32,
    pub view_distance: u32,
    pub tick: u64,
    pub thread_pool: &'a uvth::ThreadPool,
    pub gamma: f32,
    pub ambiance: f32,
    pub mouse_smoothing: bool,
    pub sprite_render_distance: f32,
    pub particles_enabled: bool,
    pub figure_lod_render_distance: f32,
    pub is_aiming: bool,
}

impl<'a> SceneData<'a> {
    pub fn get_sun_dir(&self) -> Vec3<f32> { Globals::get_sun_dir(self.state.get_time_of_day()) }

    pub fn get_moon_dir(&self) -> Vec3<f32> { Globals::get_moon_dir(self.state.get_time_of_day()) }
}

/// Approximate a scalar field of view angle using the parameterization from
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
    theta_x.min(theta_y)
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
///
/// NOTE: Equation's described behavior is *wrong!*  I have pieced together a
/// slightly different function that seems to more closely satisfy the author's
/// intent:
///
/// η =
///     -1                                                        γ < γ_a
///     -1 + (η_b + 1)            (γ - γ_a)/(γ_b - γ_a)     γ_a ≤ γ < γ_b
///     η_b + (η_c - η_b)  sin(90 (γ - γ_b)/(γ_c - γ_b))    γ_b ≤ γ < γ_c
///     η_c                                                 γ_c ≤ γ
///
/// There are other alternatives that may have more desirable properties, such
/// as:
///
/// η =
///     -1                                                        γ < γ_a
///     -1 + (η_b + 1)(1 - cos(90 (γ - γ_a)/(γ_b - γ_a)))   γ_a ≤ γ < γ_b
///     η_b + (η_c - η_b)  sin(90 (γ - γ_b)/(γ_c - γ_b))    γ_b ≤ γ < γ_c
///     η_c                                                 γ_c ≤ γ
fn compute_warping_parameter<F: Float + FloatConst>(
    gamma: F,
    (gamma_a, gamma_b, gamma_c): (F, F, F),
    (eta_b, eta_c): (F, F),
) -> F {
    if gamma < gamma_a {
        -F::one()
        /* F::zero() */
    } else if gamma_a <= gamma && gamma < gamma_b {
        /* -F::one() + (eta_b + F::one()) * (F::one() + (F::FRAC_PI_2() * (gamma - gamma_a) / (gamma_b - gamma_a)).cos()) */
        -F::one() + (eta_b + F::one()) * (F::one() - (F::FRAC_PI_2() * (gamma - gamma_a) / (gamma_b - gamma_a)).cos())
        // -F::one() + (eta_b + F::one()) * ((gamma - gamma_a) / (gamma_b - gamma_a))
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
            data: GlobalModel {
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
            },
            camera: Camera::new(resolution.x / resolution.y, CameraMode::ThirdPerson),
            camera_input_state: Vec2::zero(),
            event_lights: Vec::new(),

            skybox: Skybox {
                model: renderer.create_model(&create_skybox_mesh()).unwrap(),
                locals: renderer.create_consts(&[SkyboxLocals::default()]).unwrap(),
            },
            clouds: Clouds {
                model: renderer.create_model(&create_clouds_mesh()).unwrap(),
                locals: renderer.create_consts(&[CloudsLocals::default()]).unwrap(),
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
            particle_mgr: ParticleMgr::new(renderer),
            figure_mgr: FigureMgr::new(renderer),
            sfx_mgr: SfxMgr::default(),
            music_mgr: MusicMgr::default(),
            ambient_mgr: WindMgr::default(),
        }
    }

    /// Get a reference to the scene's globals.
    pub fn globals(&self) -> &Consts<Globals> { &self.data.globals }

    /// Get a reference to the scene's camera.
    pub fn camera(&self) -> &Camera { &self.camera }

    /// Get a reference to the scene's terrain.
    pub fn terrain(&self) -> &Terrain<TerrainChunk> { &self.terrain }

    /// Get a reference to the scene's lights.
    pub fn lights(&self) -> &Vec<Light> { &self.light_data }

    /// Get a reference to the scene's particle manager.
    pub fn particle_mgr(&self) -> &ParticleMgr { &self.particle_mgr }

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
                // when zooming in the distance the camera travelles should be based on the
                // final distance. This is to make sure the camera travelles the
                // same distance when zooming in and out
                if delta < 0.0 {
                    self.camera.zoom_switch(
                        delta * (0.05 + self.camera.get_distance() * 0.01) / (1.0 - delta * 0.01),
                    ); // Thank you Imbris for doing the math
                } else {
                    self.camera
                        .zoom_switch(delta * (0.05 + self.camera.get_distance() * 0.01));
                }
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

    pub fn handle_outcome(
        &mut self,
        outcome: &Outcome,
        scene_data: &SceneData,
        audio: &mut AudioFrontend,
    ) {
        span!(_guard, "handle_outcome", "Scene::handle_outcome");
        self.particle_mgr.handle_outcome(&outcome, &scene_data);
        self.sfx_mgr.handle_outcome(&outcome, audio);

        match outcome {
            Outcome::Explosion {
                pos,
                power,
                radius: _,
                is_attack: _,
                reagent,
            } => self.event_lights.push(EventLight {
                light: Light::new(
                    *pos,
                    match reagent {
                        Some(Reagent::Blue) => Rgb::new(0.15, 0.4, 1.0),
                        Some(Reagent::Green) => Rgb::new(0.0, 1.0, 0.0),
                        Some(Reagent::Purple) => Rgb::new(0.7, 0.0, 1.0),
                        Some(Reagent::Red) => Rgb::new(1.0, 0.0, 0.0),
                        Some(Reagent::Yellow) => Rgb::new(1.0, 1.0, 0.0),
                        None => {
                            if *power < 0.0 {
                                Rgb::new(0.0, 1.0, 0.0)
                            } else {
                                Rgb::new(1.0, 0.5, 0.0)
                            }
                        },
                    },
                    power.abs()
                        * match reagent {
                            Some(_) => 5.0,
                            None => 2.5,
                        },
                ),
                timeout: match reagent {
                    Some(_) => 1.0,
                    None => 0.5,
                },
                fadeout: |timeout| timeout * 2.0,
            }),
            Outcome::ProjectileShot { .. } => {},
        }
    }

    /// Maintain data such as GPU constant buffers, models, etc. To be called
    /// once per tick.
    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        audio: &mut AudioFrontend,
        scene_data: &SceneData,
        client: &Client,
    ) {
        span!(_guard, "maintain", "Scene::maintain");
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

        let (player_height, player_eye_height) = scene_data
            .state
            .ecs()
            .read_storage::<comp::Body>()
            .get(scene_data.player_entity)
            .map_or((1.0, 0.0), |b| (b.height(), b.eye_height()));

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
                    player_height * 0.42
                } else if is_running && on_ground.unwrap_or(false) {
                    player_eye_height + (scene_data.state.get_time() as f32 * 17.0).sin() * 0.05
                } else {
                    player_eye_height
                }
            },
            CameraMode::ThirdPerson if scene_data.is_aiming => player_height * 1.16,
            CameraMode::ThirdPerson => player_eye_height,
            CameraMode::Freefly => 0.0,
        };

        match self.camera.get_mode() {
            CameraMode::FirstPerson | CameraMode::ThirdPerson => {
                self.camera.set_focus_pos(
                    player_pos + Vec3::unit_z() * (up - tilt.min(0.0).sin() * dist * 0.6),
                );
            },
            CameraMode::Freefly => {},
        };

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
            view_mat_inv,
            proj_mat,
            proj_mat_inv,
            cam_pos,
            ..
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
                .filter(|(pos, _, _, light_anim)| {
                    light_anim.col != Rgb::zero()
                        && light_anim.strength > 0.0
                        && (pos.0.distance_squared(player_pos) as f32)
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
                })
                .chain(
                    self.event_lights
                        .iter()
                        .map(|el| el.light.with_strength((el.fadeout)(el.timeout))),
                ),
        );
        lights.sort_by_key(|light| light.get_pos().distance_squared(player_pos) as i32);
        lights.truncate(MAX_LIGHT_COUNT);
        renderer
            .update_consts(&mut self.data.lights, &lights)
            .expect("Failed to update light constants");

        // Update event lights
        let dt = ecs.fetch::<DeltaTime>().0;
        self.event_lights.drain_filter(|el| {
            el.timeout -= dt;
            el.timeout <= 0.0
        });

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
            &scene_data.state.ecs().read_storage::<comp::Health>(),
        )
            .join()
            .filter(|(_, _, _, _, health)| !health.is_dead)
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
            .update_consts(&mut self.data.shadows, &shadows)
            .expect("Failed to update light constants");

        // Remember to put the new loaded distance back in the scene.
        self.loaded_distance = loaded_distance;

        // Update light projection matrices for the shadow map.
        let time_of_day = scene_data.state.get_time_of_day();
        let focus_pos = self.camera.get_focus_pos();
        let focus_off = focus_pos.map(|e| e.trunc());

        // Update global constants.
        renderer
            .update_consts(&mut self.data.globals, &[Globals::new(
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
                scene_data.ambiance,
                self.camera.get_mode(),
                scene_data.sprite_render_distance as f32 - 20.0,
            )])
            .expect("Failed to update global constants");
        renderer
            .update_consts(&mut self.clouds.locals, &[CloudsLocals::new(
                proj_mat_inv,
                view_mat_inv,
            )])
            .expect("Failed to update cloud locals");
        renderer
            .update_consts(&mut self.postprocess.locals, &[PostProcessLocals::new(
                proj_mat_inv,
                view_mat_inv,
            )])
            .expect("Failed to update post-process locals");

        // Maintain LoD.
        self.lod.maintain(renderer);

        // Maintain the terrain.
        let (_visible_bounds, visible_light_volume, visible_psr_bounds) = self.terrain.maintain(
            renderer,
            &scene_data,
            focus_pos,
            self.loaded_distance,
            view_mat,
            proj_mat,
        );

        // Maintain the figures.
        let _figure_bounds =
            self.figure_mgr
                .maintain(renderer, scene_data, visible_psr_bounds, &self.camera);

        let sun_dir = scene_data.get_sun_dir();
        let is_daylight = sun_dir.z < 0.0;
        if renderer.render_mode().shadow.is_map() && (is_daylight || !lights.is_empty()) {
            let fov = self.camera.get_fov();
            let aspect_ratio = self.camera.get_aspect_ratio();

            let view_dir = ((focus_pos.map(f32::fract)) - cam_pos).normalized();
            let (point_shadow_res, _directed_shadow_res) = renderer.get_shadow_resolution();
            // NOTE: The aspect ratio is currently always 1 for our cube maps, since they
            // are equal on all sides.
            let point_shadow_aspect = point_shadow_res.x as f32 / point_shadow_res.y as f32;
            // Construct matrices to transform from world space to light space for the sun
            // and moon.
            let directed_light_dir = math::Vec3::from(sun_dir);

            // Optimal warping for directed lights:
            //
            // n_opt = 1 / sin y (z_n + √(z_n + (f - n) sin y))
            //
            // where n is near plane, f is far plane, y is the tilt angle between view and
            // light direction, and n_opt is the optimal near plane.
            // We also want a way to transform and scale this matrix (* 0.5 + 0.5) in order
            // to transform it correctly into texture coordinates, as well as
            // OpenGL coordinates.  Note that the matrix for directional light
            // is *already* linear in the depth buffer.
            let texture_mat = Mat4::scaling_3d(0.5f32) * Mat4::translation_3d(1.0f32);
            // We need to compute these offset matrices to transform world space coordinates
            // to the translated ones we use when multiplying by the light space
            // matrix; this helps avoid precision loss during the
            // multiplication.
            let look_at = math::Vec3::from(cam_pos);
            // We upload view matrices as well, to assist in linearizing vertex positions.
            // (only for directional lights, so far).
            let mut directed_shadow_mats = Vec::with_capacity(6);
            let new_dir = math::Vec3::from(view_dir);
            let new_dir = new_dir.normalized();
            let up: math::Vec3<f32> = math::Vec3::up();
            directed_shadow_mats.push(math::Mat4::look_at_rh(
                look_at,
                look_at + directed_light_dir,
                up,
            ));
            // This leaves us with five dummy slots, which we push as defaults.
            directed_shadow_mats
                .extend_from_slice(&[math::Mat4::default(); 6 - NUM_DIRECTED_LIGHTS] as _);
            // Now, construct the full projection matrices in the first two directed light
            // slots.
            let mut shadow_mats = Vec::with_capacity(6 * (lights.len() + 1));
            shadow_mats.extend(directed_shadow_mats.iter().enumerate().map(
                move |(idx, &light_view_mat)| {
                    if idx >= NUM_DIRECTED_LIGHTS {
                        return ShadowLocals::new(Mat4::identity(), Mat4::identity());
                    }

                    let v_p_orig =
                        math::Vec3::from(light_view_mat * math::Vec4::from_direction(new_dir));
                    let mut v_p = v_p_orig.normalized();
                    let cos_gamma = new_dir
                        .map(f64::from)
                        .dot(directed_light_dir.map(f64::from));
                    let sin_gamma = (1.0 - cos_gamma * cos_gamma).sqrt();
                    let gamma = sin_gamma.asin();
                    let view_mat = math::Mat4::from_col_array(view_mat.into_col_array());
                    let bounds1 = math::fit_psr(
                        view_mat.map_cols(math::Vec4::from),
                        visible_light_volume.iter().copied(),
                        math::Vec4::homogenized,
                    );
                    let n_e = f64::from(-bounds1.max.z);
                    let factor = compute_warping_parameter_perspective(
                        gamma,
                        n_e,
                        f64::from(fov),
                        f64::from(aspect_ratio),
                    );

                    v_p.z = 0.0;
                    v_p.normalize();
                    let l_r: math::Mat4<f32> = if factor > EPSILON_UPSILON {
                        math::Mat4::look_at_rh(math::Vec3::zero(), math::Vec3::forward_rh(), v_p)
                    } else {
                        math::Mat4::identity()
                    };
                    let directed_proj_mat = math::Mat4::new(
                        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0,
                        1.0,
                    );

                    let light_all_mat = l_r * directed_proj_mat * light_view_mat;
                    let bounds0 = math::fit_psr(
                        light_all_mat,
                        visible_light_volume.iter().copied(),
                        math::Vec4::homogenized,
                    );
                    // Vague idea: project z_n from the camera view to the light view (where it's
                    // tilted by γ).
                    let (z_0, z_1) = {
                        let p_z = bounds1.max.z;
                        let p_y = bounds0.min.y;
                        let p_x = bounds0.center().x;
                        let view_inv = view_mat.inverted();
                        let light_all_inv = light_all_mat.inverted();

                        let view_point = view_inv * math::Vec4::new(0.0, 0.0, p_z, 1.0);
                        let view_plane =
                            view_inv * math::Vec4::from_direction(math::Vec3::unit_z());

                        let light_point = light_all_inv * math::Vec4::new(0.0, p_y, 0.0, 1.0);
                        let light_plane =
                            light_all_inv * math::Vec4::from_direction(math::Vec3::unit_y());

                        let shadow_point = light_all_inv * math::Vec4::new(p_x, 0.0, 0.0, 1.0);
                        let shadow_plane =
                            light_all_inv * math::Vec4::from_direction(math::Vec3::unit_x());

                        let solve_p0 = math::Mat4::new(
                            view_plane.x,
                            view_plane.y,
                            view_plane.z,
                            -view_plane.dot(view_point),
                            light_plane.x,
                            light_plane.y,
                            light_plane.z,
                            -light_plane.dot(light_point),
                            shadow_plane.x,
                            shadow_plane.y,
                            shadow_plane.z,
                            -shadow_plane.dot(shadow_point),
                            0.0,
                            0.0,
                            0.0,
                            1.0,
                        );

                        let p0_world = solve_p0.inverted() * math::Vec4::unit_w();
                        let p0 = light_all_mat * p0_world;
                        let mut p1 = p0;
                        p1.y = bounds0.max.y;

                        let view_from_light_mat = view_mat * light_all_inv;
                        let z0 = view_from_light_mat * p0;
                        let z1 = view_from_light_mat * p1;

                        (f64::from(z0.z), f64::from(z1.z))
                    };

                    let mut light_focus_pos: math::Vec3<f32> = math::Vec3::zero();
                    light_focus_pos.x = bounds0.center().x;
                    light_focus_pos.y = bounds0.min.y;
                    light_focus_pos.z = bounds0.center().z;

                    let d = f64::from(bounds0.max.y - bounds0.min.y).abs();

                    let w_l_y = d;

                    // NOTE: See section 5.1.2.2 of Lloyd's thesis.
                    let alpha = z_1 / z_0;
                    let alpha_sqrt = alpha.sqrt();
                    let directed_near_normal = if factor < 0.0 {
                        // Standard shadow map to LiSPSM
                        (1.0 + alpha_sqrt - factor * (alpha - 1.0))
                            / ((alpha - 1.0) * (factor + 1.0))
                    } else {
                        // LiSPSM to PSM
                        ((alpha_sqrt - 1.0) * (factor * alpha_sqrt + 1.0)).recip()
                    };

                    // Equation 5.14 - 5.16
                    let y_ = |v: f64| w_l_y * (v + directed_near_normal).abs();
                    let directed_near = y_(0.0) as f32;
                    let directed_far = y_(1.0) as f32;
                    light_focus_pos.y = if factor > EPSILON_UPSILON {
                        light_focus_pos.y - directed_near
                    } else {
                        light_focus_pos.y
                    };
                    let w_v: math::Mat4<f32> = math::Mat4::translation_3d(-math::Vec3::new(
                        light_focus_pos.x,
                        light_focus_pos.y,
                        light_focus_pos.z,
                    ));
                    let shadow_view_mat: math::Mat4<f32> = w_v * light_all_mat;
                    let w_p: math::Mat4<f32> = {
                        if factor > EPSILON_UPSILON {
                            // Projection for y
                            let near = directed_near;
                            let far = directed_far;
                            let left = -1.0;
                            let right = 1.0;
                            let bottom = -1.0;
                            let top = 1.0;
                            let s_x = 2.0 * near / (right - left);
                            let o_x = (right + left) / (right - left);
                            let s_z = 2.0 * near / (top - bottom);
                            let o_z = (top + bottom) / (top - bottom);

                            let s_y = (far + near) / (far - near);
                            let o_y = -2.0 * far * near / (far - near);

                            math::Mat4::new(
                                s_x, o_x, 0.0, 0.0, 0.0, s_y, 0.0, o_y, 0.0, o_z, s_z, 0.0, 0.0,
                                1.0, 0.0, 0.0,
                            )
                        } else {
                            math::Mat4::identity()
                        }
                    };

                    let shadow_all_mat: math::Mat4<f32> = w_p * shadow_view_mat;
                    let math::Aabb::<f32> {
                        min:
                            math::Vec3 {
                                x: xmin,
                                y: ymin,
                                z: zmin,
                            },
                        max:
                            math::Vec3 {
                                x: xmax,
                                y: ymax,
                                z: zmax,
                            },
                    } = math::fit_psr(
                        shadow_all_mat,
                        visible_light_volume.iter().copied(),
                        math::Vec4::homogenized,
                    );
                    let s_x = 2.0 / (xmax - xmin);
                    let s_y = 2.0 / (ymax - ymin);
                    let s_z = 2.0 / (zmax - zmin);
                    let o_x = -(xmax + xmin) / (xmax - xmin);
                    let o_y = -(ymax + ymin) / (ymax - ymin);
                    let o_z = -(zmax + zmin) / (zmax - zmin);
                    let directed_proj_mat = Mat4::new(
                        s_x, 0.0, 0.0, o_x, 0.0, s_y, 0.0, o_y, 0.0, 0.0, s_z, o_z, 0.0, 0.0, 0.0,
                        1.0,
                    );

                    let shadow_all_mat: Mat4<f32> =
                        Mat4::from_col_arrays(shadow_all_mat.into_col_arrays());

                    let directed_texture_proj_mat = texture_mat * directed_proj_mat;
                    ShadowLocals::new(
                        directed_proj_mat * shadow_all_mat,
                        directed_texture_proj_mat * shadow_all_mat,
                    )
                },
            ));
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

            renderer
                .update_consts(&mut self.data.shadow_mats, &shadow_mats)
                .expect("Failed to update light constants");
        }

        // Remove unused figures.
        self.figure_mgr.clean(scene_data.tick);

        // Maintain the particles.
        self.particle_mgr
            .maintain(renderer, &scene_data, &self.terrain);

        // Maintain audio
        self.sfx_mgr.maintain(
            audio,
            scene_data.state,
            scene_data.player_entity,
            &self.camera,
            &self.terrain,
            client,
        );
        self.music_mgr.maintain(audio, scene_data.state, client);
        self.ambient_mgr
            .maintain(audio, scene_data.state, client, &self.camera);
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
        span!(_guard, "render", "Scene::render");
        let sun_dir = scene_data.get_sun_dir();
        let is_daylight = sun_dir.z < 0.0;
        let focus_pos = self.camera.get_focus_pos();
        let cam_pos = self.camera.dependents().cam_pos + focus_pos.map(|e| e.trunc());

        let global = &self.data;
        let light_data = (is_daylight, &*self.light_data);
        let camera_data = (&self.camera, scene_data.figure_lod_render_distance);

        // would instead have this as an extension.
        if renderer.render_mode().shadow.is_map() && (is_daylight || !light_data.1.is_empty()) {
            if is_daylight {
                // Set up shadow mapping.
                renderer.start_shadows();
            }

            // Render terrain shadows.
            self.terrain
                .render_shadows(renderer, global, light_data, focus_pos);

            // Render figure shadows.
            self.figure_mgr
                .render_shadows(renderer, state, tick, global, light_data, camera_data);

            if is_daylight {
                // Flush shadows.
                renderer.flush_shadows();
            }
        }
        let lod = self.lod.get_data();

        self.figure_mgr.render_player(
            renderer,
            state,
            player_entity,
            tick,
            global,
            lod,
            camera_data,
        );

        // Render terrain and figures.
        self.terrain.render(renderer, global, lod, focus_pos);

        self.figure_mgr.render(
            renderer,
            state,
            player_entity,
            tick,
            global,
            lod,
            camera_data,
        );
        self.lod.render(renderer, global);

        // Render the skybox.
        renderer.render_skybox(&self.skybox.model, global, &self.skybox.locals, lod);

        self.terrain.render_translucent(
            renderer,
            global,
            lod,
            focus_pos,
            cam_pos,
            scene_data.sprite_render_distance,
        );

        // Render particle effects.
        self.particle_mgr.render(renderer, scene_data, global, lod);

        // Render clouds (a post-processing effect)
        renderer.render_clouds(
            &self.clouds.model,
            &global.globals,
            &self.clouds.locals,
            self.lod.get_data(),
        );

        renderer.render_post_process(
            &self.postprocess.model,
            &global.globals,
            &self.postprocess.locals,
            self.lod.get_data(),
        );
    }
}
