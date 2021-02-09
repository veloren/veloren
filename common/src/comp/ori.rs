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
    /// assert!((ori1.look_dir().dot(*ori2.look_dir()) - 1.0).abs() <= std::f32::EPSILON);
    /// ```
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
    /// assert!((ori1.look_dir().dot(*ori2.look_dir()) - 1.0).abs() <= std::f32::EPSILON);
    /// ```
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
    /// ```
    /// use veloren_common::comp::Ori;
    ///
    /// let ang = 45_f32.to_radians();
    /// let zenith = vek::Vec3::unit_z();
    ///
    /// let rl = Ori::default().rolled_left(ang);
    /// assert!((rl.up().angle_between(zenith) - ang).abs() <= std::f32::EPSILON);
    /// assert!(rl.uprighted().up().angle_between(zenith) <= std::f32::EPSILON);
    ///
    /// let pd_rr = Ori::default().pitched_down(ang).rolled_right(ang);
    /// let pd_upr = pd_rr.uprighted();
    ///
    /// assert!((pd_upr.up().angle_between(zenith) - ang).abs() <= std::f32::EPSILON);
    ///
    /// let ang1 = pd_upr.rolled_right(ang).up().angle_between(zenith);
    /// let ang2 = pd_rr.up().angle_between(zenith);
    /// assert!((ang1 - ang2).abs() <= std::f32::EPSILON);
    /// ```
    pub fn uprighted(self) -> Self {
        let fw = self.look_dir();
        match Dir::up().projected(&Plane::from(fw)) {
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
        let from = Dir::default();
        let q = Quaternion::<f32>::rotation_from_to_3d(*from, *dir).normalized();

        Self(q).uprighted()
    }
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
    fn from(ori: Ori) -> Self { ori.look_vec().xy() }
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
impl Into<SerdeOri> for Ori {
    fn into(self) -> SerdeOri { SerdeOri(self.to_quat()) }
}

impl Component for Ori {
    type Storage = IdvStorage<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_to_dir() {
        let from_to = |dir: Dir| {
            let ori = Ori::from(dir);

            approx::assert_relative_eq!(ori.look_dir().dot(*dir), 1.0);
            approx::assert_relative_eq!((ori.to_quat() * Dir::default()).dot(*dir), 1.0);
        };

        let angles = 32;
        for i in 0..angles {
            let theta = PI * 2. * (i as f32) / (angles as f32);
            let v = Vec3::unit_y();
            let q = Quaternion::rotation_x(theta);
            from_to(Dir::new(q * v));
            let v = Vec3::unit_z();
            let q = Quaternion::rotation_y(theta);
            from_to(Dir::new(q * v));
            let v = Vec3::unit_x();
            let q = Quaternion::rotation_z(theta);
            from_to(Dir::new(q * v));
        }
    }

    #[test]
    fn dirs() {
        let ori = Ori::default();
        let def = Dir::default();
        for dir in &[ori.up(), ori.down(), ori.left(), ori.right()] {
            approx::assert_relative_eq!(dir.dot(*def), 0.0);
        }
    }
}
