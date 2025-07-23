pub mod beam;
pub mod idle;
pub mod shoot;

// Reexports
pub use self::{beam::BeamAnimation, idle::IdleAnimation, shoot::ShootAnimation};

use super::{FigureBoneData, Skeleton, vek::*};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::object::Body;

skeleton_impls!(struct ObjectSkeleton ComputedObjectSkeleton {
    + bone0
    + bone1
});

impl Skeleton for ObjectSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;
    type ComputedSkeleton = ComputedObjectSkeleton;

    const BONE_COUNT: usize = ComputedObjectSkeleton::BONE_COUNT;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"object_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "object_compute_mats"))]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        _body: Self::Body,
    ) -> Self::ComputedSkeleton {
        let scale_mat = Mat4::scaling_3d(1.0 / 11.0);

        let bone0_mat = base_mat * scale_mat * Mat4::<f32>::from(self.bone0);

        let computed_skeleton = ComputedObjectSkeleton {
            bone0: bone0_mat,
            bone1: scale_mat * Mat4::<f32>::from(self.bone1), /* Decorellated from ori */
        };

        computed_skeleton.set_figure_bone_data(buf);
        computed_skeleton
    }
}

pub struct SkeletonAttr {
    bone0: (f32, f32, f32),
    bone1: (f32, f32, f32),
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::Object(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            bone0: (0.0, 0.0, 0.0),
            bone1: (0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::object::Body::*;
        Self {
            bone0: match body {
                Crossbow => (0.0, 0.0, 11.0),
                Flamethrower => (0.0, 0.0, 11.0),
                HaniwaSentry => (0.0, 0.0, 10.5),
                _ => (0.0, 0.0, 0.0),
            },
            bone1: match body {
                Crossbow => (0.0, 0.0, 8.0),
                Flamethrower => (0.0, 0.0, 8.0),
                HaniwaSentry => (0.0, 0.0, 3.0),
                _ => (0.0, 0.0, 0.0),
            },
        }
    }
}
