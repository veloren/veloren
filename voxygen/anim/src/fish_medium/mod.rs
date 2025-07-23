pub mod idle;
pub mod swim;

// Reexports
pub use self::{idle::IdleAnimation, swim::SwimAnimation};

use super::{FigureBoneData, Skeleton, vek::*};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::fish_medium::Body;

skeleton_impls!(struct FishMediumSkeleton ComputedFishMediumSkeleton {
    + head
    + jaw
    + chest_front
    + chest_back
    + tail
    + fin_l
    + fin_r
});

impl Skeleton for FishMediumSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;
    type ComputedSkeleton = ComputedFishMediumSkeleton;

    const BONE_COUNT: usize = ComputedFishMediumSkeleton::BONE_COUNT;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"fish_medium_compute_mats\0";

    #[cfg_attr(
        feature = "be-dyn-lib",
        unsafe(export_name = "fish_medium_compute_mats")
    )]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        _body: Self::Body,
    ) -> Self::ComputedSkeleton {
        let base_mat = base_mat * Mat4::scaling_3d(1.0 / 11.0);

        let chest_front_mat = base_mat * Mat4::<f32>::from(self.chest_front);
        let chest_back_mat = Mat4::<f32>::from(self.chest_back);
        let head_mat = chest_front_mat * Mat4::<f32>::from(self.head);

        let computed_skeleton = ComputedFishMediumSkeleton {
            head: head_mat,
            jaw: head_mat * Mat4::<f32>::from(self.jaw),
            chest_front: chest_front_mat,
            chest_back: chest_front_mat * chest_back_mat,
            tail: chest_front_mat * chest_back_mat * Mat4::<f32>::from(self.tail),
            fin_l: chest_front_mat * Mat4::<f32>::from(self.fin_l),
            fin_r: chest_front_mat * Mat4::<f32>::from(self.fin_r),
        };

        computed_skeleton.set_figure_bone_data(buf);
        computed_skeleton
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    jaw: (f32, f32),
    chest_front: (f32, f32),
    chest_back: (f32, f32),
    tail: (f32, f32),
    fin: (f32, f32, f32),
    tempo: f32,
    amplitude: f32,
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::FishMedium(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            jaw: (0.0, 0.0),
            chest_front: (0.0, 0.0),
            chest_back: (0.0, 0.0),
            tail: (0.0, 0.0),
            fin: (0.0, 0.0, 0.0),
            tempo: 0.0,
            amplitude: 0.0,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::fish_medium::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Marlin, _) => (2.0, 1.5),
                (Icepike, _) => (3.0, 1.0),
            },
            jaw: match (body.species, body.body_type) {
                (Marlin, _) => (2.5, -3.0),
                (Icepike, _) => (0.0, 0.0),
            },
            chest_front: match (body.species, body.body_type) {
                (Marlin, _) => (0.0, 2.5),
                (Icepike, _) => (0.0, 2.5),
            },
            chest_back: match (body.species, body.body_type) {
                (Marlin, _) => (-1.0, 1.0),
                (Icepike, _) => (-4.5, 0.0),
            },
            tail: match (body.species, body.body_type) {
                (Marlin, _) => (-7.0, 0.0),
                (Icepike, _) => (-0.5, 1.5),
            },
            fin: match (body.species, body.body_type) {
                (Marlin, _) => (2.5, 1.0, 3.5),
                (Icepike, _) => (3.5, 3.0, 0.0),
            },
            tempo: match (body.species, body.body_type) {
                (Marlin, _) => 4.0,
                (Icepike, _) => 4.0,
            },
            amplitude: match (body.species, body.body_type) {
                (Marlin, _) => 4.0,
                (Icepike, _) => 4.0,
            },
        }
    }
}

pub fn mount_mat(
    computed_skeleton: &ComputedFishMediumSkeleton,
    skeleton: &FishMediumSkeleton,
) -> (Mat4<f32>, Quaternion<f32>) {
    (computed_skeleton.head, skeleton.head.orientation)
}

pub fn mount_transform(
    body: &Body,
    computed_skeleton: &ComputedFishMediumSkeleton,
    skeleton: &FishMediumSkeleton,
) -> Transform<f32, f32, f32> {
    use comp::fish_medium::Species::*;

    let mount_point = match (body.species, body.body_type) {
        (Marlin, _) => (0.0, 0.5, 3.0),
        (Icepike, _) => (0.0, 0.5, 4.0),
    }
    .into();

    let (mount_mat, orientation) = mount_mat(computed_skeleton, skeleton);
    Transform {
        position: mount_mat.mul_point(mount_point),
        orientation,
        scale: Vec3::one(),
    }
}
