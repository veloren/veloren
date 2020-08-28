pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{make_bone, vek::*, FigureBoneData, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::bird_small::Body;

skeleton_impls!(struct BirdSmallSkeleton {
    + head,
    + torso,
    + wing_l,
    + wing_r,
});

impl Skeleton for BirdSmallSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 4;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"bird_small_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_small_compute_mats")]

    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
    ) -> Vec3<f32> {
        let torso_mat = base_mat * Mat4::<f32>::from(self.torso);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(torso_mat * Mat4::<f32>::from(self.head)),
            make_bone(torso_mat),
            make_bone(torso_mat * Mat4::<f32>::from(self.wing_l)),
            make_bone(torso_mat * Mat4::<f32>::from(self.wing_r)),
        ];
        Vec3::default()
    }
}

pub struct SkeletonAttr;

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::BirdSmall(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self { Self }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(_body: &'a Body) -> Self { Self }
}
