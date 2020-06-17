pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{Bone, FigureBoneData, Skeleton};
use common::comp::{self};
use vek::Vec3;

#[derive(Clone, Default)]
pub struct BipedLargeSkeleton {
    head: Bone,
    upper_torso: Bone,
    lower_torso: Bone,
    shoulder_l: Bone,
    shoulder_r: Bone,
    hand_l: Bone,
    hand_r: Bone,
    leg_l: Bone,
    leg_r: Bone,
    foot_l: Bone,
    foot_r: Bone,
    torso: Bone,
}

impl BipedLargeSkeleton {
    pub fn new() -> Self { Self::default() }
}

impl Skeleton for BipedLargeSkeleton {
    type Attr = SkeletonAttr;

    fn bone_count(&self) -> usize { 11 }

    fn compute_matrices(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
        let upper_torso_mat = self.upper_torso.compute_base_matrix();
        let shoulder_l_mat = self.shoulder_l.compute_base_matrix();
        let shoulder_r_mat = self.shoulder_r.compute_base_matrix();
        let leg_l_mat = self.leg_l.compute_base_matrix();
        let leg_r_mat = self.leg_r.compute_base_matrix();
        let torso_mat = self.torso.compute_base_matrix();
        (
            [
                FigureBoneData::new(torso_mat * upper_torso_mat * self.head.compute_base_matrix()),
                FigureBoneData::new(torso_mat * upper_torso_mat),
                FigureBoneData::new(
                    torso_mat * upper_torso_mat * self.lower_torso.compute_base_matrix(),
                ),
                FigureBoneData::new(torso_mat * upper_torso_mat * shoulder_l_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * shoulder_r_mat),
                FigureBoneData::new(
                    torso_mat * upper_torso_mat * self.hand_l.compute_base_matrix(),
                ),
                FigureBoneData::new(
                    torso_mat * upper_torso_mat * self.hand_r.compute_base_matrix(),
                ),
                FigureBoneData::new(torso_mat * upper_torso_mat * leg_l_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * leg_r_mat),
                FigureBoneData::new(self.foot_l.compute_base_matrix()),
                FigureBoneData::new(self.foot_r.compute_base_matrix()),
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
        self.upper_torso.interpolate(&target.upper_torso, dt);
        self.lower_torso.interpolate(&target.lower_torso, dt);
        self.shoulder_l.interpolate(&target.shoulder_l, dt);
        self.shoulder_r.interpolate(&target.shoulder_r, dt);
        self.hand_l.interpolate(&target.hand_l, dt);
        self.hand_r.interpolate(&target.hand_r, dt);
        self.leg_l.interpolate(&target.leg_l, dt);
        self.leg_r.interpolate(&target.leg_r, dt);
        self.foot_l.interpolate(&target.foot_l, dt);
        self.foot_r.interpolate(&target.foot_r, dt);
        self.torso.interpolate(&target.torso, dt);
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    upper_torso: (f32, f32),
    lower_torso: (f32, f32),
    shoulder: (f32, f32, f32),
    hand: (f32, f32, f32),
    leg: (f32, f32, f32),
    foot: (f32, f32, f32),
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::BipedLarge(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            upper_torso: (0.0, 0.0),
            lower_torso: (0.0, 0.0),
            shoulder: (0.0, 0.0, 0.0),
            hand: (0.0, 0.0, 0.0),
            leg: (0.0, 0.0, 0.0),
            foot: (0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a comp::biped_large::Body> for SkeletonAttr {
    fn from(body: &'a comp::biped_large::Body) -> Self {
        use comp::biped_large::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Ogre, _) => (3.0, 6.0),
                (Cyclops, _) => (3.0, 9.0),
            },
            upper_torso: match (body.species, body.body_type) {
                (Ogre, _) => (0.0, 19.0),
                (Cyclops, _) => (-1.0, 27.0),
            },
            lower_torso: match (body.species, body.body_type) {
                (Ogre, _) => (1.0, -9.5),
                (Cyclops, _) => (1.0, -10.5),
            },
            shoulder: match (body.species, body.body_type) {
                (Ogre, _) => (6.1, 0.5, 2.5),
                (Cyclops, _) => (9.5, 0.5, 2.5),
            },
            hand: match (body.species, body.body_type) {
                (Ogre, _) => (10.5, -1.0, -0.5),
                (Cyclops, _) => (10.5, 0.0, -0.5),
            },
            leg: match (body.species, body.body_type) {
                (Ogre, _) => (0.0, 0.0, -6.0),
                (Cyclops, _) => (0.0, 0.0, -9.0),
            },
            foot: match (body.species, body.body_type) {
                (Ogre, _) => (4.0, 0.5, 5.5),
                (Cyclops, _) => (4.0, 0.5, 5.0),
            },
        }
    }
}
