pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{Bone, FigureBoneData, Skeleton};
use common::comp::{self};
use vek::Vec3;

#[derive(Clone, Default)]
pub struct QuadrupedSmallSkeleton {
    head: Bone,
    chest: Bone,
    leg_fl: Bone,
    leg_fr: Bone,
    leg_bl: Bone,
    leg_br: Bone,
    tail: Bone,
}

impl QuadrupedSmallSkeleton {
    pub fn new() -> Self { Self::default() }
}

impl Skeleton for QuadrupedSmallSkeleton {
    type Attr = SkeletonAttr;

    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"quadruped_small_compute_mats\0";

    fn bone_count(&self) -> usize { 7 }

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_small_compute_mats")]
    fn compute_matrices_inner(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
        let chest_mat = self.chest.compute_base_matrix();
        (
            [
                FigureBoneData::new(chest_mat * self.head.compute_base_matrix()),
                FigureBoneData::new(chest_mat),
                FigureBoneData::new(chest_mat * self.leg_fl.compute_base_matrix()),
                FigureBoneData::new(chest_mat * self.leg_fr.compute_base_matrix()),
                FigureBoneData::new(chest_mat * self.leg_bl.compute_base_matrix()),
                FigureBoneData::new(chest_mat * self.leg_br.compute_base_matrix()),
                FigureBoneData::new(chest_mat * self.tail.compute_base_matrix()),
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
        self.leg_fl.interpolate(&target.leg_fl, dt);
        self.leg_fr.interpolate(&target.leg_fr, dt);
        self.leg_bl.interpolate(&target.leg_bl, dt);
        self.leg_br.interpolate(&target.leg_br, dt);
        self.tail.interpolate(&target.tail, dt);
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    chest: (f32, f32),
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
    tail: (f32, f32),
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
            tail: (0.0, 0.0),
        }
    }
}

impl<'a> From<&'a comp::quadruped_small::Body> for SkeletonAttr {
    fn from(body: &'a comp::quadruped_small::Body) -> Self {
        use comp::quadruped_small::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Pig, _) => (5.0, 2.0),
                (Fox, _) => (7.0, 3.0),
                (Sheep, _) => (9.0, 3.0),
                (Boar, _) => (12.0, 3.0),
                (Jackalope, _) => (5.0, 4.0),
                (Skunk, _) => (6.0, 4.0),
                (Cat, _) => (6.0, 3.0),
                (Batfox, _) => (7.0, 4.0),
                (Raccoon, _) => (8.0, 2.0),
                (Quokka, _) => (9.0, 5.0),
                (Dodarock, _) => (7.0, 4.0),
                (Holladon, _) => (7.0, 3.0),
                (Hyena, _) => (7.5, 2.0),
            },
            chest: match (body.species, body.body_type) {
                (Pig, _) => (0.0, 7.0),
                (Fox, _) => (0.0, 5.0),
                (Sheep, _) => (-1.0, 6.0),
                (Boar, _) => (0.0, 8.5),
                (Jackalope, _) => (-2.0, 6.0),
                (Skunk, _) => (0.0, 6.0),
                (Cat, _) => (0.0, 6.0),
                (Batfox, _) => (-2.0, 6.0),
                (Raccoon, _) => (0.0, 6.0),
                (Quokka, _) => (2.0, 8.0),
                (Dodarock, _) => (-2.0, 8.0),
                (Holladon, _) => (-2.0, 6.0),
                (Hyena, _) => (-2.0, 9.0),
            },
            feet_f: match (body.species, body.body_type) {
                (Pig, _) => (3.5, 3.0, -5.5),
                (Fox, _) => (2.5, 3.0, -2.0),
                (Sheep, _) => (3.5, 2.0, -2.5),
                (Boar, _) => (3.5, 4.0, -4.5),
                (Jackalope, _) => (2.0, 4.0, -2.0),
                (Skunk, _) => (2.0, 2.0, -2.0),
                (Cat, _) => (2.0, 4.0, -3.0),
                (Batfox, _) => (1.5, 4.0, -3.0),
                (Raccoon, _) => (2.0, 4.0, -3.0),
                (Quokka, _) => (2.0, 4.0, -3.0),
                (Dodarock, _) => (2.5, 4.0, -2.0),
                (Holladon, _) => (2.0, 4.0, -2.0),
                (Hyena, _) => (2.5, 4.0, -4.0),
            },
            feet_b: match (body.species, body.body_type) {
                (Pig, _) => (2.0, -2.0, -5.5),
                (Fox, _) => (1.5, -2.0, -1.0),
                (Sheep, _) => (3.5, -4.0, -2.5),
                (Boar, _) => (2.0, -3.0, -2.5),
                (Jackalope, _) => (2.0, -2.0, 0.0),
                (Skunk, _) => (1.0, -4.0, -2.5),
                (Cat, _) => (1.5, -2.0, -3.0),
                (Batfox, _) => (2.0, -2.0, -2.5),
                (Raccoon, _) => (2.5, -2.0, -3.5),
                (Quokka, _) => (2.5, -4.0, -3.5),
                (Dodarock, _) => (2.0, -3.0, -5.5),
                (Holladon, _) => (3.5, -2.0, -3.5),
                (Hyena, _) => (3.0, -5.0, -4.5),
            },
            tail: match (body.species, body.body_type) {
                (Pig, _) => (-4.0, 3.0),
                (Fox, _) => (-3.5, 1.0),
                (Sheep, _) => (-5.0, 0.0),
                (Boar, _) => (-8.5, 2.0),
                (Jackalope, _) => (0.0, 5.0),
                (Skunk, _) => (-3.0, 1.5),
                (Cat, _) => (-3.0, 2.0),
                (Batfox, _) => (0.0, 5.0),
                (Raccoon, _) => (-4.0, 1.0),
                (Quokka, _) => (0.0, 6.0),
                (Dodarock, _) => (0.0, 5.0),
                (Holladon, _) => (0.0, 4.0),
                (Hyena, _) => (-8.0, 1.0),
            },
        }
    }
}
