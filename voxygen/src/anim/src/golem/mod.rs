pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{Bone, FigureBoneData, Skeleton};
use common::comp::{self};
use vek::Vec3;

#[derive(Clone, Default)]
pub struct GolemSkeleton {
    head: Bone,
    upper_torso: Bone,
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

impl GolemSkeleton {
    pub fn new() -> Self { Self::default() }
}

impl Skeleton for GolemSkeleton {
    type Attr = SkeletonAttr;

    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"golem_compute_mats\0";

    fn bone_count(&self) -> usize { 15 }

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_compute_mats")]
    fn compute_matrices_inner(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
        let upper_torso_mat = self.upper_torso.compute_base_matrix();
        let shoulder_l_mat = self.shoulder_l.compute_base_matrix();
        let shoulder_r_mat = self.shoulder_r.compute_base_matrix();
        let leg_l_mat = self.leg_l.compute_base_matrix();
        let leg_r_mat = self.leg_r.compute_base_matrix();
        let torso_mat = self.torso.compute_base_matrix();
        let foot_l_mat = self.foot_l.compute_base_matrix();
        let foot_r_mat = self.foot_r.compute_base_matrix();
        (
            [
                FigureBoneData::new(torso_mat * upper_torso_mat * self.head.compute_base_matrix()),
                FigureBoneData::new(torso_mat * upper_torso_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * shoulder_l_mat),
                FigureBoneData::new(torso_mat * upper_torso_mat * shoulder_r_mat),
                FigureBoneData::new(
                    torso_mat * upper_torso_mat * self.hand_l.compute_base_matrix(),
                ),
                FigureBoneData::new(
                    torso_mat * upper_torso_mat * self.hand_r.compute_base_matrix(),
                ),
                FigureBoneData::new(foot_l_mat * leg_l_mat),
                FigureBoneData::new(foot_r_mat * leg_r_mat),
                FigureBoneData::new(foot_l_mat),
                FigureBoneData::new(foot_r_mat),
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
        self.upper_torso.interpolate(&target.upper_torso, dt);
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
    shoulder: (f32, f32, f32),
    hand: (f32, f32, f32),
    leg: (f32, f32, f32),
    foot: (f32, f32, f32),
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::Golem(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            upper_torso: (0.0, 0.0),
            shoulder: (0.0, 0.0, 0.0),
            hand: (0.0, 0.0, 0.0),
            leg: (0.0, 0.0, 0.0),
            foot: (0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a comp::golem::Body> for SkeletonAttr {
    fn from(body: &'a comp::golem::Body) -> Self {
        use comp::golem::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (StoneGolem, _) => (0.0, 16.0),
            },
            upper_torso: match (body.species, body.body_type) {
                (StoneGolem, _) => (0.0, 33.0),
            },
            shoulder: match (body.species, body.body_type) {
                (StoneGolem, _) => (8.0, -0.5, 7.5),
            },
            hand: match (body.species, body.body_type) {
                (StoneGolem, _) => (9.5, -1.0, 4.5),
            },
            leg: match (body.species, body.body_type) {
                (StoneGolem, _) => (-1.0, 0.0, 9.0),
            },
            foot: match (body.species, body.body_type) {
                (StoneGolem, _) => (4.0, 0.5, 11.0),
            },
        }
    }
}
