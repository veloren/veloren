pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::idle::IdleAnimation;
pub use self::jump::JumpAnimation;
pub use self::run::RunAnimation;

use super::{Bone, Skeleton};
use crate::render::FigureBoneData;
use common::comp::{self};

#[derive(Clone)]
pub struct FishSmallSkeleton {
    torso: Bone,
    tail: Bone,
}

impl FishSmallSkeleton {
    pub fn new() -> Self {
        Self {
            torso: Bone::default(),
            tail: Bone::default(),
        }
    }
}

impl Skeleton for FishSmallSkeleton {
    type Attr = SkeletonAttr;
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let torso_mat = self.torso.compute_base_matrix();

        [
            FigureBoneData::new(torso_mat),
            FigureBoneData::new(self.tail.compute_base_matrix() * torso_mat),
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
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.torso.interpolate(&target.torso, dt);
        self.tail.interpolate(&target.tail, dt);
    }
}

pub struct SkeletonAttr;

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
        Self
    }
}

impl<'a> From<&'a comp::fish_small::Body> for SkeletonAttr {
    fn from(_body: &'a comp::fish_small::Body) -> Self {
        Self
    }
}
