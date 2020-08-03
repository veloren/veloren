pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{vek::Vec3, Bone, FigureBoneData, Skeleton};
use common::comp::{self};

#[derive(Clone)]
pub struct BirdSmallSkeleton {
    head: Bone,
    torso: Bone,
    wing_l: Bone,
    wing_r: Bone,
}

impl BirdSmallSkeleton {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self {
            head: Bone::default(),
            torso: Bone::default(),
            wing_l: Bone::default(),
            wing_r: Bone::default(),
        }
    }
}

impl Skeleton for BirdSmallSkeleton {
    type Attr = SkeletonAttr;

    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"bird_small_compute_mats\0";

    fn bone_count(&self) -> usize { 4 }

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_small_compute_mats")]

    fn compute_matrices_inner(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
        let torso_mat = self.torso.compute_base_matrix();

        (
            [
                FigureBoneData::new(self.head.compute_base_matrix() * torso_mat),
                FigureBoneData::new(torso_mat),
                FigureBoneData::new(self.wing_l.compute_base_matrix() * torso_mat),
                FigureBoneData::new(self.wing_r.compute_base_matrix() * torso_mat),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
            ],
            Vec3::default(),
        )
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.head.interpolate(&target.head, dt);
        self.torso.interpolate(&target.torso, dt);
        self.wing_l.interpolate(&target.wing_l, dt);
        self.wing_r.interpolate(&target.wing_r, dt);
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

impl<'a> From<&'a comp::bird_small::Body> for SkeletonAttr {
    fn from(_body: &'a comp::bird_small::Body) -> Self { Self }
}
