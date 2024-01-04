mod alpha;
mod combomelee;
mod idle;
mod jump;
mod run;
mod stunned;
mod swim;

// Reexports
pub use self::{
    alpha::AlphaAnimation, combomelee::ComboAnimation, idle::IdleAnimation, jump::JumpAnimation,
    run::RunAnimation, stunned::StunnedAnimation, swim::SwimAnimation,
};

use common::comp::{self};

use super::{make_bone, vek::*, FigureBoneData, Offsets, Skeleton};

pub type Body = comp::crustacean::Body;

skeleton_impls!(struct CrustaceanSkeleton {
    + chest,
    + tail_f,
    + tail_b,
    + arm_l,
    + pincer_l0,
    + pincer_l1,
    + arm_r,
    + pincer_r0,
    + pincer_r1,
    + leg_fl,
    + leg_cl,
    + leg_bl,
    + leg_fr,
    + leg_cr,
    + leg_br,
});

impl Skeleton for CrustaceanSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 15;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"crustacean_compute_s\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "crustacean_compute_s")]

    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets {
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

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(chest_mat),
            make_bone(tail_f_mat),
            make_bone(tail_b_mat),
            make_bone(arm_l_mat),
            make_bone(pincer_l0_mat),
            make_bone(pincer_l1_mat),
            make_bone(arm_r_mat),
            make_bone(pincer_r0_mat),
            make_bone(pincer_r1_mat),
            make_bone(leg_fl_mat),
            make_bone(leg_cl_mat),
            make_bone(leg_bl_mat),
            make_bone(leg_fr_mat),
            make_bone(leg_cr_mat),
            make_bone(leg_br_mat),
        ];

        // TODO: mount points
        //use comp::arthropod::Species::*;
        let (mount_bone_mat, mount_bone_ori) = (chest_mat, self.chest.orientation);
        // Offset from the mounted bone's origin.
        // Note: This could be its own bone if we need to animate it independently.
        let mount_position = (mount_bone_mat * Vec4::from_point(mount_point(&body)))
            .homogenized()
            .xyz();
        // NOTE: We apply the ori from base_mat externally so we don't need to worry
        // about it here for now.
        let mount_orientation = mount_bone_ori;

        Offsets {
            lantern: None,
            viewpoint: Some((chest_mat * Vec4::new(0.0, 7.0, 0.0, 1.0)).xyz()),
            mount_bone: Transform {
                position: mount_position,
                orientation: mount_orientation,
                scale: Vec3::one(),
            },
            primary_trail_mat: None,
            secondary_trail_mat: None,
        }
    }
}

pub struct SkeletonAttr {
    chest: (f32, f32),
    leg_f: (f32, f32, f32),
    leg_c: (f32, f32, f32),
    leg_b: (f32, f32, f32),
    leg_ori: (f32, f32, f32),
    scaler: f32,
}

impl From<&Body> for SkeletonAttr {
    fn from(_value: &Body) -> Self {
        Self {
            chest: (0.0, 0.0),
            leg_f: (0.0, 0.0, 0.0),
            leg_c: (0.0, 0.0, 0.0),
            leg_b: (0.0, 0.0, 0.0),
            leg_ori: (-0.4, 0.0, 0.4),
            scaler: 0.62,
        }
    }
}

fn mount_point(_body: &Body) -> Vec3<f32> {
    // TODO: mount points
    //use comp::arthropod::{BodyType::*, Species::*};
    (0.0, -6.0, 6.0).into()
}
