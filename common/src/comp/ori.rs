use crate::util::{Dir, Plane, Projection};
use core::f32::consts::{FRAC_PI_2, PI, TAU};
use serde::{Deserialize, Serialize};
use specs::Component;
use vek::{Quaternion, Vec2, Vec3};

// Orientation
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(into = "SerdeOri")]
#[serde(from = "SerdeOri")]
pub struct Ori(Quaternion<f32>);

impl Default for Ori {
    /// Returns the default orientation (no rotation; default Dir)
    fn default() -> Self { Self(Quaternion::identity()) }
}

impl Ori {
    pub fn new(quat: Quaternion<f32>) -> Self {
        #[cfg(debug_assert)]
        {
            let v4 = quat.into_vec4();
            debug_assert!(v4.map(f32::is_finite).reduce_and());
            debug_assert!(v4.is_normalized());
        }
        Self(quat)
    }

    /// Tries to convert into a Dir and then the appropriate rotation
    pub fn from_unnormalized_vec<T>(vec: T) -> Option<Self>
    where
        T: Into<Vec3<f32>>,
    {
        Dir::from_unnormalized(vec.into()).map(Self::from)
    }

    /// Look direction as a vector (no pedantic normalization performed)
    pub fn look_vec(self) -> Vec3<f32> { self.to_quat() * *Dir::default() }

    /// Get the internal quaternion representing the rotation from
    /// `Dir::default()` to this orientation.
    ///
    /// The operation is a cheap copy.
    pub fn to_quat(self) -> Quaternion<f32> {
        debug_assert!(self.is_normalized());
        self.0
    }

    /// Look direction (as a Dir it is pedantically normalized)
    pub fn look_dir(&self) -> Dir { self.to_quat() * Dir::default() }

    pub fn up(&self) -> Dir { self.pitched_up(PI / 2.0).look_dir() }

    pub fn down(&self) -> Dir { self.pitched_down(PI / 2.0).look_dir() }

    pub fn left(&self) -> Dir { self.yawed_left(PI / 2.0).look_dir() }

    pub fn right(&self) -> Dir { self.yawed_right(PI / 2.0).look_dir() }

    pub fn slerp(ori1: Self, ori2: Self, s: f32) -> Self {
        Self(Quaternion::slerp(ori1.0, ori2.0, s).normalized())
    }

    #[must_use]
    pub fn slerped_towards(self, ori: Ori, s: f32) -> Self { Self::slerp(self, ori, s) }

    /// Multiply rotation quaternion by `q`
    /// (the rotations are in local vector space).
    ///
    /// ```
    /// use vek::{Quaternion, Vec3};
    /// use veloren_common::{comp::Ori, util::Dir};
    ///
    /// let ang = 90_f32.to_radians();
    /// let roll_right = Quaternion::rotation_y(ang);
    /// let pitch_up = Quaternion::rotation_x(ang);
    ///
    /// let ori1 = Ori::from(Dir::new(Vec3::unit_x()));
    /// let ori2 = Ori::default().rotated(roll_right).rotated(pitch_up);
    ///
    /// assert!((ori1.look_dir().dot(*ori2.look_dir()) - 1.0).abs() <= f32::EPSILON);
    /// ```
    #[must_use]
    pub fn rotated(self, q: Quaternion<f32>) -> Self {
        Self((self.to_quat() * q.normalized()).normalized())
    }

    /// Premultiply rotation quaternion by `q`
    /// (the rotations are in global vector space).
    ///
    /// ```
    /// use vek::{Quaternion, Vec3};
    /// use veloren_common::{comp::Ori, util::Dir};
    ///
    /// let ang = 90_f32.to_radians();
    /// let roll_right = Quaternion::rotation_y(ang);
    /// let pitch_up = Quaternion::rotation_x(ang);
    ///
    /// let ori1 = Ori::from(Dir::up());
    /// let ori2 = Ori::default().prerotated(roll_right).prerotated(pitch_up);
    ///
    /// assert!((ori1.look_dir().dot(*ori2.look_dir()) - 1.0).abs() <= f32::EPSILON);
    /// ```
    #[must_use]
    pub fn prerotated(self, q: Quaternion<f32>) -> Self {
        Self((q.normalized() * self.to_quat()).normalized())
    }

    /// Take `global` into this Ori's local vector space
    ///
    /// ```
    /// use vek::Vec3;
    /// use veloren_common::{comp::Ori, util::Dir};
    ///
    /// let ang = 90_f32.to_radians();
    /// let (fw, left, up) = (Dir::default(), Dir::left(), Dir::up());
    ///
    /// let ori = Ori::default().rolled_left(ang).pitched_up(ang);
    /// approx::assert_relative_eq!(ori.global_to_local(fw).dot(*-up), 1.0);
    /// approx::assert_relative_eq!(ori.global_to_local(left).dot(*fw), 1.0);
    /// let ori = Ori::default().rolled_right(ang).pitched_up(2.0 * ang);
    /// approx::assert_relative_eq!(ori.global_to_local(up).dot(*left), 1.0);
    /// ```
    pub fn global_to_local<T>(&self, global: T) -> <Quaternion<f32> as std::ops::Mul<T>>::Output
    where
        Quaternion<f32>: std::ops::Mul<T>,
    {
        self.to_quat().inverse() * global
    }

    /// Take `local` into the global vector space
    ///
    /// ```
    /// use vek::Vec3;
    /// use veloren_common::{comp::Ori, util::Dir};
    ///
    /// let ang = 90_f32.to_radians();
    /// let (fw, left, up) = (Dir::default(), Dir::left(), Dir::up());
    ///
    /// let ori = Ori::default().rolled_left(ang).pitched_up(ang);
    /// approx::assert_relative_eq!(ori.local_to_global(fw).dot(*left), 1.0);
    /// approx::assert_relative_eq!(ori.local_to_global(left).dot(*-up), 1.0);
    /// let ori = Ori::default().rolled_right(ang).pitched_up(2.0 * ang);
    /// approx::assert_relative_eq!(ori.local_to_global(up).dot(*left), 1.0);
    /// ```
    pub fn local_to_global<T>(&self, local: T) -> <Quaternion<f32> as std::ops::Mul<T>>::Output
    where
        Quaternion<f32>: std::ops::Mul<T>,
    {
        self.to_quat() * local
    }

    #[must_use]
    pub fn to_horizontal(self) -> Self {
        // We don't use Self::look_dir to avoid the extra normalization step within
        // Dir's Quaternion Mul impl
        let fw = self.to_quat() * Dir::default().to_vec();
        // Check that dir is not straight up/down
        // Uses a multiple of EPSILON to be safe
        // We can just check z since beyond floating point errors `fw` should be
        // normalized
        if 1.0 - fw.z.abs() > f32::EPSILON * 4.0 {
            // We know direction lies in the xy plane so we only need to compute a rotation
            // about the z-axis
            let Vec2 { x, y } = fw.xy().normalized();
            // Negate x and swap coords since we want to compute the angle from y+
            let quat = rotation_2d(Vec2::new(y, -x), Vec3::unit_z());

            Self(quat)
        } else {
            // if the direction is straight down, pitch up, or if straight up, pitch down
            if fw.z < 0.0 {
                self.pitched_up(FRAC_PI_2)
            } else {
                self.pitched_down(FRAC_PI_2)
            }
            // TODO: test this alternative for speed and correctness compared to
            // current impl
            //
            // removes a branch
            //
            // use core::f32::consts::FRAC_1_SQRT_2;
            // let cos = FRAC_1_SQRT_2;
            // let sin = -FRAC_1_SQRT_2 * fw.z.signum();
            // let axis = Vec3::unit_x();
            // let scalar = cos;
            // let vector = sin * axis;
            // Self((self.0 * Quaternion::from_scalar_and_vec3((scalar,
            // vector))).normalized())
        }
    }

    /// Find the angle between two `Ori`s
    ///
    /// NOTE: This finds the angle of the quaternion between the two `Ori`s
    /// which can involve rolling and thus can be larger than simply the
    /// angle between vectors at the start and end points.
    ///
    /// Returns angle in radians
    pub fn angle_between(self, other: Self) -> f32 {
        // Compute quaternion from one ori to the other
        // https://www.mathworks.com/matlabcentral/answers/476474-how-to-find-the-angle-between-two-quaternions#answer_387973
        let between = self.to_quat().conjugate() * other.to_quat();
        // Then compute it's angle
        // http://www.euclideanspace.com/maths/geometry/rotations/conversions/quaternionToAngle/
        //
        // NOTE: acos is very sensitive to errors at small angles
        // - https://www.researchgate.net/post/How_do_I_calculate_the_smallest_angle_between_two_quaternions
        // - see angle_between unit test epislons
        let angle = 2.0 * between.w.clamp(-1.0, 1.0).acos();
        if angle < PI { angle } else { TAU - angle }
    }

    pub fn dot(self, other: Self) -> f32 { self.look_vec().dot(other.look_vec()) }

    #[must_use]
    pub fn pitched_up(self, angle_radians: f32) -> Self {
        self.rotated(Quaternion::rotation_x(angle_radians))
    }

    #[must_use]
    pub fn pitched_down(self, angle_radians: f32) -> Self {
        self.rotated(Quaternion::rotation_x(-angle_radians))
    }

    #[must_use]
    pub fn yawed_left(self, angle_radians: f32) -> Self {
        self.rotated(Quaternion::rotation_z(angle_radians))
    }

    #[must_use]
    pub fn yawed_right(self, angle_radians: f32) -> Self {
        self.rotated(Quaternion::rotation_z(-angle_radians))
    }

    #[must_use]
    pub fn rolled_left(self, angle_radians: f32) -> Self {
        self.rotated(Quaternion::rotation_y(-angle_radians))
    }

    #[must_use]
    pub fn rolled_right(self, angle_radians: f32) -> Self {
        self.rotated(Quaternion::rotation_y(angle_radians))
    }

    /// Returns a version which is rolled such that its up points towards `dir`
    /// as much as possible without pitching or yawing
    #[must_use]
    pub fn rolled_towards(self, dir: Dir) -> Self {
        dir.projected(&Plane::from(self.look_dir()))
            .map_or(self, |dir| self.prerotated(self.up().rotation_between(dir)))
    }

    /// Returns a version which has been pitched towards `dir` as much as
    /// possible without yawing or rolling
    #[must_use]
    pub fn pitched_towards(self, dir: Dir) -> Self {
        dir.projected(&Plane::from(self.right()))
            .map_or(self, |dir_| {
                self.prerotated(self.look_dir().rotation_between(dir_))
            })
    }

    /// Returns a version which has been yawed towards `dir` as much as possible
    /// without pitching or rolling
    #[must_use]
    pub fn yawed_towards(self, dir: Dir) -> Self {
        dir.projected(&Plane::from(self.up())).map_or(self, |dir_| {
            self.prerotated(self.look_dir().rotation_between(dir_))
        })
    }

    /// Returns a version without sideways tilt (roll)
    ///
    /// ```
    /// use veloren_common::comp::Ori;
    ///
    /// let ang = 45_f32.to_radians();
    /// let zenith = vek::Vec3::unit_z();
    ///
    /// let rl = Ori::default().rolled_left(ang);
    /// assert!((rl.up().angle_between(zenith) - ang).abs() <= f32::EPSILON);
    /// assert!(rl.uprighted().up().angle_between(zenith) <= f32::EPSILON);
    ///
    /// let pd_rr = Ori::default().pitched_down(ang).rolled_right(ang);
    /// let pd_upr = pd_rr.uprighted();
    ///
    /// assert!((pd_upr.up().angle_between(zenith) - ang).abs() <= f32::EPSILON);
    ///
    /// let ang1 = pd_upr.rolled_right(ang).up().angle_between(zenith);
    /// let ang2 = pd_rr.up().angle_between(zenith);
    /// assert!((ang1 - ang2).abs() <= f32::EPSILON);
    /// ```
    #[must_use]
    pub fn uprighted(self) -> Self { self.look_dir().into() }

    fn is_normalized(&self) -> bool { self.0.into_vec4().is_normalized() }
}

/// Produce a quaternion from an axis to rotate about and a 2D point on the unit
/// circle to rotate to
///
/// NOTE: the provided axis and 2D vector must be normalized
fn rotation_2d(Vec2 { x, y }: Vec2<f32>, axis: Vec3<f32>) -> Quaternion<f32> {
    // Skip needing the angle for quaternion construction by computing cos/sin
    // directly from the normalized x value
    //
    // scalar = cos(theta / 2)
    // vector = axis * sin(theta / 2)
    //
    // cos(a / 2) = +/- ((1 + cos(a)) / 2)^0.5
    // sin(a / 2) = +/- ((1 - cos(a)) / 2)^0.5
    //
    // scalar = +/- sqrt((1 + cos(a)) / 2)
    // vector = vec3(0, 0, 1) * +/- sqrt((1 - cos(a)) / 2)
    //
    // cos(a) = x / |xy| => x (when normalized)

    // Prevent NaNs from negative sqrt (float errors can put this slightly over 1.0)
    let x = x.clamp(-1.0, 1.0);

    let scalar = ((1.0 + x) / 2.0).sqrt() * y.signum();
    let vector = axis * ((1.0 - x) / 2.0).sqrt();

    // This is normalized by our construction above
    Quaternion::from_scalar_and_vec3((scalar, vector))
}

impl From<Dir> for Ori {
    fn from(dir: Dir) -> Self {
        // Check that dir is not straight up/down
        // Uses a multiple of EPSILON to be safe
        let quat = if 1.0 - dir.z.abs() > f32::EPSILON * 4.0 {
            // Compute rotation that will give an "upright" orientation (no
            // rolling):
            let xy_len = dir.xy().magnitude();
            let xy_norm = dir.xy() / xy_len;
            // Rotation to get to this projected point from the default direction of y+
            // Negate x and swap coords since we want to compute the angle from y+
            let yaw = rotation_2d(Vec2::new(xy_norm.y, -xy_norm.x), Vec3::unit_z());
            // Rotation to then rotate up/down to the match the input direction
            // In this rotated space the xy_len becomes the distance along the x axis
            // And since we rotated around the z-axis the z value is unchanged
            let pitch = rotation_2d(Vec2::new(xy_len, dir.z), Vec3::unit_x());

            (yaw * pitch).normalized()
        } else {
            // Nothing in particular can be considered upright if facing up or down
            // so we just produce a quaternion that will rotate to that direction
            // (once again rotating from y+)
            let pitch = PI / 2.0 * dir.z.signum();
            Quaternion::rotation_x(pitch)
        };

        Self(quat)
    }
}

impl From<Vec3<f32>> for Ori {
    fn from(dir: Vec3<f32>) -> Self { Dir::from_unnormalized(dir).unwrap_or_default().into() }
}

impl From<Quaternion<f32>> for Ori {
    fn from(quat: Quaternion<f32>) -> Self { Self::new(quat) }
}

impl From<vek::quaternion::repr_simd::Quaternion<f32>> for Ori {
    fn from(
        vek::quaternion::repr_simd::Quaternion { x, y, z, w }: vek::quaternion::repr_simd::Quaternion<f32>,
    ) -> Self {
        Self::from(Quaternion { x, y, z, w })
    }
}

impl From<Ori> for Quaternion<f32> {
    fn from(Ori(q): Ori) -> Self { q }
}

impl From<Ori> for vek::quaternion::repr_simd::Quaternion<f32> {
    fn from(Ori(Quaternion { x, y, z, w }): Ori) -> Self {
        vek::quaternion::repr_simd::Quaternion { x, y, z, w }
    }
}

impl From<Ori> for Dir {
    fn from(ori: Ori) -> Self { ori.look_dir() }
}

impl From<Ori> for Vec3<f32> {
    fn from(ori: Ori) -> Self { ori.look_vec() }
}

impl From<Ori> for vek::vec::repr_simd::Vec3<f32> {
    fn from(ori: Ori) -> Self { vek::vec::repr_simd::Vec3::from(ori.look_vec()) }
}

impl From<Ori> for Vec2<f32> {
    fn from(ori: Ori) -> Self { ori.look_dir().to_horizontal().unwrap_or_default().xy() }
}

impl From<Ori> for vek::vec::repr_simd::Vec2<f32> {
    fn from(ori: Ori) -> Self { vek::vec::repr_simd::Vec2::from(ori.look_vec().xy()) }
}

// Validate at Deserialization
#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
struct SerdeOri(Quaternion<f32>);

impl From<SerdeOri> for Ori {
    fn from(serde_quat: SerdeOri) -> Self {
        let quat: Quaternion<f32> = serde_quat.0;
        if quat.into_vec4().map(f32::is_nan).reduce_or() {
            tracing::warn!(
                ?quat,
                "Deserialized rotation quaternion containing NaNs, replacing with default"
            );
            Default::default()
        } else if !Self(quat).is_normalized() {
            tracing::warn!(
                ?quat,
                "Deserialized unnormalized rotation quaternion (magnitude: {}), replacing with \
                 default",
                quat.magnitude()
            );
            Default::default()
        } else {
            Self::new(quat)
        }
    }
}

impl From<Ori> for SerdeOri {
    fn from(other: Ori) -> SerdeOri { SerdeOri(other.to_quat()) }
}

impl Component for Ori {
    type Storage = specs::VecStorage<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper method to produce Dirs at different angles to test
    fn dirs() -> impl Iterator<Item = Dir> {
        let angles = 32;
        (0..angles).flat_map(move |i| {
            let theta = PI * 2.0 * (i as f32) / (angles as f32);

            let v = Vec3::unit_y();
            let q = Quaternion::rotation_x(theta);
            let dir_1 = Dir::new(q * v);

            let v = Vec3::unit_z();
            let q = Quaternion::rotation_y(theta);
            let dir_2 = Dir::new(q * v);

            let v = Vec3::unit_x();
            let q = Quaternion::rotation_z(theta);
            let dir_3 = Dir::new(q * v);

            [dir_1, dir_2, dir_3]
        })
    }

    #[test]
    fn to_horizontal() {
        let to_horizontal = |dir: Dir| {
            let ori = Ori::from(dir);

            let horizontal = ori.to_horizontal();

            approx::assert_relative_eq!(horizontal.look_dir().xy().magnitude(), 1.0);
            approx::assert_relative_eq!(horizontal.look_dir().z, 0.0);
            // Check correctness by comparing with Dir::to_horizontal
            if let Some(dir_h) = ori.look_dir().to_horizontal() {
                let quat_correct = Quaternion::<f32>::rotation_from_to_3d(Dir::default(), dir_h);
                #[rustfmt::skip]
                assert!(
                    dir_h
                        .map2(*horizontal.look_dir(), |d, o| approx::relative_eq!(d, o, epsilon = f32::EPSILON * 4.0))
                        .reduce_and(),
                    "\n\
                    Original: {:?}\n\
                    Dir::to_horizontal: {:?}\n\
                    Ori::to_horizontal(as dir): {:?}\n\
                    Ori::to_horizontal(as quat): {:?}\n\
                    Correct quaternion {:?}",
                    ori.look_dir(),
                    dir_h,
                    horizontal.look_dir(),
                    horizontal,
                    quat_correct,
                );
            }
        };

        dirs().for_each(to_horizontal);
    }

    #[test]
    fn angle_between() {
        let axis_list = (-16..17)
            .map(|i| i as f32 / 16.0)
            .flat_map(|fraction| {
                [
                    Vec3::new(1.0 - fraction, fraction, 0.0),
                    Vec3::new(0.0, 1.0 - fraction, fraction),
                    Vec3::new(fraction, 0.0, 1.0 - fraction),
                ]
            })
            .collect::<Vec<_>>();
        // Iterator over some angles between 0 and 180
        let angles = (0..129).map(|i| i as f32 / 128.0 * PI);

        for angle_a in angles.clone() {
            for angle_b in angles.clone() {
                for axis in axis_list.iter().copied() {
                    let ori_a = Ori(Quaternion::rotation_3d(angle_a, axis));
                    let ori_b = Ori(Quaternion::rotation_3d(angle_b, axis));

                    let angle = (angle_a - angle_b).abs();
                    let epsilon = match angle {
                        angle if angle > 0.5 => f32::EPSILON * 20.0,
                        angle if angle > 0.2 => 0.00001,
                        angle if angle > 0.01 => 0.0001,
                        _ => 0.002,
                    };
                    approx::assert_relative_eq!(
                        ori_a.angle_between(ori_b),
                        angle,
                        epsilon = epsilon,
                    );
                }
            }
        }
    }

    #[test]
    fn from_to_dir() {
        let from_to = |dir: Dir| {
            let ori = Ori::from(dir);

            assert!(ori.is_normalized(), "ori {:?}\ndir {:?}", ori, dir);
            assert!(
                approx::relative_eq!(ori.look_dir().dot(*dir), 1.0),
                "Ori::from(dir).look_dir() != dir\ndir: {:?}\nOri::from(dir).look_dir(): {:?}",
                dir,
                ori.look_dir(),
            );
            approx::assert_relative_eq!((ori.to_quat() * Dir::default()).dot(*dir), 1.0);
        };

        dirs().for_each(from_to);
    }

    #[test]
    fn orthogonal_dirs() {
        let ori = Ori::default();
        let def = Dir::default();
        for dir in &[ori.up(), ori.down(), ori.left(), ori.right()] {
            approx::assert_relative_eq!(dir.dot(*def), 0.0);
        }
    }
}
