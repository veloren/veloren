pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{Bone, Skeleton};
use crate::render::FigureBoneData;
use common::comp::{self};

#[derive(Clone)]
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
    pub fn new() -> Self {
        Self {
            head: Bone::default(),
            chest_front: Bone::default(),
            chest_rear: Bone::default(),
            tail_front: Bone::default(),
            tail_rear: Bone::default(),
            wing_in_l: Bone::default(),
            wing_in_r: Bone::default(),
            wing_out_l: Bone::default(),
            wing_out_r: Bone::default(),
            foot_fl: Bone::default(),
            foot_fr: Bone::default(),
            foot_bl: Bone::default(),
            foot_br: Bone::default(),
        }
    }
}

impl Skeleton for DragonSkeleton {
    type Attr = SkeletonAttr;

    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let chest_front_mat = self.chest_front.compute_base_matrix();
        let wing_in_l_mat = self.wing_in_l.compute_base_matrix();
        let wing_in_r_mat = self.wing_in_r.compute_base_matrix();
        let tail_front_mat = self.tail_front.compute_base_matrix();

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
        ]
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

pub struct SkeletonAttr;

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
    fn default() -> Self { Self }
}

impl<'a> From<&'a comp::dragon::Body> for SkeletonAttr {
    fn from(_body: &'a comp::dragon::Body) -> Self { Self }
}
