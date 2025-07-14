pub mod idle;
pub mod swim;

// Reexports
pub use self::{idle::IdleAnimation, swim::SwimAnimation};

use super::{FigureBoneData, Skeleton, vek::*};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::fish_small::Body;

skeleton_impls!(struct FishSmallSkeleton ComputedFishSmallSkeleton {
    + chest
    + tail
    + fin_l
    + fin_r
});

impl Skeleton for FishSmallSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;
    type ComputedSkeleton = ComputedFishSmallSkeleton;

    const BONE_COUNT: usize = ComputedFishSmallSkeleton::BONE_COUNT;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"fish_small_compute_mats\0";

    #[cfg_attr(
        feature = "be-dyn-lib",
        unsafe(export_name = "fish_small_compute_mats")
    )]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        _body: Self::Body,
    ) -> Self::ComputedSkeleton {
        let base_mat = base_mat * Mat4::scaling_3d(1.0 / 13.0);
        let chest_mat = base_mat * Mat4::<f32>::from(self.chest);

        let computed_skeleton = ComputedFishSmallSkeleton {
            chest: chest_mat,
            tail: chest_mat * Mat4::<f32>::from(self.tail),
            fin_l: chest_mat * Mat4::<f32>::from(self.fin_l),
            fin_r: chest_mat * Mat4::<f32>::from(self.fin_r),
        };

        computed_skeleton.set_figure_bone_data(buf);
        computed_skeleton
    }
}

pub struct SkeletonAttr {
    chest: (f32, f32),
    tail: (f32, f32),
    fin: (f32, f32, f32),
    tempo: f32,
    amplitude: f32,
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::FishSmall(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            chest: (0.0, 0.0),
            tail: (0.0, 0.0),
            fin: (0.0, 0.0, 0.0),
            tempo: 0.0,
            amplitude: 0.0,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::fish_small::Species::*;
        Self {
            chest: match (body.species, body.body_type) {
                (Clownfish, _) => (0.0, 5.0),
                (Piranha, _) => (0.0, 5.0),
            },
            tail: match (body.species, body.body_type) {
                (Clownfish, _) => (-7.5, -0.5),
                (Piranha, _) => (-5.5, -0.5),
            },
            fin: match (body.species, body.body_type) {
                (Clownfish, _) => (2.0, 0.5, 1.0),
                (Piranha, _) => (2.0, 0.5, -0.5),
            },
            tempo: match (body.species, body.body_type) {
                (Clownfish, _) => 5.0,
                (Piranha, _) => 5.0,
            },
            amplitude: match (body.species, body.body_type) {
                (Clownfish, _) => 4.0,
                (Piranha, _) => 4.0,
            },
        }
    }
}

pub fn mount_mat(
    computed_skeleton: &ComputedFishSmallSkeleton,
    skeleton: &FishSmallSkeleton,
) -> (Mat4<f32>, Quaternion<f32>) {
    (computed_skeleton.chest, skeleton.chest.orientation)
}

pub fn mount_transform(
    body: &Body,
    computed_skeleton: &ComputedFishSmallSkeleton,
    skeleton: &FishSmallSkeleton,
) -> Transform<f32, f32, f32> {
    use comp::fish_small::Species::*;

    let mount_point = match (body.species, body.body_type) {
        (Clownfish, _) => (0.0, 0.5, 3.0),
        (Piranha, _) => (0.0, -1.0, 4.5),
    }
    .into();

    let (mount_mat, orientation) = mount_mat(computed_skeleton, skeleton);
    Transform {
        position: mount_mat.mul_point(mount_point),
        orientation,
        scale: Vec3::one(),
    }
}
