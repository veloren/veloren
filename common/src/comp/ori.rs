use crate::util::Dir;
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
    pub fn from_unnormalized_vec(vec: Vec3<f32>) -> Option<Self> {
        Dir::from_unnormalized(vec).map(Self::from)
    }

    pub fn to_vec(self) -> Vec3<f32> { *self.look_dir() }

    pub fn to_quat(self) -> Quaternion<f32> { self.0 }

    /// Transform the vector from local into global vector space
    pub fn relative_to_world(&self, vec: Vec3<f32>) -> Vec3<f32> { self.0 * vec }

    /// Transform the vector from global into local vector space
    pub fn relative_to_self(&self, vec: Vec3<f32>) -> Vec3<f32> { self.0.inverse() * vec }

    pub fn look_dir(&self) -> Dir { Dir::new(self.0.normalized() * *Dir::default()) }

    pub fn up(&self) -> Dir { self.pitched_up(PI / 2.0).look_dir() }

    pub fn down(&self) -> Dir { self.pitched_down(PI / 2.0).look_dir() }

    pub fn left(&self) -> Dir { self.yawed_left(PI / 2.0).look_dir() }

    pub fn right(&self) -> Dir { self.yawed_right(PI / 2.0).look_dir() }

    pub fn slerp(ori1: Self, ori2: Self, s: f32) -> Self {
        Self(Quaternion::slerp(ori1.0, ori2.0, s).normalized())
    }

    pub fn slerped_towards(self, ori: Ori, s: f32) -> Self { Self::slerp(self, ori, s) }

    /// Multiply rotation quaternion by `q`
    pub fn rotated(self, q: Quaternion<f32>) -> Self { Self((self.0 * q).normalized()) }

    /// Premultiply rotation quaternion by `q`
    pub fn rotated_world(self, q: Quaternion<f32>) -> Self { Self((q * self.0).normalized()) }

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
    pub fn uprighted(self) -> Self { self.look_dir().into() }

    fn is_normalized(&self) -> bool { self.0.into_vec4().is_normalized() }
}

impl From<Dir> for Ori {
    fn from(dir: Dir) -> Self {
        // rotate horizontally first and then vertically to prevent rolling
        let from = *Dir::default();
        let q1 = (*dir * Vec3::new(1.0, 1.0, 0.0))
            .try_normalized()
            .map(|hv| Quaternion::<f32>::rotation_from_to_3d(from, hv).normalized())
            .unwrap_or_default();
        let q2 = (from + Vec3::new(0.0, 0.0, dir.z))
            .try_normalized()
            .map(|to| Quaternion::<f32>::rotation_from_to_3d(from, to).normalized())
            .unwrap_or_default();
        Self((q1 * q2).normalized())
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
        Self(Quaternion { x, y, z, w }.normalized())
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

impl From<Ori> for Vec2<f32> {
    fn from(ori: Ori) -> Self { ori.look_dir().xy() }
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
