use super::{make_bone, vek::*, FigureBoneData, Skeleton};
use common::comp::{self};

pub type Body = comp::object::Body;

#[derive(Clone, Default)]
pub struct ObjectSkeleton;

impl<'a, Factor> Lerp<Factor> for &'a ObjectSkeleton {
    type Output = ObjectSkeleton;

    fn lerp_unclamped_precise(_from: Self, _to: Self, _factor: Factor) -> Self::Output {
        ObjectSkeleton
    }

    fn lerp_unclamped(_from: Self, _to: Self, _factor: Factor) -> Self::Output { ObjectSkeleton }
}

pub struct SkeletonAttr;

const SCALE: f32 = 1.0 / 11.0;

impl Skeleton for ObjectSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 1;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"object_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "object_compute_mats")]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
    ) -> Vec3<f32> {
        buf[0] = make_bone(base_mat * Mat4::scaling_3d(SCALE));
        // TODO: Make dependent on bone, when we find an easier way to make that
        // information available.
        Vec3::unit_z() * 0.5
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self { Self }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(_body: &'a Body) -> Self { Self }
}
