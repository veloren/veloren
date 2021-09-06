pub mod alpha;
pub mod dash;
pub mod idle;
pub mod jump;
pub mod leapmelee;
pub mod run;
pub mod stunned;

// Reexports
pub use self::{
    alpha::AlphaAnimation, dash::DashAnimation, idle::IdleAnimation, jump::JumpAnimation,
    leapmelee::LeapMeleeAnimation, run::RunAnimation, stunned::StunnedAnimation,
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
        let base_mat = base_mat * Mat4::scaling_3d(SkeletonAttr::from(&body).scaler / 4.0);

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

        // TODO: mount points
        //use comp::arthropod::Species::*;
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
    mandible: (f32, f32, f32),
    wing_f: (f32, f32, f32),
    wing_b: (f32, f32, f32),
    leg_f: (f32, f32, f32),
    leg_fc: (f32, f32, f32),
    leg_bc: (f32, f32, f32),
    leg_b: (f32, f32, f32),
    scaler: f32,
    leg_ori: (f32, f32, f32, f32),
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
            mandible: (0.0, 0.0, 0.0),
            wing_f: (0.0, 0.0, 0.0),
            wing_b: (0.0, 0.0, 0.0),
            leg_f: (0.0, 0.0, 0.0),
            leg_fc: (0.0, 0.0, 0.0),
            leg_bc: (0.0, 0.0, 0.0),
            leg_b: (0.0, 0.0, 0.0),
            scaler: 0.0,
            leg_ori: (0.0, 0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::arthropod::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Tarantula, _) => (6.0, 0.5),
                (Blackwidow, _) => (5.5, -3.0),
                (Antlion, _) => (4.5, 0.0),
                (Hornbeetle, _) => (5.0, 3.0),
                (Leafbeetle, _) => (4.0, 0.0),
                (Stagbeetle, _) => (5.0, -1.0),
                (Weevil, _) => (4.0, 0.0),
                (Cavespider, _) => (6.0, 0.5),
                (Moltencrawler, _) => (4.0, -1.0),
                (Mosscrawler, _) => (4.0, -1.5),
                (Sandcrawler, _) => (4.0, -1.0),
            },
            chest: match (body.species, body.body_type) {
                (Tarantula, _) => (-9.0, 6.0),
                (Blackwidow, _) => (-9.0, 10.0),
                (Antlion, _) => (-9.0, 8.5),
                (Hornbeetle, _) => (-9.0, 7.5),
                (Leafbeetle, _) => (-9.0, 6.0),
                (Stagbeetle, _) => (-9.0, 6.5),
                (Weevil, _) => (-9.0, 6.0),
                (Cavespider, _) => (-9.0, 7.0),
                (Moltencrawler, _) => (-9.0, 6.0),
                (Mosscrawler, _) => (-9.0, 6.5),
                (Sandcrawler, _) => (-9.0, 6.0),
            },
            mandible: match (body.species, body.body_type) {
                (Tarantula, _) => (1.5, 7.0, -0.5),
                (Blackwidow, _) => (2.5, 8.0, 0.0),
                (Antlion, _) => (8.5, 9.0, -3.5),
                (Hornbeetle, _) => (1.5, 7.0, -0.5),
                (Leafbeetle, _) => (1.5, 7.0, -0.5),
                (Stagbeetle, _) => (1.5, 10.0, 1.0),
                (Weevil, _) => (1.5, 7.0, -0.5),
                (Cavespider, _) => (2.5, 8.0, -0.5),
                (Moltencrawler, _) => (2.5, 8.0, 0.0),
                (Mosscrawler, _) => (2.5, 8.0, 0.0),
                (Sandcrawler, _) => (2.5, 8.0, 0.0),
            },
            wing_f: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, 0.0, -4.0),
                (Blackwidow, _) => (3.0, 0.0, -4.0),
                (Antlion, _) => (3.0, 0.0, -4.0),
                (Hornbeetle, _) => (-0.5, 5.0, 3.0),
                (Leafbeetle, _) => (0.5, 5.0, 3.0),
                (Stagbeetle, _) => (0.5, 6.0, 4.5),
                (Weevil, _) => (0.5, 5.0, 3.0),
                (Cavespider, _) => (3.0, 0.0, -4.0),
                (Moltencrawler, _) => (3.0, 0.0, -4.0),
                (Mosscrawler, _) => (3.0, 0.0, -4.0),
                (Sandcrawler, _) => (3.0, 0.0, -4.0),
            },
            wing_b: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, 0.0, -4.0),
                (Blackwidow, _) => (3.0, 0.0, -4.0),
                (Antlion, _) => (3.0, 0.0, -4.0),
                (Hornbeetle, _) => (0.0, 6.0, 2.0),
                (Leafbeetle, _) => (0.5, 4.0, 2.0),
                (Stagbeetle, _) => (0.5, -5.0, 3.0),
                (Weevil, _) => (0.5, 4.0, 1.5),
                (Cavespider, _) => (3.0, 0.0, -4.0),
                (Moltencrawler, _) => (3.0, 0.0, -4.0),
                (Mosscrawler, _) => (3.0, 0.0, -4.0),
                (Sandcrawler, _) => (3.0, 0.0, -4.0),
            },
            leg_f: match (body.species, body.body_type) {
                (Tarantula, _) => (4.0, 11.0, -1.5),
                (Blackwidow, _) => (4.0, 13.5, -6.0),
                (Antlion, _) => (4.0, 11.5, -4.0),
                (Hornbeetle, _) => (5.0, 6.0, -3.0),
                (Leafbeetle, _) => (5.0, 6.0, -1.0),
                (Stagbeetle, _) => (5.0, 6.0, -2.0),
                (Weevil, _) => (5.0, 9.0, -2.0),
                (Cavespider, _) => (4.0, 13.0, -3.0),
                (Moltencrawler, _) => (2.5, 14.0, -3.0),
                (Mosscrawler, _) => (1.5, 14.0, -3.5),
                (Sandcrawler, _) => (1.5, 14.0, -3.0),
            },
            leg_fc: match (body.species, body.body_type) {
                (Tarantula, _) => (1.5, 13.5, -1.5),
                (Blackwidow, _) => (2.5, 13.0, -5.5),
                (Antlion, _) => (1.5, 6.0, -4.0),
                (Hornbeetle, _) => (1.5, 7.5, -3.0),
                (Leafbeetle, _) => (1.5, 6.5, -1.5),
                (Stagbeetle, _) => (1.5, 7.5, -2.0),
                (Weevil, _) => (1.5, 8.5, -2.0),
                (Cavespider, _) => (2.5, 12.5, -2.5),
                (Moltencrawler, _) => (3.5, 11.0, -3.0),
                (Mosscrawler, _) => (2.5, 11.0, -3.5),
                (Sandcrawler, _) => (2.5, 11.0, -3.0),
            },
            leg_bc: match (body.species, body.body_type) {
                (Tarantula, _) => (1.5, 10.5, -1.5),
                (Blackwidow, _) => (2.5, 10.0, -5.5),
                (Antlion, _) => (6.0, 7.5, -4.0),
                (Hornbeetle, _) => (6.0, 6.0, -3.0),
                (Leafbeetle, _) => (6.0, 5.0, -2.5),
                (Stagbeetle, _) => (6.0, 6.0, -2.0),
                (Weevil, _) => (6.0, 5.0, -2.5),
                (Cavespider, _) => (2.5, 9.5, -2.5),
                (Moltencrawler, _) => (2.5, 8.0, -3.0),
                (Mosscrawler, _) => (1.5, 8.0, -3.5),
                (Sandcrawler, _) => (1.5, 8.0, -3.0),
            },
            leg_b: match (body.species, body.body_type) {
                (Tarantula, _) => (1.5, 7.5, -1.5),
                (Blackwidow, _) => (2.5, 7.0, -5.5),
                (Antlion, _) => (1.5, 7.5, -1.5),
                (Hornbeetle, _) => (1.5, 7.5, -1.5),
                (Leafbeetle, _) => (1.5, 7.5, -1.5),
                (Stagbeetle, _) => (1.5, 7.5, -1.5),
                (Weevil, _) => (1.5, 7.5, -1.5),
                (Cavespider, _) => (2.5, 6.5, -2.5),
                (Moltencrawler, _) => (2.5, 7.0, -5.5),
                (Mosscrawler, _) => (2.5, 7.0, -5.5),
                (Sandcrawler, _) => (2.5, 7.0, -5.5),
            },
            scaler: match (body.species, body.body_type) {
                (Tarantula, _) => (1.0),
                (Blackwidow, _) => (1.0),
                (Antlion, _) => (1.0),
                (Hornbeetle, _) => (1.0),
                (Leafbeetle, _) => (1.0),
                (Stagbeetle, _) => (1.0),
                (Weevil, _) => (1.0),
                (Cavespider, _) => (1.0),
                (Moltencrawler, _) => (1.0),
                (Mosscrawler, _) => (1.0),
                (Sandcrawler, _) => (1.0),
            },
            // Z ori (front, front center, back center, center)
            leg_ori: match (body.species, body.body_type) {
                (Antlion, _) => (0.7, -0.3, -0.4, 0.4),
                (_, _) => (0.1, -0.3, 0.0, 0.4),
            },
        }
    }
}

fn mount_point(body: &Body) -> Vec3<f32> {
    // TODO: mount points
    //use comp::arthropod::{BodyType::*, Species::*};
    match (body.species, body.body_type) {
        (_, _) => (0.0, -6.0, 6.0),
    }
    .into()
}
