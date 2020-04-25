pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{Bone, Skeleton};
use crate::render::FigureBoneData;
use common::comp::{self};
use vek::Vec3;

#[derive(Clone, Default)]
pub struct DragonSkeleton {
    head: Bone,
    chest_front: Bone,
    chest_rear: Bone,
    tail_front: Bone,
    tail_rear: Bone,
    wing_in_l: Bone,
    wing_in_r: Bone,
    wing_out_l: Bone,
    wing_out_r: Bone,
    foot_fl: Bone,
    foot_fr: Bone,
    foot_bl: Bone,
    foot_br: Bone,
}

impl DragonSkeleton {
    pub fn new() -> Self { Self::default() }
}

impl Skeleton for DragonSkeleton {
    type Attr = SkeletonAttr;

    fn bone_count(&self) -> usize { 13 }

    fn compute_matrices(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
        let chest_front_mat = self.chest_front.compute_base_matrix();
        let wing_in_l_mat = self.wing_in_l.compute_base_matrix();
        let wing_in_r_mat = self.wing_in_r.compute_base_matrix();
        let tail_front_mat = self.tail_front.compute_base_matrix();

        (
            [
                FigureBoneData::new(self.head.compute_base_matrix() * chest_front_mat),
                FigureBoneData::new(chest_front_mat),
                FigureBoneData::new(self.chest_rear.compute_base_matrix() * chest_front_mat),
                FigureBoneData::new(tail_front_mat),
                FigureBoneData::new(self.tail_rear.compute_base_matrix() * tail_front_mat),
                FigureBoneData::new(wing_in_l_mat),
                FigureBoneData::new(wing_in_r_mat),
                FigureBoneData::new(self.wing_out_l.compute_base_matrix() * wing_in_l_mat),
                FigureBoneData::new(self.wing_out_r.compute_base_matrix() * wing_in_r_mat),
                FigureBoneData::new(self.foot_fl.compute_base_matrix()),
                FigureBoneData::new(self.foot_fr.compute_base_matrix()),
                FigureBoneData::new(self.foot_bl.compute_base_matrix()),
                FigureBoneData::new(self.foot_br.compute_base_matrix()),
                FigureBoneData::default(),
                FigureBoneData::default(),
                FigureBoneData::default(),
            ],
            Vec3::default(),
        )
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.head.interpolate(&target.head, dt);
        self.chest_front.interpolate(&target.chest_front, dt);
        self.chest_rear.interpolate(&target.chest_rear, dt);
        self.tail_front.interpolate(&target.tail_front, dt);
        self.tail_rear.interpolate(&target.tail_rear, dt);
        self.wing_in_l.interpolate(&target.wing_in_l, dt);
        self.wing_in_r.interpolate(&target.wing_in_r, dt);
        self.wing_out_l.interpolate(&target.wing_out_l, dt);
        self.wing_out_r.interpolate(&target.wing_out_r, dt);
        self.foot_fl.interpolate(&target.foot_fl, dt);
        self.foot_fr.interpolate(&target.foot_fr, dt);
        self.foot_bl.interpolate(&target.foot_bl, dt);
        self.foot_br.interpolate(&target.foot_br, dt);
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    chest_front: (f32, f32),
    chest_rear: (f32, f32),
    tail_front: (f32, f32),
    tail_rear: (f32, f32),
    wing_in: (f32, f32),
    wing_out: (f32, f32),
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::Dragon(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            chest_front: (0.0, 0.0),
            chest_rear: (0.0, 0.0),
            tail_front: (0.0, 0.0),
            tail_rear: (0.0, 0.0),
            wing_in: (0.0, 0.0),
            wing_out: (0.0, 0.0),
            feet_f: (0.0, 0.0, 0.0),
            feet_b: (0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a comp::dragon::Body> for SkeletonAttr {
    fn from(body: &'a comp::dragon::Body) -> Self { 
        use comp::dragon::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Reddragon, _) => (4.0, 3.0),
            },
            chest_front: match (body.species, body.body_type) {
                (Reddragon, _) => (0.0, 5.0),
            },
            chest_rear: match (body.species, body.body_type) {
                (Reddragon, _) => (0.0, 5.0),
            },
            tail_front: match (body.species, body.body_type) {
                (Reddragon, _) => (-3.0, 1.5),
            },
            tail_rear: match (body.species, body.body_type) {
                (Reddragon, _) => (-3.0, 1.5),
            },
            wing_in: match (body.species, body.body_type) {
                (Reddragon, _) => (2.75, 0.0),
            },
            wing_out: match (body.species, body.body_type) {
                (Reddragon, _) => (2.75, 0.0),
            },
            feet_f: match (body.species, body.body_type) {
                (Reddragon, _) => (2.0, -1.5, 4.0),
            },
            feet_b: match (body.species, body.body_type) {
                (Reddragon, _) => (2.0, -1.5, 4.0),
            },
        }
    }
}