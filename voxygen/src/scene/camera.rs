use common::{terrain::TerrainGrid, vol::ReadVol};
use common_base::span;
use core::{f32::consts::PI, fmt::Debug, ops::Range};
use num::traits::{real::Real, FloatConst};
use treeculler::Frustum;
use vek::*;

pub const NEAR_PLANE: f32 = 0.0625;
pub const FAR_PLANE: f32 = 524288.06; // excessive precision: 524288.0625

const FIRST_PERSON_INTERP_TIME: f32 = 0.1;
const THIRD_PERSON_INTERP_TIME: f32 = 0.1;
const FREEFLY_INTERP_TIME: f32 = 0.0;
const LERP_ORI_RATE: f32 = 15.0;
const CLIPPING_MODE_RANGE: Range<f32> = 2.0..20.0;
pub const MIN_ZOOM: f32 = 0.1;

// Possible TODO: Add more modes
#[derive(PartialEq, Debug, Clone, Copy, Eq, Hash)]
pub enum CameraMode {
    FirstPerson = 0,
    ThirdPerson = 1,
    Freefly = 2,
}

impl Default for CameraMode {
    fn default() -> Self { Self::ThirdPerson }
}

#[derive(Clone, Copy)]
pub struct Dependents {
    pub view_mat: Mat4<f32>,
    pub view_mat_inv: Mat4<f32>,
    pub proj_mat: Mat4<f32>,
    pub proj_mat_inv: Mat4<f32>,
    /// Specifically there for satisfying our treeculler dependency, which can't
    /// handle inverted depth planes.
    pub proj_mat_treeculler: Mat4<f32>,
    pub cam_pos: Vec3<f32>,
    pub cam_dir: Vec3<f32>,
}

pub struct Camera {
    tgt_focus: Vec3<f32>,
    focus: Vec3<f32>,
    tgt_ori: Vec3<f32>,
    ori: Vec3<f32>,
    tgt_dist: f32,
    dist: f32,
    tgt_fov: f32,
    fov: f32,
    tgt_fixate: f32,
    fixate: f32,
    aspect: f32,
    mode: CameraMode,

    last_time: Option<f64>,

    dependents: Dependents,
    frustum: Frustum<f32>,
}

fn clamp_and_modulate(ori: Vec3<f32>) -> Vec3<f32> {
    Vec3 {
        // Wrap camera yaw
        x: ori.x.rem_euclid(2.0 * PI),
        // Clamp camera pitch to the vertical limits
        y: ori.y.clamp(-PI / 2.0 + 0.0001, PI / 2.0 - 0.0001),
        // Wrap camera roll
        z: ori.z.rem_euclid(2.0 * PI),
    }
}

/// Generalized method to construct a perspective projection with x ∈ [-1,1], y
/// ∈ [-1,1], z ∈ [0,1] given fov_y_radians, aspect_ratio, 1/n, and 1/f.  Note
/// that you pass in *1/n* and *1/f*, not n and f like you normally would for a
/// perspective projection; this is done to enable uniform handling of both
/// finite and infinite far planes.
///
/// The only requirements on n and f are: 1/n ≠ 1/f, and 0 ≤ 1/n * 1/f.
///
/// This ensures that the near and far plane are not identical (or else your
/// projection would not cover any distance), and that they have the same sign
/// (or else we cannot rely on clipping to properly fix your scene).  This also
/// ensures that at least one of 1/n and 1/f is not 0, and by construction it
/// guarantees that neither n nor f are 0; these are required in order to make
/// sense of the definition of near and far planes, and avoid collapsing all
/// depths to a single point.
///
/// For "typical" projections (matching perspective_lh_no), you would satisfy
/// the stronger requirements.  We give the typical conditions for each bullet
/// point, and then explain the consequences of not satisfying these conditions:
///
/// * 1/n < 1/f (0 to 1 depth planes, meaning n = near and f = far; if f < n,
///   depth planes go from 1 to 0, meaning f = near and n = far, aka "reverse
///   depth").
///
///     This is by far the most
///     likely thing to want to change; inverted depth coordinates have *far*
/// better accuracy for     DirectX / Metal / WGPU-like APIs, when using
/// floating point depth, while not being *worse*     than the alternative
/// (OpenGL-like depth, or when using fixed-point / integer depth).  For
///     maximum benefit, make sure you are using Depth32F, as on most platforms
/// this is the only     depth buffer size where floating point can be used.
///
///     It is a bit unintuitive to prove this, but it turns out that when using
/// 1 to 0 depth planes,     the point where the depth buffer has its worst
/// precision is not at the far plane (as with 0     to 1 depth planes) nor at
/// the near plane, as you might expect, but exactly at far/2 (the
///     near plane setting does not affect the point of minimum accuracy at
/// all!).  However, don't     let this fool you into believing the point of
/// worst precision has simply been moved     around--for *any* fixed Δz that is
/// the minimum amount of depth precision you want over the     whole range, and
/// any near plane, you can set the far plane farther (generally much much
///     farther!) with reversed clip space than you can with standard clip space
/// while still     getting at least that much depth precision in the worst
/// case.  Nor is this a small     worst-case; for many desirable near and far
/// plane combinations, more than half the visible     space will have
/// completely unusable precision under 0 to 1 depth, while having much better
///     than needed precision under 1 to 0 depth.
///
///     To compute the exact (at least "roughly exact") worst-case accuracy for
/// floating     point depth and a given precision target Δz, for reverse clip
/// planes (this can be computed     for the non-reversed case too, but it's
/// painful and the values are horrible, so don't     bother), we compute
/// (assuming a finite far plane--see below for details on the infinite
///     case) the change in the integer representation of the mantissa at z=n/2:
///
///     ```ignore
///     e = floor(ln(near/(far - near))/ln(2))
///     db/dz = 2^(2-e) / ((1 / far - 1 / near) * (far)^2)
///     ```
///
///     Then the maximum precision you can safely use to get a change in the
/// integer representation     of the mantissa (assuming 32-bit floating points)
/// is around:
///
///     ```ignore
///     abs(2^(-23) / (db/dz)).
///     ```
///
///     In particular, if your worst-case target accuracy over the depth range
/// is Δz, you should     be okay if:
///
///     ```ignore
///     abs(Δz * (db/dz)) * 2^(23) ≥ 1.
///     ```
///
///     This only accounts for precision of the final floating-point value, so
/// it's     possible that artifacts may be introduced elsewhere during the
/// computation that reduce     precision further; the most famous example of
/// this is that OpenGL wipes out most of the     precision gains by going from
/// [-1,1] to [0,1] by letting
///
///     ```ignore
///     clip space depth = depth * 0.5 + 0.5
///     ```
///
///     which results in huge precision errors by removing nearly all the
/// floating point values     with the most precision (those close to 0).
/// Fortunately, most such artifacts are absent     under the wgpu/DirectX/Metal
/// depth clip space model, so with any luck remaining depth     errors due to
/// the perspective warp itself should be minimal.
///
/// * 0 ≠ 1/far (finite far plane).  When this is false, the far plane is at
///   infinity; this removes the restriction of having a far plane at all, often
///   with minimal reduction in accuracy for most values in the scene.  In fact,
///   in almost all cases with non-reversed depth planes, it *improves* accuracy
///   over the finite case for the vast majority of the range; however, you
///   should be using reversed depth planes, and if you are then there is a
///   quite natural accuracy vs. distance tradeoff in the infinite case.
///
///     When using an infinite far plane, the worst-case accuracy is *always* at
/// infinity, and gets     progressively worse as you get farther away from the
/// near plane.  However, there is a     second advantage that may not be
/// immediately apparent: the perspective warp becomes much     simpler,
/// potentially removing artifacts!  Specifically, in the 0 to 1 depth plane
/// case, the     assigned depth value (after perspective division) becomes:
///
///     ```ignore
///     depth = 1 - near/z
///     ```
///
///     while in the 1 to 0 depth plane case (which you should be using), the
/// equation is even     simpler:
///
///     ```ignore
///     depth = near/z
///     ```
///
///     In the 1 to 0 case, in particular, you can see that the depth value is
/// *linear in z in     log space.*  This lets us compute, for any given target
/// precision, a *very* simple     worst-case upper bound on the maximum
/// absolute z value for which that precision can     be achieved (the upper
/// bound is tight in some cases, but in others may be conservative):
///
///     ```ignore
///     db/dz ≥ 1/z
///     ```
///
///     Plugging that into our old formula, we find that we attain the required
/// precision at least     in the range (again, this is for the 1 to 0 infinite
/// case only!):
///
///     ```ignore
///     abs(z) ≤ Δz * 2^23
///     ```
///
///     One thing you may notice is that this worst-case bound *does not depend
/// on the near plane.*     This means that (within reason) you can put the near
/// plane as close as you like and still     attain this bound.  Of course, the
/// bound is not completely tight, but it should not be off     by more than a
/// factor of 2 or so (informally proven, not made rigorous yet), so for most
///     practical purposes you can set the near plane as low as you like in this
/// case.
///
/// * 0 < 1/near (positive near plane--best used when moving *to* left-handed
///   spaces, as we normally do in OpenGL and DirectX).  A use case for *not*
///   doing this is that it allows moving *from* a left-handed space *to* a
///   right-handed space in WGPU / DirectX / Metal coordinates; this means that
///   if matrices were already set up for OpenGL using functions like look_at_rh
///   that assume right-handed coordinates, we can simply switch these to
///   look_at_lh and use a right-handed perspective projection with a negative
///   near plane, to get correct rendering behavior.  Details are out of scope
///   for this comment.
///
/// Note that there is one final, very important thing that affects possible
/// precision--the actual underlying precision of the floating point format at a
/// particular value!  As your z values go up, their precision will shrink, so
/// if at all possible try to shrink your z values down to the lowest range in
/// which they can be.  Unfortunately, this cannot be part of the perspective
/// projection itself, because by the time z gets to the projection it is
/// usually too late for values to still be integers (or coarse-grained powers
/// of 2).  Instead, try to scale down x, y, and z as soon as possible before
/// submitting them to the GPU, ideally by as large as possible of a power of 2
/// that works for your use case.  Not only will this improve depth precision
/// and recall, it will also help address other artifacts caused by values far
/// from z (such as improperly rounded rotations, or improper line equations due
/// to greedy meshing).
///
/// TODO: Consider passing fractions rather than 1/n and 1/f directly, even
/// though the logic for why it should be okay to pass them directly is probably
/// sound (they are both valid z values in the range, so gl_FragCoord.w will be
/// assigned to this, meaning if they are imprecise enough then the whole
/// calculation will be similarly imprecise).
///
/// TODO: Since it's a bit confusing that n and f are not always near and far,
/// and a negative near plane can (probably) be emulated with simple actions on
/// the perspective matrix, consider removing this functionality and replacing
/// our assertion with a single condition: `(1/far) * (1/near) < (1/near)²`.
pub fn perspective_lh_zo_general<T>(
    fov_y_radians: T,
    aspect_ratio: T,
    inv_n: T,
    inv_f: T,
) -> Mat4<T>
where
    T: Real + FloatConst + Debug,
{
    // Per comments, we only need these two assertions to make sure our calculations
    // make sense.
    debug_assert_ne!(
        inv_n, inv_f,
        "The near and far plane distances cannot be equal, found: {:?} = {:?}",
        inv_n, inv_f
    );
    debug_assert!(
        T::zero() <= inv_n * inv_f,
        "The near and far plane distances must have the same sign, found: {:?} * {:?} < 0",
        inv_n,
        inv_f
    );

    // TODO: Would be nice to separate out the aspect ratio computations.
    let two = T::one() + T::one();
    let tan_half_fovy = (fov_y_radians / two).tan();
    let m00 = T::one() / (aspect_ratio * tan_half_fovy);
    let m11 = T::one() / tan_half_fovy;
    let m23 = -T::one() / (inv_n - inv_f);
    let m22 = inv_n * (-m23);
    Mat4::new(
        m00,
        T::zero(),
        T::zero(),
        T::zero(),
        T::zero(),
        m11,
        T::zero(),
        T::zero(),
        T::zero(),
        T::zero(),
        m22,
        m23,
        T::zero(),
        T::zero(),
        T::one(),
        T::zero(),
    )
}

/// Same as perspective_lh_zo_general, but for right-handed source spaces.
pub fn perspective_rh_zo_general<T>(
    fov_y_radians: T,
    aspect_ratio: T,
    inv_n: T,
    inv_f: T,
) -> Mat4<T>
where
    T: Real + FloatConst + Debug,
{
    let mut m = perspective_lh_zo_general(fov_y_radians, aspect_ratio, inv_n, inv_f);
    m[(2, 2)] = -m[(2, 2)];
    m[(3, 2)] = -m[(3, 2)];
    m
}

impl Camera {
    /// Create a new `Camera` with default parameters.
    pub fn new(aspect: f32, mode: CameraMode) -> Self {
        // Make sure aspect is valid
        let aspect = if aspect.is_normal() { aspect } else { 1.0 };

        let dist = match mode {
            CameraMode::ThirdPerson => 10.0,
            CameraMode::FirstPerson | CameraMode::Freefly => MIN_ZOOM,
        };

        Self {
            tgt_focus: Vec3::unit_z() * 10.0,
            focus: Vec3::unit_z() * 10.0,
            tgt_ori: Vec3::zero(),
            ori: Vec3::zero(),
            tgt_dist: dist,
            dist,
            tgt_fov: 1.1,
            fov: 1.1,
            tgt_fixate: 1.0,
            fixate: 1.0,
            aspect,
            mode,

            last_time: None,

            dependents: Dependents {
                view_mat: Mat4::identity(),
                view_mat_inv: Mat4::identity(),
                proj_mat: Mat4::identity(),
                proj_mat_inv: Mat4::identity(),
                proj_mat_treeculler: Mat4::identity(),
                cam_pos: Vec3::zero(),
                cam_dir: Vec3::unit_y(),
            },
            frustum: Frustum::from_modelview_projection(Mat4::identity().into_col_arrays()),
        }
    }

    /// Compute the transformation matrices (view matrix and projection matrix)
    /// and position of the camera.
    pub fn compute_dependents(&mut self, terrain: &TerrainGrid) {
        self.compute_dependents_full(terrain, |block| block.is_opaque())
    }

    /// The is_fluid argument should return true for transparent voxels.
    pub fn compute_dependents_full<V: ReadVol>(
        &mut self,
        terrain: &V,
        is_transparent: fn(&V::Vox) -> bool,
    ) {
        span!(_guard, "compute_dependents", "Camera::compute_dependents");
        // TODO: More intelligent function to decide on which strategy to use
        if self.tgt_dist < CLIPPING_MODE_RANGE.end {
            self.compute_dependents_near(terrain, is_transparent)
        } else {
            self.compute_dependents_far(terrain, is_transparent)
        }
    }

    fn compute_dependents_near<V: ReadVol>(
        &mut self,
        terrain: &V,
        is_transparent: fn(&V::Vox) -> bool,
    ) {
        const FRUSTUM_PADDING: [Vec3<f32>; 4] = [
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        // Calculate new frustum location as there may have been lerp towards tgt_dist
        // Without this, there will be camera jumping back and forth in some scenarios
        // TODO: Optimize and fix clipping still happening if self.dist << self.tgt_dist

        // Use tgt_dist, as otherwise we end up in loop due to dist depending on frustum
        // and vice versa
        let local_dependents = self.compute_dependents_helper(self.tgt_dist);
        let frustum = self.compute_frustum(&local_dependents);
        let dist = {
            frustum
                .points
                .iter()
                .take(4)
                .zip(FRUSTUM_PADDING.iter())
                .map(|(pos, padding)| {
                    let fwd = self.forward();
                    // TODO: undo once treeculler is vek15.7
                    let transformed = Vec3::new(pos.x, pos.y, pos.z);
                    transformed + 0.6 * (fwd.cross(*padding) + fwd.cross(*padding).cross(fwd))
                })
                .chain([(self.focus - self.forward() * (self.dist + 0.5))])  // Padding to behind
                .map(|pos| {
                    match terrain
                        .ray(self.focus, pos)
                        .ignore_error()
                        .max_iter(500)
                        .until(is_transparent)
                        .cast()
                    {
                        (d, Ok(Some(_))) => f32::min(d, self.tgt_dist),
                        (_, Ok(None)) => self.dist,
                        (_, Err(_)) => self.dist,
                    }
                    .max(0.0)
                })
                .reduce(f32::min)
                .unwrap_or(0.0)
        };

        // If the camera ends up being too close to the focus point, switch policies.
        if dist < CLIPPING_MODE_RANGE.start {
            self.compute_dependents_far(terrain, is_transparent);
        } else {
            if self.dist >= dist {
                self.dist = dist;
            }

            // Recompute only if needed
            if (dist - self.tgt_dist).abs() > f32::EPSILON {
                let dependents = self.compute_dependents_helper(dist);
                self.frustum = self.compute_frustum(&dependents);
                self.dependents = dependents;
            } else {
                self.dependents = local_dependents;
                self.frustum = frustum;
            }
        }
    }

    fn compute_dependents_far<V: ReadVol>(
        &mut self,
        terrain: &V,
        is_transparent: fn(&V::Vox) -> bool,
    ) {
        let dist = {
            let (start, end) = (self.focus - self.forward() * self.dist, self.focus);

            match terrain
                .ray(start, end)
                .ignore_error()
                .max_iter(500)
                .until(|b| !is_transparent(b))
                .cast()
            {
                (d, Ok(Some(_))) => f32::min(self.dist - d - 0.03, self.dist),
                (_, Ok(None)) => self.dist,
                (_, Err(_)) => self.dist,
            }
            .max(0.0)
        };

        let dependents = self.compute_dependents_helper(dist);
        self.frustum = self.compute_frustum(&dependents);
        self.dependents = dependents;
    }

    fn compute_dependents_helper(&self, dist: f32) -> Dependents {
        let view_mat = Mat4::<f32>::identity()
            * Mat4::translation_3d(-Vec3::unit_z() * dist)
            * Mat4::rotation_z(self.ori.z)
            * Mat4::rotation_x(self.ori.y)
            * Mat4::rotation_y(self.ori.x)
            * Mat4::rotation_3d(PI / 2.0, -Vec4::unit_x())
            * Mat4::translation_3d(-self.focus.map(|e| e.fract()));
        let view_mat_inv: Mat4<f32> = view_mat.inverted();

        let fov = self.get_effective_fov();
        // NOTE: We reverse the far and near planes to produce an inverted depth
        // buffer (1 to 0 z planes).
        let proj_mat =
            perspective_rh_zo_general(fov, self.aspect, 1.0 / FAR_PLANE, 1.0 / NEAR_PLANE);
        // For treeculler, we also produce a version without inverted depth.
        let proj_mat_treeculler =
            perspective_rh_zo_general(fov, self.aspect, 1.0 / NEAR_PLANE, 1.0 / FAR_PLANE);

        Dependents {
            view_mat,
            view_mat_inv,
            proj_mat,
            proj_mat_inv: proj_mat.inverted(),
            proj_mat_treeculler,
            cam_pos: Vec3::from(view_mat_inv * Vec4::unit_w()),
            cam_dir: Vec3::from(view_mat_inv * -Vec4::unit_z()),
        }
    }

    fn compute_frustum(&mut self, dependents: &Dependents) -> Frustum<f32> {
        Frustum::from_modelview_projection(
            (dependents.proj_mat_treeculler
                * dependents.view_mat
                * Mat4::translation_3d(-self.focus.map(|e| e.trunc())))
            .into_col_arrays(),
        )
    }

    pub fn frustum(&self) -> &Frustum<f32> { &self.frustum }

    pub fn dependents(&self) -> Dependents { self.dependents }

    /// Rotate the camera about its focus by the given delta, limiting the input
    /// accordingly.
    pub fn rotate_by(&mut self, delta: Vec3<f32>) {
        let delta = delta * self.fixate;
        // Wrap camera yaw
        self.tgt_ori.x = (self.tgt_ori.x + delta.x).rem_euclid(2.0 * PI);
        // Clamp camera pitch to the vertical limits
        self.tgt_ori.y = (self.tgt_ori.y + delta.y).clamp(-PI / 2.0 + 0.001, PI / 2.0 - 0.001);
        // Wrap camera roll
        self.tgt_ori.z = (self.tgt_ori.z + delta.z).rem_euclid(2.0 * PI);
    }

    /// Set the orientation of the camera about its focus.
    pub fn set_orientation(&mut self, ori: Vec3<f32>) { self.tgt_ori = clamp_and_modulate(ori); }

    /// Set the orientation of the camera about its focus without lerping.
    pub fn set_orientation_instant(&mut self, ori: Vec3<f32>) {
        self.set_orientation(ori);
        self.ori = self.tgt_ori;
    }

    /// Zoom the camera by the given delta, limiting the input accordingly.
    pub fn zoom_by(&mut self, delta: f32, cap: Option<f32>) {
        if self.mode == CameraMode::ThirdPerson {
            // Clamp camera dist to the 2 <= x <= infinity range
            self.tgt_dist = (self.tgt_dist + delta).max(2.0);
        }

        if let Some(cap) = cap {
            self.tgt_dist = self.tgt_dist.min(cap);
        }
    }

    /// Zoom with the ability to switch between first and third-person mode.
    ///
    /// Note that cap > 18237958000000.0 can cause panic due to float overflow
    pub fn zoom_switch(&mut self, delta: f32, cap: f32) {
        if delta > 0_f32 || self.mode != CameraMode::FirstPerson {
            let t = self.tgt_dist + delta;
            const MIN_THIRD_PERSON: f32 = 2.35;
            match self.mode {
                CameraMode::ThirdPerson => {
                    if t < MIN_THIRD_PERSON {
                        self.set_mode(CameraMode::FirstPerson);
                    } else {
                        self.tgt_dist = t;
                    }
                },
                CameraMode::FirstPerson => {
                    self.set_mode(CameraMode::ThirdPerson);
                    self.tgt_dist = MIN_THIRD_PERSON;
                },
                _ => {},
            }
        }

        self.tgt_dist = self.tgt_dist.min(cap);
    }

    /// Get the distance of the camera from the focus
    pub fn get_distance(&self) -> f32 { self.dist }

    /// Set the distance of the camera from the focus (i.e., zoom).
    pub fn set_distance(&mut self, dist: f32) { self.tgt_dist = dist; }

    pub fn update(&mut self, time: f64, dt: f32, smoothing_enabled: bool) {
        // This is horribly frame time dependent, but so is most of the game
        let delta = self.last_time.replace(time).map_or(0.0, |t| time - t);
        if (self.dist - self.tgt_dist).abs() > 0.01 {
            self.dist = Lerp::lerp(
                self.dist,
                self.tgt_dist,
                0.65 * (delta as f32) / self.interp_time(),
            );
        }

        if (self.fov - self.tgt_fov).abs() > 0.01 {
            self.fov = Lerp::lerp(
                self.fov,
                self.tgt_fov,
                0.65 * (delta as f32) / self.interp_time(),
            );
        }

        if (self.fixate - self.tgt_fixate).abs() > 0.01 {
            self.fixate = Lerp::lerp(
                self.fixate,
                self.tgt_fixate,
                0.65 * (delta as f32) / self.interp_time(),
            );
        }

        if (self.focus - self.tgt_focus).magnitude_squared() > 0.001 {
            let lerped_focus = Lerp::lerp(
                self.focus,
                self.tgt_focus,
                (delta as f32) / self.interp_time()
                    * if matches!(self.mode, CameraMode::FirstPerson) {
                        2.0
                    } else {
                        1.0
                    },
            );

            self.focus.x = lerped_focus.x;
            self.focus.y = lerped_focus.y;

            // Always lerp in z
            self.focus.z = lerped_focus.z;
        }

        let lerp_angle = |a: f32, b: f32, rate: f32| {
            let offs = [-2.0 * PI, 0.0, 2.0 * PI]
                .iter()
                .min_by_key(|offs: &&f32| ((a - (b + *offs)).abs() * 1000.0) as i32)
                .unwrap();
            Lerp::lerp(a, b + *offs, rate)
        };

        let ori = if smoothing_enabled {
            Vec3::new(
                lerp_angle(self.ori.x, self.tgt_ori.x, LERP_ORI_RATE * dt),
                Lerp::lerp(self.ori.y, self.tgt_ori.y, LERP_ORI_RATE * dt),
                lerp_angle(self.ori.z, self.tgt_ori.z, LERP_ORI_RATE * dt),
            )
        } else {
            self.tgt_ori
        };
        self.ori = clamp_and_modulate(ori);
    }

    pub fn interp_time(&self) -> f32 {
        match self.mode {
            CameraMode::FirstPerson => FIRST_PERSON_INTERP_TIME,
            CameraMode::ThirdPerson => THIRD_PERSON_INTERP_TIME,
            CameraMode::Freefly => FREEFLY_INTERP_TIME,
        }
    }

    /// Get the focus position of the camera.
    pub fn get_focus_pos(&self) -> Vec3<f32> { self.focus }

    /// Set the focus position of the camera.
    pub fn set_focus_pos(&mut self, focus: Vec3<f32>) { self.tgt_focus = focus; }

    /// Set the focus position of the camera, without lerping.
    pub fn force_focus_pos(&mut self, focus: Vec3<f32>) {
        self.tgt_focus = focus;
        self.focus = focus;
    }

    /// Get the aspect ratio of the camera.
    pub fn get_aspect_ratio(&self) -> f32 { self.aspect }

    /// Set the aspect ratio of the camera.
    pub fn set_aspect_ratio(&mut self, aspect: f32) {
        self.aspect = if aspect.is_normal() { aspect } else { 1.0 };
    }

    /// Get the orientation of the camera.
    pub fn get_orientation(&self) -> Vec3<f32> { self.ori }

    /// Get the field of view of the camera in radians, taking into account
    /// fixation.
    pub fn get_effective_fov(&self) -> f32 { self.fov * self.fixate }

    // /// Get the field of view of the camera in radians.
    // pub fn get_fov(&self) -> f32 { self.fov }

    /// Set the field of view of the camera in radians.
    pub fn set_fov(&mut self, fov: f32) { self.tgt_fov = fov; }

    /// Set the 'fixation' proportion, allowing the camera to focus in with
    /// precise aiming. Fixation is applied on top of the regular FoV.
    pub fn set_fixate(&mut self, fixate: f32) { self.tgt_fixate = fixate; }

    /// Set the FOV in degrees
    pub fn set_fov_deg(&mut self, fov: u16) {
        //Magic value comes from pi/180; no use recalculating.
        self.set_fov((fov as f32) * 0.01745329)
    }

    /// Set the mode of the camera.
    pub fn set_mode(&mut self, mode: CameraMode) {
        if self.mode != mode {
            self.mode = mode;
            match self.mode {
                CameraMode::ThirdPerson => {
                    self.zoom_by(5.0, None);
                },
                CameraMode::FirstPerson => {
                    self.set_distance(MIN_ZOOM);
                },
                CameraMode::Freefly => {
                    self.set_distance(MIN_ZOOM);
                },
            }
        }
    }

    /// Get the mode of the camera
    pub fn get_mode(&self) -> CameraMode {
        // Perform a bit of a trick... don't report first-person until the camera has
        // lerped close enough to the player.
        match self.mode {
            CameraMode::FirstPerson if self.dist < 0.5 => CameraMode::FirstPerson,
            CameraMode::FirstPerson => CameraMode::ThirdPerson,
            mode => mode,
        }
    }

    /// Cycle the camera to its next valid mode. If is_admin is false then only
    /// modes which are accessible without admin access will be cycled to.
    pub fn next_mode(&mut self, is_admin: bool, has_target: bool) {
        if has_target {
            self.set_mode(match self.mode {
                CameraMode::ThirdPerson => CameraMode::FirstPerson,
                CameraMode::FirstPerson => {
                    if is_admin {
                        CameraMode::Freefly
                    } else {
                        CameraMode::ThirdPerson
                    }
                },
                CameraMode::Freefly => CameraMode::ThirdPerson,
            });
        } else {
            self.set_mode(CameraMode::Freefly);
        }
    }

    /// Return a unit vector in the forward direction for the current camera
    /// orientation
    pub fn forward(&self) -> Vec3<f32> {
        Vec3::new(
            f32::sin(self.ori.x) * f32::cos(self.ori.y),
            f32::cos(self.ori.x) * f32::cos(self.ori.y),
            -f32::sin(self.ori.y),
        )
    }

    /// Return a unit vector in the right direction for the current camera
    /// orientation
    pub fn right(&self) -> Vec3<f32> {
        const UP: Vec3<f32> = Vec3::new(0.0, 0.0, 1.0);
        self.forward().cross(UP).normalized()
    }

    /// Return a unit vector in the forward direction on the XY plane for
    /// the current camera orientation
    pub fn forward_xy(&self) -> Vec2<f32> { Vec2::new(f32::sin(self.ori.x), f32::cos(self.ori.x)) }

    /// Return a unit vector in the right direction on the XY plane for
    /// the current camera orientation
    pub fn right_xy(&self) -> Vec2<f32> { Vec2::new(f32::cos(self.ori.x), -f32::sin(self.ori.x)) }
}
