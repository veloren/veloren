pub mod alpha;
pub mod beam;
pub mod combomelee;
pub mod idle;
pub mod run;
pub mod shockwave;
pub mod shoot;

// Reexports
pub use self::{
    alpha::AlphaAnimation, beam::BeamAnimation, combomelee::ComboAnimation, idle::IdleAnimation,
    run::RunAnimation, shockwave::ShockwaveAnimation, shoot::ShootAnimation,
};

use super::{FigureBoneData, Skeleton, vek::*};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::golem::Body;

skeleton_impls!(struct GolemSkeleton ComputedGolemSkeleton {
    + head
    + jaw
    + upper_torso
    + lower_torso
    + shoulder_l
    + shoulder_r
    + hand_l
    + hand_r
    + leg_l
    + leg_r
    + foot_l
    + foot_r
    torso
});

impl Skeleton for GolemSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;
    type ComputedSkeleton = ComputedGolemSkeleton;

    const BONE_COUNT: usize = ComputedGolemSkeleton::BONE_COUNT;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"golem_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "golem_compute_mats"))]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Self::ComputedSkeleton {
        let base_mat = base_mat * Mat4::scaling_3d(SkeletonAttr::from(&body).scaler / 8.0);

        let torso_mat = base_mat * Mat4::<f32>::from(self.torso);
        let upper_torso_mat = torso_mat * Mat4::<f32>::from(self.upper_torso);
        let lower_torso_mat = upper_torso_mat * Mat4::<f32>::from(self.lower_torso);
        let leg_l_mat = lower_torso_mat * Mat4::<f32>::from(self.leg_l);
        let leg_r_mat = lower_torso_mat * Mat4::<f32>::from(self.leg_r);
        let shoulder_l_mat = upper_torso_mat * Mat4::<f32>::from(self.shoulder_l);
        let shoulder_r_mat = upper_torso_mat * Mat4::<f32>::from(self.shoulder_r);
        let head_mat = upper_torso_mat * Mat4::<f32>::from(self.head);

        let computed_skeleton = ComputedGolemSkeleton {
            head: head_mat,
            jaw: upper_torso_mat * Mat4::<f32>::from(self.head) * Mat4::<f32>::from(self.jaw),
            upper_torso: upper_torso_mat,
            lower_torso: lower_torso_mat,
            shoulder_l: upper_torso_mat * Mat4::<f32>::from(self.shoulder_l),
            shoulder_r: upper_torso_mat * Mat4::<f32>::from(self.shoulder_r),
            hand_l: shoulder_l_mat * Mat4::<f32>::from(self.hand_l),
            hand_r: shoulder_r_mat * Mat4::<f32>::from(self.hand_r),
            leg_l: leg_l_mat,
            leg_r: leg_r_mat,
            foot_l: leg_l_mat * Mat4::<f32>::from(self.foot_l),
            foot_r: leg_r_mat * Mat4::<f32>::from(self.foot_r),
        };

        computed_skeleton.set_figure_bone_data(buf);
        computed_skeleton
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    jaw: (f32, f32),
    upper_torso: (f32, f32),
    lower_torso: (f32, f32),
    shoulder: (f32, f32, f32),
    hand: (f32, f32, f32),
    leg: (f32, f32, f32),
    foot: (f32, f32, f32),
    scaler: f32,
    tempo: f32,
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
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
            jaw: (0.0, 0.0),
            upper_torso: (0.0, 0.0),
            lower_torso: (0.0, 0.0),
            shoulder: (0.0, 0.0, 0.0),
            hand: (0.0, 0.0, 0.0),
            leg: (0.0, 0.0, 0.0),
            foot: (0.0, 0.0, 0.0),
            scaler: 0.0,
            tempo: 0.0,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::golem::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (StoneGolem, _) => (0.0, 2.0),
                (Treant, _) => (18.0, -8.0),
                (ClayGolem, _) => (0.0, 7.0),
                (WoodGolem, _) => (3.0, 6.0),
                (CoralGolem, _) => (-1.0, 3.0),
                (Gravewarden, _) => (-2.0, 7.0),
                (AncientEffigy, _) => (-2.0, 8.0),
                (Mogwai, _) => (-8.0, 2.0),
                (IronGolem, _) => (0.0, 3.0),
            },
            jaw: match (body.species, body.body_type) {
                (StoneGolem, _) => (0.0, 0.0),
                (Treant, _) => (-6.5, -1.0),
                (ClayGolem, _) => (0.0, 0.0),
                (WoodGolem, _) => (0.0, 0.0),
                (CoralGolem, _) => (0.0, 0.0),
                (Gravewarden, _) => (0.0, 0.0),
                (AncientEffigy, _) => (0.0, 0.0),
                (Mogwai, _) => (-6.0, -5.0),
                (IronGolem, _) => (0.0, 0.0),
            },
            upper_torso: match (body.species, body.body_type) {
                (StoneGolem, _) => (0.0, 34.5),
                (Treant, _) => (0.0, 28.5),
                (ClayGolem, _) => (0.0, 23.0),
                (WoodGolem, _) => (0.0, 24.5),
                (CoralGolem, _) => (0.0, 25.0),
                (Gravewarden, _) => (0.0, 26.5),
                (AncientEffigy, _) => (0.0, 18.0),
                (Mogwai, _) => (0.0, 18.0),
                (IronGolem, _) => (0.0, 34.5),
            },
            lower_torso: match (body.species, body.body_type) {
                (StoneGolem, _) => (0.0, -10.5),
                (Treant, _) => (0.0, -10.5),
                (ClayGolem, _) => (0.0, -7.5),
                (WoodGolem, _) => (0.0, -4.5),
                (CoralGolem, _) => (0.0, -11.5),
                (Gravewarden, _) => (0.0, -4.5),
                (AncientEffigy, _) => (0.0, -4.5),
                (Mogwai, _) => (0.0, -4.5),
                (IronGolem, _) => (0.0, -10.5),
            },
            shoulder: match (body.species, body.body_type) {
                (StoneGolem, _) => (8.0, -1.5, 4.0),
                (Treant, _) => (8.0, 4.5, -3.0),
                (ClayGolem, _) => (8.0, -1.0, -1.0),
                (WoodGolem, _) => (6.0, 2.0, 1.0),
                (CoralGolem, _) => (11.0, 1.0, 0.0),
                (Gravewarden, _) => (8.0, 2.0, 3.0),
                (AncientEffigy, _) => (8.0, 2.0, 3.0),
                (Mogwai, _) => (8.0, 2.0, 3.0),
                (IronGolem, _) => (8.0, -1.5, 0.0),
            },
            hand: match (body.species, body.body_type) {
                (StoneGolem, _) => (12.5, -1.0, -7.0),
                (Treant, _) => (8.5, -1.0, -7.0),
                (ClayGolem, _) => (6.5, 0.0, -2.0),
                (WoodGolem, _) => (5.5, -1.0, -6.0),
                (CoralGolem, _) => (2.5, -1.5, -5.0),
                (Gravewarden, _) => (8.5, -1.0, -7.0),
                (AncientEffigy, _) => (8.5, -1.0, -7.0),
                (Mogwai, _) => (8.5, -1.0, -7.0),
                (IronGolem, _) => (12.5, -1.0, -7.0),
            },
            leg: match (body.species, body.body_type) {
                (StoneGolem, _) => (4.0, 0.0, -3.5),
                (Treant, _) => (2.0, 9.5, -1.0),
                (ClayGolem, _) => (1.0, 0.0, -3.0),
                (WoodGolem, _) => (2.0, 0.5, -6.0),
                (CoralGolem, _) => (2.5, 0.5, -3.0),
                (Gravewarden, _) => (1.0, 0.5, -6.0),
                (AncientEffigy, _) => (1.0, 0.5, -6.0),
                (Mogwai, _) => (1.0, 0.5, -6.0),
                (IronGolem, _) => (1.5, 1.5, -3.5),
            },
            foot: match (body.species, body.body_type) {
                (StoneGolem, _) => (3.5, 0.5, -9.5),
                (Treant, _) => (3.5, -5.0, -8.5),
                (ClayGolem, _) => (3.5, 0.0, -5.5),
                (WoodGolem, _) => (2.5, 1.0, -5.5),
                (CoralGolem, _) => (2.5, 1.0, -1.5),
                (Gravewarden, _) => (3.5, -1.0, -8.5),
                (AncientEffigy, _) => (3.5, -1.0, -8.5),
                (Mogwai, _) => (3.5, -1.0, -8.5),
                (IronGolem, _) => (3.5, 0.5, -9.5),
            },
            scaler: match (body.species, body.body_type) {
                (StoneGolem, _) => 1.5,
                (Treant, _) => 1.5,
                (ClayGolem, _) => 1.5,
                (WoodGolem, _) => 1.5,
                (CoralGolem, _) => 1.0,
                (Gravewarden, _) => 1.5,
                (AncientEffigy, _) => 1.0,
                (Mogwai, _) => 1.0,
                (IronGolem, _) => 1.5,
            },
            tempo: match (body.species, body.body_type) {
                (StoneGolem, _) => 1.0,
                (Treant, _) => 1.0,
                (ClayGolem, _) => 1.0,
                (WoodGolem, _) => 1.0,
                (CoralGolem, _) => 1.0,
                (Gravewarden, _) => 1.0,
                (AncientEffigy, _) => 1.0,
                (Mogwai, _) => 1.0,
                (IronGolem, _) => 1.0,
            },
        }
    }
}

pub fn mount_mat(
    computed_skeleton: &ComputedGolemSkeleton,
    skeleton: &GolemSkeleton,
) -> (Mat4<f32>, Quaternion<f32>) {
    (
        computed_skeleton.head,
        skeleton.torso.orientation * skeleton.upper_torso.orientation * skeleton.head.orientation,
    )
}

pub fn mount_transform(
    body: &Body,
    computed_skeleton: &ComputedGolemSkeleton,
    skeleton: &GolemSkeleton,
) -> Transform<f32, f32, f32> {
    use comp::golem::Species::*;

    let mount_point = match (body.species, body.body_type) {
        (StoneGolem, _) => (0.0, 0.5, 10.0),
        (Treant, _) => (0.0, 0.0, 14.0),
        (ClayGolem, _) => (0.0, 0.0, 12.0),
        (WoodGolem, _) => (0.0, 0.0, 8.0),
        (CoralGolem, _) => (0.0, 0.0, 5.0),
        (Gravewarden, _) => (0.0, -0.5, 7.0),
        (AncientEffigy, _) => (0.0, -0.5, 4.0),
        (Mogwai, _) => (0.0, 11.0, 10.5),
        (IronGolem, _) => (0.0, 0.0, 17.0),
    }
    .into();

    let (mount_mat, orientation) = mount_mat(computed_skeleton, skeleton);
    Transform {
        position: mount_mat.mul_point(mount_point),
        orientation,
        scale: Vec3::one(),
    }
}
