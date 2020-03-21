mod color;

pub const GIT_VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/githash"));

lazy_static::lazy_static! {
    pub static ref GIT_HASH: &'static str = GIT_VERSION.split("/").nth(0).expect("failed to retrieve git_hash!");
    pub static ref GIT_DATE: &'static str = GIT_VERSION.split("/").nth(1).expect("failed to retrieve git_date!");
}

pub use color::*;

/// Begone ye NaN's
/// Slerp two `Vec3`s skipping the slerp if their directions are very close
/// This avoids a case where `vek`s slerp produces NaN's
/// Additionally, it avoids unnecessary calculations if they are near identical
/// Assumes `from` is normalized and returns a normalized vector, but `to`
/// doesn't need to be normalized
// TODO: in some cases we might want to base the slerp rate on the magnitude of
// `to` for example when `to` is velocity and `from` is orientation
#[inline(always)]
pub fn safe_slerp(from: vek::Vec3<f32>, to: vek::Vec3<f32>, factor: f32) -> vek::Vec3<f32> {
    use vek::Vec3;

    debug_assert!(!to.map(f32::is_nan).reduce_or());
    debug_assert!(!from.map(f32::is_nan).reduce_or());
    // Ensure from is normalized
    #[cfg(debug_assertions)]
    {
        if {
            let len_sq = from.magnitude_squared();
            len_sq < 0.999 || len_sq > 1.001
        } {
            panic!("Called safe_slerp with unnormalized from: {:?}", from);
        }
    }

    let to = if to.magnitude_squared() > 0.001 {
        to.normalized()
    } else {
        return from;
    };

    let dot = from.dot(to);
    if dot > 0.999 {
        // Close together, just use to
        return to;
    }

    let (from, to, factor) = if dot < -0.999 {
        // Not linearly independent (slerp will fail since it doesn't check for this)
        // Instead we will choose a midpoint and slerp from or to that depending on the
        // factor
        let mid_dir = if from.z > 0.999 {
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
        if {
            let len_sq = slerped_normalized.magnitude_squared();
            len_sq < 0.999 || len_sq > 1.001
        } || slerped_normalized.map(f32::is_nan).reduce_or()
        {
            panic!(
                "Failed to normalize {:?} produced from:\nslerp(\n    {:?},\n    {:?},\n    \
                 {:?},\n)\nWith result: {:?})",
                slerped, from, to, factor, slerped_normalized
            );
        }
    }

    slerped_normalized
}
