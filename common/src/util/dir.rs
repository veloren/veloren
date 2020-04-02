use vek::*;

/// Type representing a direction using Vec3 that is normalized and NaN free
/// These properties are enforced actively via panics when `debug_assertions` is
/// enabled
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(into = "SerdeDir")]
#[serde(from = "SerdeDir")]
pub struct Dir(Vec3<f32>);
impl Default for Dir {
    fn default() -> Self { Self(Vec3::unit_y()) }
}

// Validate at Deserialization
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
struct SerdeDir(Vec3<f32>);
impl From<SerdeDir> for Dir {
    fn from(dir: SerdeDir) -> Self {
        let dir = dir.0;
        if dir.map(f32::is_nan).reduce_or() {
            warn!("Deserialized dir containing NaNs, replacing with default");
            Default::default()
        } else if !dir.is_normalized() {
            warn!("Deserialized unnormalized dir, replacing with default");
            Default::default()
        } else {
            Self(dir)
        }
    }
}
impl Into<SerdeDir> for Dir {
    fn into(self) -> SerdeDir { SerdeDir(*self) }
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

    /// Note: this uses `from` if `to` is unormalizable
    pub fn slerp_to_vec3(from: Self, to: Vec3<f32>, factor: f32) -> Self {
        Self(slerp_to_unnormalized(from.0, to, factor).unwrap_or_else(|e| e))
    }

    pub fn is_valid(&self) -> bool { !self.0.map(f32::is_nan).reduce_or() && self.is_normalized() }
}

impl std::ops::Deref for Dir {
    type Target = Vec3<f32>;

    fn deref(&self) -> &Vec3<f32> { &self.0 }
}

impl From<Vec3<f32>> for Dir {
    fn from(dir: Vec3<f32>) -> Self { Dir::new(dir.into()) }
}
/// Begone ye NaN's
/// Slerp two `Vec3`s skipping the slerp if their directions are very close
/// This avoids a case where `vek`s slerp produces NaN's
/// Additionally, it avoids unnecessary calculations if they are near identical
/// Assumes `from` and `to` are normalized and returns a normalized vector
#[inline(always)]
fn slerp_normalized(from: vek::Vec3<f32>, to: vek::Vec3<f32>, factor: f32) -> vek::Vec3<f32> {
    debug_assert!(!to.map(f32::is_nan).reduce_or());
    debug_assert!(!from.map(f32::is_nan).reduce_or());
    // Ensure from is normalized
    #[cfg(debug_assertions)]
    {
        if {
            let len_sq = from.magnitude_squared();
            len_sq < 0.999 || len_sq > 1.001
        } {
            panic!("Called slerp_normalized with unnormalized from: {:?}", from);
        }
    }
    // Ensure to is normalized
    #[cfg(debug_assertions)]
    {
        if {
            let len_sq = from.magnitude_squared();
            len_sq < 0.999 || len_sq > 1.001
        } {
            panic!("Called slerp_normalized with unnormalized to: {:?}", to);
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
/// Returns `Err(from)`` if `to` is unormalizable
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
