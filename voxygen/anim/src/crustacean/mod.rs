mod alpha;
mod combomelee;
mod idle;
mod jump;
mod leapmelee;
mod ripostemelee;
mod run;
mod stunned;
mod summon;
mod swim;

// Reexports
pub use self::{
    alpha::AlphaAnimation, combomelee::ComboAnimation, idle::IdleAnimation, jump::JumpAnimation,
    leapmelee::LeapMeleeAnimation, ripostemelee::RiposteMeleeAnimation, run::RunAnimation,
    stunned::StunnedAnimation, summon::SummonAnimation, swim::SwimAnimation,
};

use common::comp::{self};

use super::{FigureBoneData, Skeleton, vek::*};

pub type Body = comp::crustacean::Body;

skeleton_impls!(struct CrustaceanSkeleton ComputedCrustaceanSkeleton {
    + chest
    + tail_f
    + tail_b
    + arm_l
    + pincer_l0
    + pincer_l1
    + arm_r
    + pincer_r0
    + pincer_r1
    + leg_fl
    + leg_cl
    + leg_bl
    + leg_fr
    + leg_cr
    + leg_br
});

impl Skeleton for CrustaceanSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;
    type ComputedSkeleton = ComputedCrustaceanSkeleton;

    const BONE_COUNT: usize = ComputedCrustaceanSkeleton::BONE_COUNT;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"crustacean_compute_s\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "crustacean_compute_s"))]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Self::ComputedSkeleton {
        let base_mat = base_mat * Mat4::scaling_3d(SkeletonAttr::from(&body).scaler / 6.0);

        let chest_mat = base_mat * Mat4::<f32>::from(self.chest);
        let tail_f_mat = chest_mat * Mat4::<f32>::from(self.tail_f);
        let tail_b_mat = chest_mat * Mat4::<f32>::from(self.tail_b);
        let arm_l_mat = chest_mat * Mat4::<f32>::from(self.arm_l);
        let pincer_l0_mat = arm_l_mat * Mat4::<f32>::from(self.pincer_l0);
        let pincer_l1_mat = pincer_l0_mat * Mat4::<f32>::from(self.pincer_l1);
        let arm_r_mat = chest_mat * Mat4::<f32>::from(self.arm_r);
        let pincer_r0_mat = arm_r_mat * Mat4::<f32>::from(self.pincer_r0);
        let pincer_r1_mat = pincer_r0_mat * Mat4::<f32>::from(self.pincer_r1);
        let leg_fl_mat = chest_mat * Mat4::<f32>::from(self.leg_fl);
        let leg_cl_mat = chest_mat * Mat4::<f32>::from(self.leg_cl);
        let leg_bl_mat = chest_mat * Mat4::<f32>::from(self.leg_bl);
        let leg_fr_mat = chest_mat * Mat4::<f32>::from(self.leg_fr);
        let leg_cr_mat = chest_mat * Mat4::<f32>::from(self.leg_cr);
        let leg_br_mat = chest_mat * Mat4::<f32>::from(self.leg_br);

        let computed_skeleton = ComputedCrustaceanSkeleton {
            chest: chest_mat,
            tail_f: tail_f_mat,
            tail_b: tail_b_mat,
            arm_l: arm_l_mat,
            pincer_l0: pincer_l0_mat,
            pincer_l1: pincer_l1_mat,
            arm_r: arm_r_mat,
            pincer_r0: pincer_r0_mat,
            pincer_r1: pincer_r1_mat,
            leg_fl: leg_fl_mat,
            leg_cl: leg_cl_mat,
            leg_bl: leg_bl_mat,
            leg_fr: leg_fr_mat,
            leg_cr: leg_cr_mat,
            leg_br: leg_br_mat,
        };

        computed_skeleton.set_figure_bone_data(buf);
        computed_skeleton
    }
}

pub struct SkeletonAttr {
    chest: (f32, f32),
    arm: (f32, f32, f32),
    leg_f: (f32, f32, f32),
    leg_c: (f32, f32, f32),
    leg_b: (f32, f32, f32),
    leg_ori: (f32, f32, f32),
    move_sideways: bool,
    scaler: f32,
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::crustacean::Species::*;
        Self {
            chest: match (body.species, body.body_type) {
                (Crab, _) => (0.0, 0.0),
                (SoldierCrab, _) => (0.0, 0.0),
                (Karkatha, _) => (0.0, 0.0),
            },
            arm: match (body.species, body.body_type) {
                (Crab, _) => (0.0, 5.0, 0.0),
                (SoldierCrab, _) => (0.0, 5.0, 0.0),
                (Karkatha, _) => (0.0, 0.0, 0.0),
            },
            leg_f: match (body.species, body.body_type) {
                (Crab, _) => (0.0, 0.0, 0.0),
                (SoldierCrab, _) => (0.0, 0.0, 0.0),
                (Karkatha, _) => (3.0, 0.0, 0.0),
            },
            leg_c: match (body.species, body.body_type) {
                (Crab, _) => (0.0, 0.0, 0.0),
                (SoldierCrab, _) => (0.0, 0.0, 0.0),
                (Karkatha, _) => (0.0, 0.0, 0.0),
            },
            leg_b: match (body.species, body.body_type) {
                (Crab, _) => (0.0, 0.0, 0.0),
                (SoldierCrab, _) => (0.0, 0.0, 0.0),
                (Karkatha, _) => (0.0, 0.0, 0.0),
            },
            leg_ori: match (body.species, body.body_type) {
                (Crab, _) => (-0.4, 0.0, 0.4),
                (SoldierCrab, _) => (-0.4, 0.0, 0.4),
                (Karkatha, _) => (-0.4, 0.0, 0.4),
            },
            move_sideways: match (body.species, body.body_type) {
                (Crab, _) => true,
                (SoldierCrab, _) => true,
                (Karkatha, _) => false,
            },
            scaler: match (body.species, body.body_type) {
                (Crab, _) => 0.62,
                (SoldierCrab, _) => 0.62,
                (Karkatha, _) => 1.2,
            },
        }
    }
}

pub fn mount_mat(
    computed_skeleton: &ComputedCrustaceanSkeleton,
    skeleton: &CrustaceanSkeleton,
) -> (Mat4<f32>, Quaternion<f32>) {
    (computed_skeleton.chest, skeleton.chest.orientation)
}

pub fn mount_transform(
    body: &Body,
    computed_skeleton: &ComputedCrustaceanSkeleton,
    skeleton: &CrustaceanSkeleton,
) -> Transform<f32, f32, f32> {
    use comp::crustacean::Species::*;

    let mount_point = match (body.species, body.body_type) {
        (Crab, _) => (0.0, -3.5, 6.0),
        (SoldierCrab, _) => (0.0, -2.5, 8.0),
        (Karkatha, _) => (0.0, -1.0, 32.0),
    }
    .into();

    let (mount_mat, orientation) = mount_mat(computed_skeleton, skeleton);
    Transform {
        position: mount_mat.mul_point(mount_point),
        orientation,
        scale: Vec3::one(),
    }
}
