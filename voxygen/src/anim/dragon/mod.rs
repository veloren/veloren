pub mod fly;
pub mod idle;
pub mod run;

// Reexports
pub use self::{fly::FlyAnimation, idle::IdleAnimation, run::RunAnimation};

use super::{Bone, Skeleton};
use crate::render::FigureBoneData;
<<<<<<< HEAD
use common::comp::{self};
use vek::Vec3;

#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const HEAD_UPPER_X: f32 = 2.5;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const HEAD_UPPER_Z: f32 = 4.5;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const HEAD_LOWER_X: f32 = 7.5;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const HEAD_LOWER_Z: f32 = 3.5;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const JAW_X: f32 = 7.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const JAW_Z: f32 = -5.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const CHEST_F_X: f32 = 0.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const CHEST_F_Z: f32 = 14.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const CHEST_R_X: f32 = -12.5;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const CHEST_R_Z: f32 = 0.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const TAIL_F_X: f32 = -12.5;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const TAIL_F_Z: f32 = 1.5;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const TAIL_R_X: f32 = -14.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const TAIL_R_Z: f32 = 0.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const WING_IN_X: f32 = 2.5;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const WING_IN_Y: f32 = -16.5;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const WING_IN_Z: f32 = 0.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const WING_OUT_X: f32 = 23.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const WING_OUT_Y: f32 = 0.5;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const WING_OUT_Z: f32 = 4.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const FEET_F_X: f32 = 6.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const FEET_F_Y: f32 = 0.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const FEET_F_Z: f32 = 1.5;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const FEET_B_X: f32 = 6.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const FEET_B_Y: f32 = -15.0;
#[const_tweaker::tweak(min = -40.0, max = 40.0, step = 0.5)]
const FEET_B_Z: f32 = 3.0;   
=======
use common::comp::{self}; 
>>>>>>> Cleanup

#[derive(Clone, Default)]
pub struct DragonSkeleton {
    head_upper: Bone,
    head_lower: Bone,
    jaw: Bone,
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

    fn bone_count(&self) -> usize { 15 }

<<<<<<< HEAD
    fn compute_matrices(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
=======
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let head_upper_mat = self.head_upper.compute_base_matrix();
        let head_lower_mat = self.head_lower.compute_base_matrix();
>>>>>>> New dragon model, added jaw, splitted head into upper/lower
        let chest_front_mat = self.chest_front.compute_base_matrix();
        let chest_rear_mat = self.chest_rear.compute_base_matrix();
        let wing_in_l_mat = self.wing_in_l.compute_base_matrix();
        let wing_in_r_mat = self.wing_in_r.compute_base_matrix();
        let tail_front_mat = self.tail_front.compute_base_matrix();

<<<<<<< HEAD
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
=======
        [
            FigureBoneData::new(chest_front_mat * head_lower_mat * head_upper_mat),
            FigureBoneData::new(chest_front_mat * head_lower_mat),
            FigureBoneData::new(chest_front_mat * head_lower_mat * head_upper_mat * self.jaw.compute_base_matrix()),
            FigureBoneData::new(chest_front_mat),
            FigureBoneData::new(chest_front_mat * self.chest_rear.compute_base_matrix() ),
            FigureBoneData::new(chest_front_mat * chest_rear_mat * tail_front_mat),
            FigureBoneData::new(chest_front_mat * chest_rear_mat * tail_front_mat * self.tail_rear.compute_base_matrix()),
            FigureBoneData::new(chest_front_mat * self.wing_in_l.compute_base_matrix()),
            FigureBoneData::new(chest_front_mat * self.wing_in_r.compute_base_matrix()),
            FigureBoneData::new(chest_front_mat * wing_in_l_mat * self.wing_out_l.compute_base_matrix()),
            FigureBoneData::new(chest_front_mat * wing_in_r_mat * self.wing_out_r.compute_base_matrix()),
            FigureBoneData::new(self.foot_fl.compute_base_matrix()),
            FigureBoneData::new(self.foot_fr.compute_base_matrix()),
            FigureBoneData::new(self.foot_bl.compute_base_matrix()),
            FigureBoneData::new(self.foot_br.compute_base_matrix()),
            FigureBoneData::default(),
        ]
>>>>>>> Symmetry of dragon skeleton
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.head_upper.interpolate(&target.head_upper, dt);
        self.head_lower.interpolate(&target.head_lower, dt);
        self.jaw.interpolate(&target.jaw, dt);
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
    head_upper: (f32, f32),
    head_lower: (f32, f32),
    jaw: (f32, f32),
    chest_front: (f32, f32),
    chest_rear: (f32, f32),
    tail_front: (f32, f32),
    tail_rear: (f32, f32),
    wing_in: (f32, f32, f32),
    wing_out: (f32, f32, f32),
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
    height: f32,
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
            head_upper: (0.0, 0.0),
            head_lower: (0.0, 0.0),
            jaw: (0.0, 0.0),
            chest_front: (0.0, 0.0),
            chest_rear: (0.0, 0.0),
            tail_front: (0.0, 0.0),
            tail_rear: (0.0, 0.0),
            wing_in: (0.0, 0.0, 0.0),
            wing_out: (0.0, 0.0, 0.0),
            feet_f: (0.0, 0.0, 0.0),
            feet_b: (0.0, 0.0, 0.0),
            height: (0.0),
        }
    }
}

impl<'a> From<&'a comp::dragon::Body> for SkeletonAttr {
    fn from(body: &'a comp::dragon::Body) -> Self { 
        use comp::dragon::Species::*;
        Self {
            head_upper: match (body.species, body.body_type) {
                (Reddragon, _) => (2.5, 4.5),
            },
            head_lower: match (body.species, body.body_type) {
                (Reddragon, _) => (7.5, 3.5),
            },
            jaw: match (body.species, body.body_type) {
                (Reddragon, _) => (7.0, -5.0),
            },
            chest_front: match (body.species, body.body_type) {
                (Reddragon, _) => (0.0, 14.0),
            },
            chest_rear: match (body.species, body.body_type) {
                (Reddragon, _) => (-12.5, 0.0),
            },
            tail_front: match (body.species, body.body_type) {
                (Reddragon, _) => (-6.5, 1.5),
            },
            tail_rear: match (body.species, body.body_type) {
                (Reddragon, _) => (-11.5, -1.0),
            },
            wing_in: match (body.species, body.body_type) {
                (Reddragon, _) => (2.5, -16.5, 0.0),
            },
            wing_out: match (body.species, body.body_type) {
                (Reddragon, _) => (23.0, 0.5, 4.0),
            },
            feet_f: match (body.species, body.body_type) {
                (Reddragon, _) => (6.0, 0.0, 1.5),
            },
            feet_b: match (body.species, body.body_type) {
                (Reddragon, _) => (6.0, -15.0, 3.0),
            },
            height: match (body.species, body.body_type) {
                (Reddragon, _) => (1.0),
            },
        }
    }
}