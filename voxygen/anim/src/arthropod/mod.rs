pub mod alpha;
pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{
    alpha::AlphaAnimation, idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation,
};

use super::{make_bone, vek::*, FigureBoneData, Offsets, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::arthropod::Body;

skeleton_impls!(struct ArthropodSkeleton {
    + head,
    + chest,
    + mandible_l,
    + mandible_r,
    + wing_fl,
    + wing_fr,
    + wing_bl,
    + wing_br,
    + leg_fl,
    + leg_fr,
    + leg_fcl,
    + leg_fcr,
    + leg_bcl,
    + leg_bcr,
    + leg_bl,
    + leg_br,
});

impl Skeleton for ArthropodSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 16;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"arthropod_compute_s\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "arthropod_compute_s")]

    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets {
        let chest_mat = base_mat * Mat4::<f32>::from(self.chest);
        let head_mat = chest_mat * Mat4::<f32>::from(self.head);
        let mandible_l_mat = head_mat * Mat4::<f32>::from(self.mandible_l);
        let mandible_r_mat = head_mat * Mat4::<f32>::from(self.mandible_r);
        let wing_fl_mat = chest_mat * Mat4::<f32>::from(self.wing_fl);
        let wing_fr_mat = chest_mat * Mat4::<f32>::from(self.wing_fr);
        let wing_bl_mat = chest_mat * Mat4::<f32>::from(self.wing_bl);
        let wing_br_mat = chest_mat * Mat4::<f32>::from(self.wing_br);
        let leg_fl_mat = chest_mat * Mat4::<f32>::from(self.leg_fl);
        let leg_fr_mat = chest_mat * Mat4::<f32>::from(self.leg_fr);
        let leg_fcl_mat = chest_mat * Mat4::<f32>::from(self.leg_fcl);
        let leg_fcr_mat = chest_mat * Mat4::<f32>::from(self.leg_fcr);
        let leg_bcl_mat = chest_mat * Mat4::<f32>::from(self.leg_bcl);
        let leg_bcr_mat = chest_mat * Mat4::<f32>::from(self.leg_bcr);
        let leg_bl_mat = chest_mat * Mat4::<f32>::from(self.leg_bl);
        let leg_br_mat = chest_mat * Mat4::<f32>::from(self.leg_br);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_mat),
            make_bone(chest_mat),
            make_bone(mandible_l_mat),
            make_bone(mandible_r_mat),
            make_bone(wing_fl_mat),
            make_bone(wing_fr_mat),
            make_bone(wing_bl_mat),
            make_bone(wing_br_mat),
            make_bone(leg_fl_mat),
            make_bone(leg_fr_mat),
            make_bone(leg_fcl_mat),
            make_bone(leg_fcr_mat),
            make_bone(leg_bcl_mat),
            make_bone(leg_bcr_mat),
            make_bone(leg_bl_mat),
            make_bone(leg_br_mat),
        ];

        use comp::arthropod::Species::*;
        let (mount_bone_mat, mount_bone_ori) = match (body.species, body.body_type) {
            _ => (chest_mat, self.chest.orientation),
        };
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
            mount_bone: Transform {
                position: mount_position,
                orientation: mount_orientation,
                scale: Vec3::one(),
            },
        }
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    chest: (f32, f32),
    mandible_l: (f32, f32, f32),
    mandible_r: (f32, f32, f32),
    wing_fl: (f32, f32, f32),
    wing_fr: (f32, f32, f32),
    wing_bl: (f32, f32, f32),
    wing_br: (f32, f32, f32),
    leg_fl: (f32, f32, f32),
    leg_fr: (f32, f32, f32),
    leg_fcl: (f32, f32, f32),
    leg_fcr: (f32, f32, f32),
    leg_bcl: (f32, f32, f32),
    leg_bcr: (f32, f32, f32),
    leg_bl: (f32, f32, f32),
    leg_br: (f32, f32, f32),
    scaler: f32,
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::Arthropod(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            chest: (0.0, 0.0),
            mandible_l: (0.0, 0.0, 0.0),
            mandible_r: (0.0, 0.0, 0.0),
            wing_fl: (0.0, 0.0, 0.0),
            wing_fr: (0.0, 0.0, 0.0),
            wing_bl: (0.0, 0.0, 0.0),
            wing_br: (0.0, 0.0, 0.0),
            leg_fl: (0.0, 0.0, 0.0),
            leg_fr: (0.0, 0.0, 0.0),
            leg_fcl: (0.0, 0.0, 0.0),
            leg_fcr: (0.0, 0.0, 0.0),
            leg_bcl: (0.0, 0.0, 0.0),
            leg_bcr: (0.0, 0.0, 0.0),
            leg_bl: (0.0, 0.0, 0.0),
            leg_br: (0.0, 0.0, 0.0),
            scaler: 0.0,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::arthropod::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Tarantula, _) => (8.0, 4.0),
            },
            chest: match (body.species, body.body_type) {
                (Tarantula, _) => (1.0, -7.0),
            },
            mandible_l: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, 0.0, -4.0),
            },
            mandible_r: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, 0.0, -4.0),
            },
            wing_fl: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, 0.0, -4.0),
            },
            wing_fr: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, 0.0, -4.0),
            },
            wing_bl: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, 0.0, -4.0),
            },
            wing_br: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, 0.0, -4.0),
            },
            leg_fl: match (body.species, body.body_type) {
                (Tarantula, _) => (2.5, -3.0, -4.0),
            },
            leg_fr: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, -0.5, -7.0),
            },
            leg_fcl: match (body.species, body.body_type) {
                (Tarantula, _) => (2.5, -3.0, -4.0),
            },
            leg_fcr: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, -0.5, -7.0),
            },
            leg_bcl: match (body.species, body.body_type) {
                (Tarantula, _) => (2.5, -3.0, -4.0),
            },
            leg_bcr: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, -0.5, -7.0),
            },
            leg_bl: match (body.species, body.body_type) {
                (Tarantula, _) => (2.5, -3.0, -4.0),
            },
            leg_br: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, -0.5, -7.0),
            },
            scaler: match (body.species, body.body_type) {
                (Tarantula, _) => (1.0),
            },
        }
    }
}

fn mount_point(body: &Body) -> Vec3<f32> {
    use comp::arthropod::{BodyType::*, Species::*};
    match (body.species, body.body_type) {
        (_, _) => (0.0, -6.0, 6.0),
    }
    .into()
}
