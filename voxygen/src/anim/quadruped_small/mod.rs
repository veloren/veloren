pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{Bone, Skeleton};
use crate::render::FigureBoneData;
use common::comp::{self};
use vek::{Mat4, Vec3};

#[derive(Clone, Default)]
pub struct QuadrupedSmallSkeleton {
    head: Bone,
    chest: Bone,
    leg_lf: Bone,
    leg_rf: Bone,
    leg_lb: Bone,
    leg_rb: Bone,
}

impl QuadrupedSmallSkeleton {
    pub fn new() -> Self { Self::default() }
}

impl Skeleton for QuadrupedSmallSkeleton {
    type Attr = SkeletonAttr;

    fn bone_count(&self) -> usize { 6 }

    fn compute_matrices<F: FnMut(Mat4<f32>) -> FigureBoneData>(
        &self,
        mut make_bone: F,
    ) -> ([FigureBoneData; 16], Vec3<f32>) {
        (
            [
                make_bone(self.head.compute_base_matrix()),
                make_bone(self.chest.compute_base_matrix()),
                make_bone(self.leg_lf.compute_base_matrix()),
                make_bone(self.leg_rf.compute_base_matrix()),
                make_bone(self.leg_lb.compute_base_matrix()),
                make_bone(self.leg_rb.compute_base_matrix()),
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
        self.chest.interpolate(&target.chest, dt);
        self.leg_lf.interpolate(&target.leg_lf, dt);
        self.leg_rf.interpolate(&target.leg_rf, dt);
        self.leg_lb.interpolate(&target.leg_lb, dt);
        self.leg_rb.interpolate(&target.leg_rb, dt);
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    chest: (f32, f32),
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
}
impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::QuadrupedSmall(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            chest: (0.0, 0.0),
            feet_f: (0.0, 0.0, 0.0),
            feet_b: (0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a comp::quadruped_small::Body> for SkeletonAttr {
    fn from(body: &'a comp::quadruped_small::Body) -> Self {
        use comp::quadruped_small::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Pig, _) => (6.0, 7.0),
                (Fox, _) => (8.0, 8.0),
                (Sheep, _) => (8.0, 8.0),
                (Boar, _) => (13.0, 8.0),
                (Jackalope, _) => (6.0, 9.0),
                (Skunk, _) => (7.0, 9.0),
                (Cat, _) => (7.0, 8.0),
                (Batfox, _) => (8.0, 9.0),
                (Raccoon, _) => (9.0, 7.0),
                (Quokka, _) => (10.0, 10.0),
                (Dodarock, _) => (8.0, 9.0),
                (Holladon, _) => (8.0, 8.0),
                (Hyena, _) => (7.5, 13.0),
            },
            chest: match (body.species, body.body_type) {
                (Pig, _) => (0.0, 8.0),
                (Fox, _) => (-2.0, 5.0),
                (Sheep, _) => (-2.0, 6.0),
                (Boar, _) => (-2.0, 7.0),
                (Jackalope, _) => (-2.0, 6.0),
                (Skunk, _) => (-5.0, 6.0),
                (Cat, _) => (-2.0, 6.0),
                (Batfox, _) => (-2.0, 6.0),
                (Raccoon, _) => (-2.0, 6.0),
                (Quokka, _) => (2.0, 8.0),
                (Dodarock, _) => (-2.0, 8.0),
                (Holladon, _) => (-2.0, 6.0),
                (Hyena, _) => (-2.0, 9.0),
            },
            feet_f: match (body.species, body.body_type) {
                (Pig, _) => (3.0, 5.0, 2.0),
                (Fox, _) => (3.0, 5.0, 3.0),
                (Sheep, _) => (3.0, 3.0, 3.0),
                (Boar, _) => (3.0, 5.0, 3.0),
                (Jackalope, _) => (3.0, 5.0, 4.0),
                (Skunk, _) => (3.0, 3.0, 4.0),
                (Cat, _) => (3.0, 5.0, 3.0),
                (Batfox, _) => (2.5, 5.0, 3.0),
                (Raccoon, _) => (3.0, 5.0, 3.0),
                (Quokka, _) => (3.0, 5.0, 3.0),
                (Dodarock, _) => (3.5, 5.0, 4.0),
                (Holladon, _) => (3.0, 5.0, 4.0),
                (Hyena, _) => (2.5, 5.0, 6.0),
            },
            feet_b: match (body.species, body.body_type) {
                (Pig, _) => (3.0, -2.0, 2.0),
                (Fox, _) => (2.5, -2.0, 3.0),
                (Sheep, _) => (3.0, -4.0, 3.0),
                (Boar, _) => (3.0, -3.0, 5.0),
                (Jackalope, _) => (3.0, -2.0, 4.0),
                (Skunk, _) => (3.0, -4.0, 4.0),
                (Cat, _) => (2.0, -2.0, 3.0),
                (Batfox, _) => (2.5, -2.0, 3.0),
                (Raccoon, _) => (3.0, -2.0, 3.0),
                (Quokka, _) => (3.0, -4.0, 3.0),
                (Dodarock, _) => (4.5, -3.0, 4.0),
                (Holladon, _) => (4.0, -4.0, 3.0),
                (Hyena, _) => (2.5, -7.0, 6.0),
            },
        }
    }
}
