use super::{Plane, Projection};
use serde::{Deserialize, Serialize};
use tracing::warn;
use vek::*;

/// Type representing a direction using Vec3 that is normalized and NaN free
/// These properties are enforced actively via panics when `debug_assertions` is
/// enabled
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(into = "SerdeDir")]
#[serde(from = "SerdeDir")]
pub struct Dir(Vec3<f32>);
impl Default for Dir {
    fn default() -> Self { Self::forward() }
}

// Validate at Deserialization
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
struct SerdeDir(Vec3<f32>);
impl From<SerdeDir> for Dir {
    fn from(dir: SerdeDir) -> Self {
        let dir = dir.0;
        if dir.map(f32::is_nan).reduce_or() {
            warn!(
                ?dir,
                "Deserialized dir containing NaNs, replacing with default"
            );
            Default::default()
        } else if !dir.is_normalized() {
            warn!(
                ?dir,
                "Deserialized unnormalized dir, replacing with default"
            );
            Default::default()
        } else {
            Self(dir)
        }
    }
}

impl From<Dir> for SerdeDir {
    fn from(other: Dir) -> SerdeDir { SerdeDir(*other) }
}
/*pub enum TryFromVec3Error {
    ContainsNans,
    NotNormalized,
}

impl TryFrom<Vec3<f32>> for Dir {
    type Error = TryFromVec3Error;

    fn try_from(v: Vec3) -> Result<Self, TryFromVec3Error> {
        if v.map(f32::is_nan).reduce_or() {
            Err(TryFromVec3Error::ContainsNans)
        } else {
            v.try_normalized()
                .map(|n| Self(n))
                .ok_or(TryFromVec3Error::NotNormalized)
        }
    }
}*/

impl Dir {
    pub fn new(dir: Vec3<f32>) -> Self {
        debug_assert!(!dir.map(f32::is_nan).reduce_or());
        debug_assert!(dir.is_normalized());
        Self(dir)
    }

    pub fn from_unnormalized(dirs: Vec3<f32>) -> Option<Self> {
        dirs.try_normalized().map(|dir| {
            #[cfg(debug_assertions)]
            {
                if dir.map(f32::is_nan).reduce_or() {
                    panic!("{} => {}", dirs, dir);
                }
            }
            Self(dir)
        })
    }

    pub fn slerp(from: Self, to: Self, factor: f32) -> Self {
        Self(slerp_normalized(from.0, to.0, factor))
    }

    #[must_use]
    pub fn slerped_to(self, to: Self, factor: f32) -> Self {
        Self(slerp_normalized(self.0, to.0, factor))
    }

    /// Note: this uses `from` if `to` is unnormalizable
    pub fn slerp_to_vec3(from: Self, to: Vec3<f32>, factor: f32) -> Self {
        Self(slerp_to_unnormalized(from.0, to, factor).unwrap_or_else(|e| e))
    }

    pub fn rotation_between(&self, to: Self) -> Quaternion<f32> {
        Quaternion::<f32>::rotation_from_to_3d(self.0, to.0)
    }

    pub fn rotation(&self) -> Quaternion<f32> { Self::default().rotation_between(*self) }

    pub fn is_valid(&self) -> bool { !self.0.map(f32::is_nan).reduce_or() && self.is_normalized() }

    pub fn up() -> Self { Dir::new(Vec3::<f32>::unit_z()) }

    pub fn down() -> Self { -Dir::new(Vec3::<f32>::unit_z()) }

    pub fn left() -> Self { -Dir::new(Vec3::<f32>::unit_x()) }

    pub fn right() -> Self { Dir::new(Vec3::<f32>::unit_x()) }

    pub fn forward() -> Self { Dir::new(Vec3::<f32>::unit_y()) }

    pub fn back() -> Self { -Dir::new(Vec3::<f32>::unit_y()) }

    pub fn to_horizontal(self) -> Option<Self> { Self::from_unnormalized(self.xy().into()) }

    pub fn vec(&self) -> &Vec3<f32> { &self.0 }

    pub fn to_vec(self) -> Vec3<f32> { self.0 }
}

impl std::ops::Deref for Dir {
    type Target = Vec3<f32>;

    fn deref(&self) -> &Vec3<f32> { &self.0 }
}

impl From<Dir> for Vec3<f32> {
    fn from(dir: Dir) -> Self { *dir }
}

impl Projection<Plane> for Dir {
    type Output = Option<Self>;

    fn projected(self, plane: &Plane) -> Self::Output {
        Dir::from_unnormalized(plane.projection(*self))
    }
}

impl Projection<Dir> for Vec3<f32> {
    type Output = Vec3<f32>;

    fn projected(self, dir: &Dir) -> Self::Output {
        let dir = **dir;
        self.dot(dir) * dir
    }
}

impl std::ops::Mul<Dir> for Quaternion<f32> {
    type Output = Dir;

    fn mul(self, dir: Dir) -> Self::Output { Dir((self * *dir).normalized()) }
}

impl std::ops::Neg for Dir {
    type Output = Dir;

    fn neg(self) -> Dir { Dir::new(-self.0) }
}

/// Begone ye NaN's
/// Slerp two `Vec3`s skipping the slerp if their directions are very close
/// This avoids a case where `vek`s slerp produces NaN's
/// Additionally, it avoids unnecessary calculations if they are near identical
/// Assumes `from` and `to` are normalized and returns a normalized vector
#[inline(always)]
fn slerp_normalized(from: Vec3<f32>, to: Vec3<f32>, factor: f32) -> Vec3<f32> {
    debug_assert!(!to.map(f32::is_nan).reduce_or());
    debug_assert!(!from.map(f32::is_nan).reduce_or());
    // Ensure from is normalized
    #[cfg(debug_assertions)]
    {
        let unnormalized = {
            let len_sq = from.magnitude_squared();
            !(0.999..=1.001).contains(&len_sq)
        };

        if unnormalized {
            panic!("Called slerp_normalized with unnormalized `from`: {}", from);
        }
    }

    // Ensure to is normalized
    #[cfg(debug_assertions)]
    {
        let unnormalized = {
            let len_sq = from.magnitude_squared();
            !(0.999..=1.001).contains(&len_sq)
        };

        if unnormalized {
            panic!("Called slerp_normalized with unnormalized `to`: {}", to);
        }
    }

    let dot = from.dot(to);
    if dot >= 1.0 - 1E-6 {
        // Close together, just use to
        return to;
    }

    let (from, to, factor) = if dot < -0.999 {
        // Not linearly independent (slerp will fail since it doesn't check for this)
        // Instead we will choose a midpoint and slerp from or to that depending on the
        // factor
        let mid_dir = if from.z.abs() > 0.999 {
            // If vec's lie along the z-axis default to (1, 0, 0) as midpoint
            Vec3::unit_x()
        } else {
            // Default to picking midpoint in the xy plane
            Vec3::new(from.y, -from.x, 0.0).normalized()
        };

        if factor > 0.5 {
            (mid_dir, to, factor * 2.0 - 1.0)
        } else {
            (from, mid_dir, factor * 2.0)
        }
    } else {
        (from, to, factor)
    };

    let slerped = Vec3::slerp(from, to, factor);
    let slerped_normalized = slerped.normalized();
    // Ensure normalization worked
    // This should not be possible but I will leave it here for now just in case
    // something was missed
    #[cfg(debug_assertions)]
    {
        if !slerped_normalized.is_normalized() || slerped_normalized.map(f32::is_nan).reduce_or() {
            panic!(
                "Failed to normalize {:?} produced from:\nslerp(\n    {:?},\n    {:?},\n    \
                 {:?},\n)\nWith result: {:?})",
                slerped, from, to, factor, slerped_normalized
            );
        }
    }

    slerped_normalized
}

/// Begone ye NaN's
/// Slerp two `Vec3`s skipping the slerp if their directions are very close
/// This avoids a case where `vek`s slerp produces NaN's
/// Additionally, it avoids unnecessary calculations if they are near identical
/// Assumes `from` is normalized and returns a normalized vector, but `to`
/// doesn't need to be normalized
/// Returns `Err(from)`` if `to` is unnormalizable
// TODO: in some cases we might want to base the slerp rate on the magnitude of
// `to` for example when `to` is velocity and `from` is orientation
fn slerp_to_unnormalized(
    from: Vec3<f32>,
    to: Vec3<f32>,
    factor: f32,
) -> Result<Vec3<f32>, Vec3<f32>> {
    to.try_normalized()
        .map(|to| slerp_normalized(from, to, factor))
        .ok_or(from)
}
