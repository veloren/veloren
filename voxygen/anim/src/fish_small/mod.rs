pub mod idle;
pub mod jump;
pub mod run;
pub mod swim;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation, swim::SwimAnimation};

use super::{make_bone, vek::*, FigureBoneData, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::fish_small::Body;

skeleton_impls!(struct FishSmallSkeleton {
    + head,
    + chest,
    + tail,
    + fin_l,
    + fin_r,
});

impl Skeleton for FishSmallSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 5;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"fish_small_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "fish_small_compute_mats")]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
    ) -> Vec3<f32> {
        let chest_mat = base_mat * Mat4::<f32>::from(self.chest);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(chest_mat * Mat4::<f32>::from(self.head)),
            make_bone(chest_mat),
            make_bone(chest_mat * Mat4::<f32>::from(self.tail)),
            make_bone(chest_mat * Mat4::<f32>::from(self.fin_l)),
            make_bone(chest_mat * Mat4::<f32>::from(self.fin_r)),
        ];
        Vec3::default()
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    chest: (f32, f32),
    tail: (f32, f32),
    fin: (f32, f32, f32),
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
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
            head: (0.0, 0.0),
            chest: (0.0, 0.0),
            tail: (0.0, 0.0),
            fin: (0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::fish_small::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Sardine, _) => (1.5, 2.0),
            },
            chest: match (body.species, body.body_type) {
                (Sardine, _) => (0.0, 5.0),
            },
            tail: match (body.species, body.body_type) {
                (Sardine, _) => (-6.0, 0.0),
            },
            fin: match (body.species, body.body_type) {
                (Sardine, _) => (3.5, 6.0, -1.0),
            },
        }
    }
}
