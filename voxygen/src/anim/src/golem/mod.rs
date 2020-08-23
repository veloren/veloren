pub mod alpha;
pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{
    alpha::AlphaAnimation, idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation,
};

use super::{make_bone, vek::*, FigureBoneData, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::golem::Body;

skeleton_impls!(struct GolemSkeleton {
    + head,
    + upper_torso,
    + lower_torso,
    + shoulder_l,
    + shoulder_r,
    + hand_l,
    + hand_r,
    + leg_l,
    + leg_r,
    + foot_l,
    + foot_r,
    torso,
});

impl Skeleton for GolemSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 11;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"golem_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "golem_compute_mats")]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
    ) -> Vec3<f32> {
        let torso_mat = base_mat * Mat4::<f32>::from(self.torso);
        let upper_torso_mat = torso_mat * Mat4::<f32>::from(self.upper_torso);
        let lower_torso_mat = upper_torso_mat * Mat4::<f32>::from(self.lower_torso);
        let leg_l_mat = lower_torso_mat * Mat4::<f32>::from(self.leg_l);
        let leg_r_mat = lower_torso_mat * Mat4::<f32>::from(self.leg_r);
        let shoulder_l_mat = upper_torso_mat * Mat4::<f32>::from(self.shoulder_l);
        let shoulder_r_mat = upper_torso_mat * Mat4::<f32>::from(self.shoulder_r);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(upper_torso_mat * Mat4::<f32>::from(self.head)),
            make_bone(upper_torso_mat),
            make_bone(lower_torso_mat),
            make_bone(upper_torso_mat * Mat4::<f32>::from(self.shoulder_l)),
            make_bone(upper_torso_mat * Mat4::<f32>::from(self.shoulder_r)),
            make_bone(shoulder_l_mat * Mat4::<f32>::from(self.hand_l)),
            make_bone(shoulder_r_mat * Mat4::<f32>::from(self.hand_r)),
            make_bone(leg_l_mat),
            make_bone(leg_r_mat),
            make_bone(leg_l_mat * Mat4::<f32>::from(self.foot_l)),
            make_bone(leg_r_mat * Mat4::<f32>::from(self.foot_r)),
        ];
        Vec3::default()
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
            lower_torso: (0.0, 0.0),
            shoulder: (0.0, 0.0, 0.0),
            hand: (0.0, 0.0, 0.0),
            leg: (0.0, 0.0, 0.0),
            foot: (0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::golem::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (StoneGolem, _) => (0.0, 2.0),
            },
            upper_torso: match (body.species, body.body_type) {
                (StoneGolem, _) => (0.0, 34.5),
            },
            lower_torso: match (body.species, body.body_type) {
                (StoneGolem, _) => (0.0, -10.5),
            },
            shoulder: match (body.species, body.body_type) {
                (StoneGolem, _) => (8.0, -1.5, 4.0),
            },
            hand: match (body.species, body.body_type) {
                (StoneGolem, _) => (12.5, -1.0, -7.0),
            },
            leg: match (body.species, body.body_type) {
                (StoneGolem, _) => (4.0, 0.0, -3.5),
            },
            foot: match (body.species, body.body_type) {
                (StoneGolem, _) => (3.5, 0.5, -9.5),
            },
        }
    }
}
