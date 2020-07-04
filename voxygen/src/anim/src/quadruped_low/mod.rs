pub mod alpha;
pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{
    alpha::AlphaAnimation, idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation,
};

use super::{Bone, FigureBoneData, Skeleton};
use common::comp::{self};
use vek::Vec3;

#[derive(Clone, Default)]
pub struct QuadrupedLowSkeleton {
    head_upper: Bone,
    head_lower: Bone,
    jaw: Bone,
    chest: Bone,
    tail_front: Bone,
    tail_rear: Bone,
    foot_fl: Bone,
    foot_fr: Bone,
    foot_bl: Bone,
    foot_br: Bone,
}

impl QuadrupedLowSkeleton {
    pub fn new() -> Self { Self::default() }
}

impl Skeleton for QuadrupedLowSkeleton {
    type Attr = SkeletonAttr;

    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"quadruped_low_compute_mats\0";

    fn bone_count(&self) -> usize { 10 }

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_compute_mats")]
    fn compute_matrices_inner(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
        let head_upper_mat = self.head_upper.compute_base_matrix();
        let head_lower_mat = self.head_lower.compute_base_matrix();
        let chest_mat = self.chest.compute_base_matrix();
        (
            [
                FigureBoneData::new(chest_mat * head_lower_mat * head_upper_mat),
                FigureBoneData::new(chest_mat * head_lower_mat),
                FigureBoneData::new(
                    chest_mat * head_lower_mat * head_upper_mat * self.jaw.compute_base_matrix(),
                ),
                FigureBoneData::new(chest_mat),
                FigureBoneData::new(chest_mat * self.tail_front.compute_base_matrix()),
                FigureBoneData::new(
                    chest_mat
                        * self.tail_front.compute_base_matrix()
                        * self.tail_rear.compute_base_matrix(),
                ),
                FigureBoneData::new(chest_mat * self.foot_fl.compute_base_matrix()),
                FigureBoneData::new(chest_mat * self.foot_fr.compute_base_matrix()),
                FigureBoneData::new(chest_mat * self.foot_bl.compute_base_matrix()),
                FigureBoneData::new(chest_mat * self.foot_br.compute_base_matrix()),
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
        self.head_upper.interpolate(&target.head_upper, dt);
        self.head_lower.interpolate(&target.head_lower, dt);
        self.jaw.interpolate(&target.jaw, dt);
        self.chest.interpolate(&target.chest, dt);
        self.tail_front.interpolate(&target.tail_front, dt);
        self.tail_rear.interpolate(&target.tail_rear, dt);
        self.foot_fl.interpolate(&target.foot_fl, dt);
        self.foot_fr.interpolate(&target.foot_fr, dt);
        self.foot_bl.interpolate(&target.foot_bl, dt);
        self.foot_br.interpolate(&target.foot_br, dt);
    }
}

pub struct SkeletonAttr {
    head_upper: (f32, f32),
    head_lower: (f32, f32),
    jaw: (f32, f32),
    chest: (f32, f32),
    tail_front: (f32, f32),
    tail_rear: (f32, f32),
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
    lean: (f32, f32),
    scaler: f32,
    tempo: f32,
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::QuadrupedLow(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head_upper: (0.0, 0.0),
            head_lower: (0.0, 0.0),
            jaw: (0.0, 0.0),
            chest: (0.0, 0.0),
            tail_front: (0.0, 0.0),
            tail_rear: (0.0, 0.0),
            feet_f: (0.0, 0.0, 0.0),
            feet_b: (0.0, 0.0, 0.0),
            lean: (0.0, 0.0),
            scaler: 0.0,
            tempo: 0.0,
        }
    }
}

impl<'a> From<&'a comp::quadruped_low::Body> for SkeletonAttr {
    fn from(body: &'a comp::quadruped_low::Body) -> Self {
        use comp::quadruped_low::Species::*;
        Self {
            head_upper: match (body.species, body.body_type) {
                (Crocodile, _) => (10.0, 2.0),
                (Alligator, _) => (0.5, 2.0),
                (Salamander, _) => (6.5, 2.5),
                (Monitor, _) => (4.5, 1.0),
                (Asp, _) => (6.0, 5.5),
                (Tortoise, _) => (5.0, 1.0),
                (Rocksnapper, _) => (6.0, 0.5),
                (Pangolin, _) => (-0.5, 8.0),
            },
            head_lower: match (body.species, body.body_type) {
                (Crocodile, _) => (8.0, 0.0),
                (Alligator, _) => (9.0, 0.25),
                (Salamander, _) => (9.0, 0.0),
                (Monitor, _) => (10.0, 2.0),
                (Asp, _) => (9.0, 2.5),
                (Tortoise, _) => (12.0, -3.5),
                (Rocksnapper, _) => (12.0, -9.0),
                (Pangolin, _) => (8.0, -9.0),
            },
            jaw: match (body.species, body.body_type) {
                (Crocodile, _) => (-6.0, -3.0),
                (Alligator, _) => (2.5, -2.0),
                (Salamander, _) => (-6.0, -2.0),
                (Monitor, _) => (-2.0, -1.0),
                (Asp, _) => (-3.0, -2.0),
                (Tortoise, _) => (-3.5, -2.0),
                (Rocksnapper, _) => (-5.0, -1.5),
                (Pangolin, _) => (0.0, 0.0),
            },
            chest: match (body.species, body.body_type) {
                (Crocodile, _) => (0.0, 5.0),
                (Alligator, _) => (0.0, 5.0),
                (Salamander, _) => (0.0, 5.0),
                (Monitor, _) => (0.0, 5.0),
                (Asp, _) => (0.0, 8.0),
                (Tortoise, _) => (0.0, 11.0),
                (Rocksnapper, _) => (0.0, 18.5),
                (Pangolin, _) => (0.0, 7.0),
            },
            tail_rear: match (body.species, body.body_type) {
                (Crocodile, _) => (-12.5, -1.0),
                (Alligator, _) => (-13.0, -1.0),
                (Salamander, _) => (-8.0, 0.0),
                (Monitor, _) => (-12.0, 0.0),
                (Asp, _) => (-14.0, -2.0),
                (Tortoise, _) => (-10.0, -1.5),
                (Rocksnapper, _) => (-14.5, -2.0),
                (Pangolin, _) => (-7.0, -3.0),
            },
            tail_front: match (body.species, body.body_type) {
                (Crocodile, _) => (-6.0, 0.0),
                (Alligator, _) => (-5.0, 0.0),
                (Salamander, _) => (-7.5, 0.0),
                (Monitor, _) => (-6.5, 0.0),
                (Asp, _) => (-6.0, -2.0),
                (Tortoise, _) => (-13.0, -3.5),
                (Rocksnapper, _) => (-13.5, -6.5),
                (Pangolin, _) => (-7.5, -0.5),
            },
            feet_f: match (body.species, body.body_type) {
                (Crocodile, _) => (3.5, 6.0, -1.0),
                (Alligator, _) => (4.5, 4.25, -1.0),
                (Salamander, _) => (5.0, 5.0, -2.0),
                (Monitor, _) => (3.0, 5.0, 0.0),
                (Asp, _) => (1.5, 4.0, -1.0),
                (Tortoise, _) => (5.5, 6.5, -3.0),
                (Rocksnapper, _) => (7.5, 5.0, -8.5),
                (Pangolin, _) => (5.0, 5.0, -1.0),
            },
            feet_b: match (body.species, body.body_type) {
                (Crocodile, _) => (3.5, -6.0, -1.0),
                (Alligator, _) => (4.5, -5.5, -1.0),
                (Salamander, _) => (4.0, -6.0, -2.0),
                (Monitor, _) => (2.5, -6.5, 0.0),
                (Asp, _) => (2.5, -5.5, -1.0),
                (Tortoise, _) => (6.0, -11.5, -3.0),
                (Rocksnapper, _) => (8.0, -12.0, -9.5),
                (Pangolin, _) => (6.0, -4.0, -1.0),
            },
            lean: match (body.species, body.body_type) {
                (Pangolin, _) => (0.4, 0.0),
                _ => (0.0, 1.0),
            },
            scaler: match (body.species, body.body_type) {
                (Crocodile, _) => (1.3),
                (Alligator, _) => (1.5),
                (Salamander, _) => (1.4),
                (Monitor, _) => (1.1),
                (Asp, _) => (1.4),
                (Tortoise, _) => (1.0),
                (Rocksnapper, _) => (1.4),
                (Pangolin, _) => (1.3),
            },
            tempo: match (body.species, body.body_type) {
                (Crocodile, _) => (0.8),
                (Alligator, _) => (0.8),
                (Salamander, _) => (1.0),
                (Monitor, _) => (1.3),
                (Asp, _) => (1.0),
                (Tortoise, _) => (0.9),
                (Rocksnapper, _) => (0.9),
                (Pangolin, _) => (1.15),
            },
        }
    }
}
