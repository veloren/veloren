use crate::util::{Dir, Plane, Projection};
use serde::{Deserialize, Serialize};
use specs::Component;
use specs_idvs::IdvStorage;
use std::f32::consts::PI;
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
        debug_assert!(quat.into_vec4().map(f32::is_finite).reduce_and());
        debug_assert!(quat.into_vec4().is_normalized());
        Self(quat)
    }

    /// Tries to convert into a Dir and then the appropriate rotation
    pub fn from_unnormalized_vec<T>(vec: T) -> Option<Self>
    where
        T: Into<Vec3<f32>>,
    {
        Dir::from_unnormalized(vec.into()).map(Self::from)
    }

    pub fn to_vec(self) -> Vec3<f32> { *self.look_dir() }

    pub fn to_quat(self) -> Quaternion<f32> { self.0 }

    pub fn look_dir(&self) -> Dir { Dir::new(self.0 * *Dir::default()) }

    pub fn up(&self) -> Dir { self.pitched_up(PI / 2.0).look_dir() }

    pub fn down(&self) -> Dir { self.pitched_down(PI / 2.0).look_dir() }

    pub fn left(&self) -> Dir { self.yawed_left(PI / 2.0).look_dir() }

    pub fn right(&self) -> Dir { self.yawed_right(PI / 2.0).look_dir() }

    pub fn slerp(ori1: Self, ori2: Self, s: f32) -> Self {
        Self(Quaternion::slerp(ori1.0, ori2.0, s).normalized())
    }

    pub fn slerped_towards(self, ori: Ori, s: f32) -> Self { Self::slerp(self, ori, s) }

    /// Multiply rotation quaternion by `q`
    /// (the rotations are in local vector space).
    pub fn rotated(self, q: Quaternion<f32>) -> Self { Self((self.0 * q).normalized()) }

    /// Premultiply rotation quaternion by `q`
    /// (the rotations are in global vector space).
    pub fn prerotated(self, q: Quaternion<f32>) -> Self { Self((q * self.0).normalized()) }

    /// Take `global` into this Ori's local vector space
    pub fn global_to_local<T>(&self, global: T) -> <Quaternion<f32> as std::ops::Mul<T>>::Output
    where
        Quaternion<f32>: std::ops::Mul<T>,
    {
        self.0.inverse() * global
    }

    /// Take `local` into the global vector space
    pub fn local_to_global<T>(&self, local: T) -> <Quaternion<f32> as std::ops::Mul<T>>::Output
    where
        Quaternion<f32>: std::ops::Mul<T>,
    {
        self.0 * local
    }

    pub fn pitched_up(self, angle_radians: f32) -> Self {
        self.rotated(Quaternion::rotation_x(angle_radians))
    }

    pub fn pitched_down(self, angle_radians: f32) -> Self {
        self.rotated(Quaternion::rotation_x(-angle_radians))
    }

    pub fn yawed_left(self, angle_radians: f32) -> Self {
        self.rotated(Quaternion::rotation_z(angle_radians))
    }

    pub fn yawed_right(self, angle_radians: f32) -> Self {
        self.rotated(Quaternion::rotation_z(-angle_radians))
    }

    pub fn rolled_left(self, angle_radians: f32) -> Self {
        self.rotated(Quaternion::rotation_y(-angle_radians))
    }

    pub fn rolled_right(self, angle_radians: f32) -> Self {
        self.rotated(Quaternion::rotation_y(angle_radians))
    }

    /// Returns a version without sideways tilt (roll)
    ///
    /// # Examples
    /// ```
    /// use veloren_common::comp::Ori;
    ///
    /// let ang = 45_f32.to_radians();
    /// let zenith = vek::Vec3::unit_z();
    ///
    /// let rl = Ori::default().rolled_left(ang);
    /// assert!((rl.up().angle_between(zenith) - ang).abs() < std::f32::EPSILON);
    /// assert!(rl.uprighted().up().angle_between(zenith) < std::f32::EPSILON);
    ///
    /// let pd_rr = Ori::default().pitched_down(ang).rolled_right(ang);
    /// let pd_upr = pd_rr.uprighted();
    ///
    /// assert!((pd_upr.up().angle_between(zenith) - ang).abs() < std::f32::EPSILON);
    ///
    /// let ang1 = pd_upr.rolled_right(ang).up().angle_between(zenith);
    /// let ang2 = pd_rr.up().angle_between(zenith);
    /// assert!((ang1 - ang2).abs() < std::f32::EPSILON);
    /// ```
    pub fn uprighted(self) -> Self {
        let fw = self.look_dir();
        match Dir::new(Vec3::unit_z()).projected(&Plane::from(fw)) {
            Some(dir_p) => {
                let up = self.up();
                let go_right_s = fw.cross(*up).dot(*dir_p).signum();
                self.rolled_right(up.angle_between(*dir_p) * go_right_s)
            },
            None => self,
        }
    }

    fn is_normalized(&self) -> bool { self.0.into_vec4().is_normalized() }
}

impl From<Dir> for Ori {
    fn from(dir: Dir) -> Self {
        let from = *Dir::default();
        Self::from(Quaternion::<f32>::rotation_from_to_3d(from, *dir)).uprighted()
    }
}

impl From<Ori> for Quaternion<f32> {
    fn from(Ori(q): Ori) -> Self { q }
}

impl From<Quaternion<f32>> for Ori {
    fn from(quat: Quaternion<f32>) -> Self { Self(quat.normalized()) }
}

impl From<vek::quaternion::repr_simd::Quaternion<f32>> for Ori {
    fn from(
        vek::quaternion::repr_simd::Quaternion { x, y, z, w }: vek::quaternion::repr_simd::Quaternion<f32>,
    ) -> Self {
        Self::from(Quaternion { x, y, z, w })
    }
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
    fn from(ori: Ori) -> Self { *ori.look_dir() }
}

impl From<Ori> for vek::vec::repr_simd::Vec3<f32> {
    fn from(ori: Ori) -> Self { vek::vec::repr_simd::Vec3::from(*ori.look_dir()) }
}

impl From<Ori> for Vec2<f32> {
    fn from(ori: Ori) -> Self { ori.look_dir().xy() }
}

impl From<Ori> for vek::vec::repr_simd::Vec2<f32> {
    fn from(ori: Ori) -> Self { vek::vec::repr_simd::Vec2::from(ori.look_dir().xy()) }
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
impl Into<SerdeOri> for Ori {
    fn into(self) -> SerdeOri { SerdeOri(self.0) }
}

impl Component for Ori {
    type Storage = IdvStorage<Self>;
}
