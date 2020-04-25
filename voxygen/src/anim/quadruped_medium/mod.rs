pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{Bone, Skeleton};
use crate::render::FigureBoneData;
use common::comp::{self};

#[derive(Clone, Default)]
pub struct QuadrupedMediumSkeleton {
    head_upper: Bone,
    head_lower: Bone,
    jaw: Bone,
    tail: Bone,
    torso_back: Bone,
    torso_mid: Bone,
    ears: Bone,
    foot_lf: Bone,
    foot_rf: Bone,
    foot_lb: Bone,
    foot_rb: Bone,
}

impl QuadrupedMediumSkeleton {
    pub fn new() -> Self { Self::default() }
}

impl Skeleton for QuadrupedMediumSkeleton {
    type Attr = SkeletonAttr;

    fn bone_count(&self) -> usize { 11 }

    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let ears_mat = self.ears.compute_base_matrix();
        let head_upper_mat = self.head_upper.compute_base_matrix();
        let head_lower_mat = self.head_lower.compute_base_matrix();
        let torso_mid_mat = self.torso_mid.compute_base_matrix();
        [
            FigureBoneData::new(head_upper_mat),
            FigureBoneData::new(head_upper_mat * head_lower_mat),
            FigureBoneData::new(head_upper_mat * self.jaw.compute_base_matrix()),
            FigureBoneData::new(torso_mid_mat * self.tail.compute_base_matrix()),
            FigureBoneData::new(self.torso_back.compute_base_matrix()),
            FigureBoneData::new(torso_mid_mat),
            FigureBoneData::new(head_upper_mat * ears_mat),
            FigureBoneData::new(self.foot_lf.compute_base_matrix()),
            FigureBoneData::new(self.foot_rf.compute_base_matrix()),
            FigureBoneData::new(self.foot_lb.compute_base_matrix()),
            FigureBoneData::new(self.foot_rb.compute_base_matrix()),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.head_upper.interpolate(&target.head_upper, dt);
        self.head_lower.interpolate(&target.head_lower, dt);
        self.jaw.interpolate(&target.jaw, dt);
        self.tail.interpolate(&target.tail, dt);
        self.torso_back.interpolate(&target.torso_back, dt);
        self.torso_mid.interpolate(&target.torso_mid, dt);
        self.ears.interpolate(&target.ears, dt);
        self.foot_lf.interpolate(&target.foot_lf, dt);
        self.foot_rf.interpolate(&target.foot_rf, dt);
        self.foot_lb.interpolate(&target.foot_lb, dt);
        self.foot_rb.interpolate(&target.foot_rb, dt);
    }
}

pub struct SkeletonAttr {
    head_upper: (f32, f32),
    head_lower: (f32, f32),
    jaw: (f32, f32),
    tail: (f32, f32),
    torso_back: (f32, f32),
    torso_mid: (f32, f32),
    ears: (f32, f32),
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
    height: f32,
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::QuadrupedMedium(body) => Ok(SkeletonAttr::from(body)),
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
            tail: (0.0, 0.0),
            torso_back: (0.0, 0.0),
            torso_mid: (0.0, 0.0),
            ears: (0.0, 0.0),
            feet_f: (0.0, 0.0, 0.0),
            feet_b: (0.0, 0.0, 0.0),
            height: (0.0),
        }
    }
}

impl<'a> From<&'a comp::quadruped_medium::Body> for SkeletonAttr {
    fn from(body: &'a comp::quadruped_medium::Body) -> Self {
        use comp::quadruped_medium::Species::*;
        Self {
            head_upper: match (body.species, body.body_type) {
                (Wolf, _) => (12.0, 16.0),
                (Saber, _) => (14.0, 12.0),
                (Viper, _) => (14.0, 10.0),
                (Tuskram, _) => (9.0, 12.0),
                (Alligator, _) => (16.0, 11.0),
                (Monitor, _) => (14.0, 6.0),
                (Lion, _) => (12.5, 14.0),
                (Tarasque, _) => (12.0, 19.0),
            },
            head_lower: match (body.species, body.body_type) {
                (Wolf, _) => (-4.0, -7.0),
                (Saber, _) => (-6.0, 0.0),
                (Viper, _) => (-3.0, -1.0),
                (Tuskram, _) => (-3.0, -1.0),
                (Alligator, _) => (-5.0, -4.0),
                (Monitor, _) => (-3.0, -1.0),
                (Lion, _) => (-5.0, -1.0),
                (Tarasque, _) => (-5.0, -6.0),
            },
            jaw: match (body.species, body.body_type) {
                (Wolf, _) => (3.0, -5.0),
                (Saber, _) => (2.0, -1.0),
                (Viper, _) => (3.0, -2.0),
                (Tuskram, _) => (2.0, -2.0),
                (Alligator, _) => (6.0, -6.0),
                (Monitor, _) => (4.0, -3.0),
                (Lion, _) => (2.0, -3.0),
                (Tarasque, _) => (4.0, -9.0),
            },
            tail: match (body.species, body.body_type) {
                (Wolf, _) => (-6.0, -2.0),
                (Saber, _) => (-4.0, -2.0),
                (Viper, _) => (-6.0, -1.0),
                (Tuskram, _) => (-6.0, -2.0),
                (Alligator, _) => (-7.0, -1.0),
                (Monitor, _) => (-7.0, -1.0),
                (Lion, _) => (-8.0, -6.0),
                (Tarasque, _) => (-7.0, -2.0),
            },
            torso_back: match (body.species, body.body_type) {
                (Wolf, _) => (4.0, 11.0),
                (Saber, _) => (4.0, 9.0),
                (Viper, _) => (4.0, 7.0),
                (Tuskram, _) => (4.0, 9.0),
                (Alligator, _) => (4.0, 6.0),
                (Monitor, _) => (4.0, 4.0),
                (Lion, _) => (4.0, 10.0),
                (Tarasque, _) => (4.0, 9.0),
            },
            torso_mid: match (body.species, body.body_type) {
                (Wolf, _) => (-7.0, 10.5),
                (Saber, _) => (-7.0, 9.5),
                (Viper, _) => (-7.0, 7.0),
                (Tuskram, _) => (-7.0, 9.0),
                (Alligator, _) => (-7.0, 6.0),
                (Monitor, _) => (-7.0, 4.0),
                (Lion, _) => (-9.0, 9.0),
                (Tarasque, _) => (-7.0, 8.0),
            },
            ears: match (body.species, body.body_type) {
                (Wolf, _) => (-1.0, 5.0),
                (Saber, _) => (-1.0, 6.0),
                (Viper, _) => (10.0, 2.0),
                (Tuskram, _) => (10.0, 2.0),
                (Alligator, _) => (10.0, 2.0),
                (Monitor, _) => (10.0, 2.0),
                (Lion, _) => (-2.0, 4.0),
                (Tarasque, _) => (1.5, -2.0),
            },
            feet_f: match (body.species, body.body_type) {
                (Wolf, _) => (5.0, 6.0, 2.0),
                (Saber, _) => (4.0, 6.0, 3.0),
                (Viper, _) => (4.0, 6.0, 3.0),
                (Tuskram, _) => (4.0, 6.0, 4.5),
                (Alligator, _) => (4.0, 6.0, 3.0),
                (Monitor, _) => (4.0, 6.0, 3.0),
                (Lion, _) => (5.0, 6.0, 3.0),
                (Tarasque, _) => (4.0, 6.0, 3.0),
            },
            feet_b: match (body.species, body.body_type) {
                (Wolf, _) => (5.0, -4.0, 3.0),
                (Saber, _) => (4.0, -6.0, 3.5),
                (Viper, _) => (4.0, -4.0, 3.5),
                (Tuskram, _) => (4.0, -8.0, 5.5),
                (Alligator, _) => (4.0, -4.0, 3.5),
                (Monitor, _) => (4.0, -6.0, 3.5),
                (Lion, _) => (5.5, -8.0, 3.5),
                (Tarasque, _) => (4.0, -8.0, 3.5),
            },
            height: match (body.species, body.body_type) {
                (Wolf, _) => (1.2),
                (Saber, _) => (1.0),
                (Viper, _) => (0.7),
                (Tuskram, _) => (1.0),
                (Alligator, _) => (0.5),
                (Monitor, _) => (0.4),
                (Lion, _) => (1.4),
                (Tarasque, _) => (1.1),
            },
        }
    }
}
