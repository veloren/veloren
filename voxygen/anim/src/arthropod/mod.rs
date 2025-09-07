pub mod basic;
pub mod idle;
pub mod jump;
pub mod multi;
pub mod run;
pub mod stunned;

// Reexports
pub use self::{
    basic::{BasicAction, BasicActionDependency},
    idle::IdleAnimation,
    jump::JumpAnimation,
    multi::{MultiAction, MultiActionDependency},
    run::RunAnimation,
    stunned::StunnedAnimation,
};

use super::{FigureBoneData, Skeleton, vek::*};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::arthropod::Body;

skeleton_impls!(struct ArthropodSkeleton ComputedArthropodSkeleton {
    + head
    + chest
    + mandible_l
    + mandible_r
    + wing_fl
    + wing_fr
    + wing_bl
    + wing_br
    + leg_fl
    + leg_fr
    + leg_fcl
    + leg_fcr
    + leg_bcl
    + leg_bcr
    + leg_bl
    + leg_br
});

impl Skeleton for ArthropodSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;
    type ComputedSkeleton = ComputedArthropodSkeleton;

    const BONE_COUNT: usize = ComputedArthropodSkeleton::BONE_COUNT;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"arthropod_compute_s\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "arthropod_compute_s"))]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Self::ComputedSkeleton {
        let base_mat = base_mat * Mat4::scaling_3d(SkeletonAttr::from(&body).scaler / 6.0);

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

        let computed_skeleton = ComputedArthropodSkeleton {
            head: head_mat,
            chest: chest_mat,
            mandible_l: mandible_l_mat,
            mandible_r: mandible_r_mat,
            wing_fl: wing_fl_mat,
            wing_fr: wing_fr_mat,
            wing_bl: wing_bl_mat,
            wing_br: wing_br_mat,
            leg_fl: leg_fl_mat,
            leg_fr: leg_fr_mat,
            leg_fcl: leg_fcl_mat,
            leg_fcr: leg_fcr_mat,
            leg_bcl: leg_bcl_mat,
            leg_bcr: leg_bcr_mat,
            leg_bl: leg_bl_mat,
            leg_br: leg_br_mat,
        };

        computed_skeleton.set_figure_bone_data(buf);
        computed_skeleton
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
    snapper: bool,
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
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
            snapper: false,
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
                (Dagonite, _) => (4.0, -1.0),
                (Emberfly, _) => (-1.5, 0.5),
            },
            chest: match (body.species, body.body_type) {
                (Tarantula, _) => (-5.0, 6.0),
                (Blackwidow, _) => (-5.0, 10.0),
                (Antlion, _) => (-5.0, 8.5),
                (Hornbeetle, _) => (-5.0, 7.5),
                (Leafbeetle, _) => (-5.0, 6.0),
                (Stagbeetle, _) => (-5.0, 6.5),
                (Weevil, _) => (-5.0, 6.0),
                (Cavespider, _) => (-5.0, 7.0),
                (Moltencrawler, _) => (-7.0, 6.0),
                (Mosscrawler, _) => (-7.0, 6.5),
                (Sandcrawler, _) => (-7.0, 6.0),
                (Dagonite, _) => (-6.0, 8.0),
                (Emberfly, _) => (-5.0, 2.5),
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
                (Dagonite, _) => (2.5, 8.0, 0.0),
                (Emberfly, _) => (1.5, 7.0, -0.5),
            },
            wing_f: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, 0.0, -4.0),
                (Blackwidow, _) => (3.0, 0.0, -4.0),
                (Antlion, _) => (3.0, 0.0, -4.0),
                (Hornbeetle, _) => (5.5, 5.0, 3.0),
                (Leafbeetle, _) => (0.5, 5.0, 3.0),
                (Stagbeetle, _) => (0.5, 6.0, 4.5),
                (Weevil, _) => (0.5, 5.0, 3.0),
                (Cavespider, _) => (3.0, 0.0, -4.0),
                (Moltencrawler, _) => (3.0, 0.0, -4.0),
                (Mosscrawler, _) => (3.0, 0.0, -4.0),
                (Sandcrawler, _) => (3.0, 0.0, -4.0),
                (Dagonite, _) => (3.0, 0.0, -4.0),
                (Emberfly, _) => (5.5, 6.0, 3.0),
            },
            wing_b: match (body.species, body.body_type) {
                (Tarantula, _) => (3.0, 0.0, -4.0),
                (Blackwidow, _) => (3.0, 0.0, -4.0),
                (Antlion, _) => (3.0, 0.0, -4.0),
                (Hornbeetle, _) => (4.0, 6.0, 2.0),
                (Leafbeetle, _) => (0.5, 4.0, 2.0),
                (Stagbeetle, _) => (0.5, 6.0, 3.0),
                (Weevil, _) => (0.5, 4.0, 1.5),
                (Cavespider, _) => (3.0, 0.0, -4.0),
                (Moltencrawler, _) => (3.0, 0.0, -4.0),
                (Mosscrawler, _) => (3.0, 0.0, -4.0),
                (Sandcrawler, _) => (3.0, 0.0, -4.0),
                (Dagonite, _) => (3.0, 0.0, -4.0),
                (Emberfly, _) => (4.0, 6.0, 2.0),
            },
            leg_f: match (body.species, body.body_type) {
                (Tarantula, _) => (4.0, 11.0, -1.5),
                (Blackwidow, _) => (4.0, 13.5, -6.0),
                (Antlion, _) => (4.0, 11.5, -4.0),
                (Hornbeetle, _) => (5.0, 6.0, -3.0),
                (Leafbeetle, _) => (5.0, 6.0, -1.0),
                (Stagbeetle, _) => (4.5, 6.0, -2.0),
                (Weevil, _) => (5.0, 9.0, -2.0),
                (Cavespider, _) => (4.0, 13.0, -3.0),
                (Moltencrawler, _) => (2.5, 14.0, -3.0),
                (Mosscrawler, _) => (1.5, 14.0, -3.5),
                (Sandcrawler, _) => (1.5, 14.0, -3.0),
                (Dagonite, _) => (1.5, 14.0, -3.0),
                (Emberfly, _) => (2.5, 6.0, -2.5),
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
                (Dagonite, _) => (2.5, 11.0, -3.0),
                (Emberfly, _) => (1.5, 7.5, -2.5),
            },
            leg_bc: match (body.species, body.body_type) {
                (Tarantula, _) => (1.5, 10.5, -1.5),
                (Blackwidow, _) => (2.5, 10.0, -5.5),
                (Antlion, _) => (6.0, 7.5, -4.0),
                (Hornbeetle, _) => (5.0, 6.0, -3.0),
                (Leafbeetle, _) => (4.5, 5.0, -2.5),
                (Stagbeetle, _) => (5.0, 6.0, -2.0),
                (Weevil, _) => (6.0, 5.0, -2.5),
                (Cavespider, _) => (2.5, 9.5, -2.5),
                (Moltencrawler, _) => (2.5, 8.0, -3.0),
                (Mosscrawler, _) => (1.5, 8.0, -3.5),
                (Sandcrawler, _) => (1.5, 8.0, -3.0),
                (Dagonite, _) => (1.5, 8.0, -3.0),
                (Emberfly, _) => (2.5, 3.5, -2.5),
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
                (Dagonite, _) => (2.5, 7.0, -5.5),
                (Emberfly, _) => (1.5, 7.5, -1.0),
            },
            scaler: match (body.species, body.body_type) {
                (Tarantula, _) => 1.0,
                (Blackwidow, _) => 1.0,
                (Antlion, _) => 1.0,
                (Hornbeetle, _) => 1.0,
                (Leafbeetle, _) => 0.8,
                (Stagbeetle, _) => 1.0,
                (Weevil, _) => 0.75,
                (Cavespider, _) => 1.0,
                (Moltencrawler, _) => 1.0,
                (Mosscrawler, _) => 1.0,
                (Sandcrawler, _) => 1.0,
                (Dagonite, _) => 1.0,
                (Emberfly, _) => 0.5,
            },
            // Z ori (front, front center, back center, center)
            leg_ori: match (body.species, body.body_type) {
                (Antlion, _) => (0.7, -0.3, -0.4, 0.4),
                (_, _) => (0.1, -0.3, 0.0, 0.4),
            },
            // Whether or not it used its mandibles for attacks
            snapper: match (body.species, body.body_type) {
                (Stagbeetle, _) => true,
                (Antlion, _) => true,
                (_, _) => false,
            },
        }
    }
}

pub fn mount_mat(
    body: &Body,
    computed_skeleton: &ComputedArthropodSkeleton,
    skeleton: &ArthropodSkeleton,
) -> (Mat4<f32>, Quaternion<f32>) {
    use comp::arthropod::Species::*;

    match (body.species, body.body_type) {
        (
            Hornbeetle | Leafbeetle | Stagbeetle | Weevil | Moltencrawler | Mosscrawler
            | Sandcrawler | Dagonite,
            _,
        ) => (computed_skeleton.head, skeleton.head.orientation),
        _ => (computed_skeleton.chest, skeleton.chest.orientation),
    }
}

pub fn mount_transform(
    body: &Body,
    computed_skeleton: &ComputedArthropodSkeleton,
    skeleton: &ArthropodSkeleton,
) -> Transform<f32, f32, f32> {
    use comp::arthropod::Species::*;

    let mount_point = match (body.species, body.body_type) {
        (Tarantula, _) => (0.0, 1.0, 4.0),
        (Blackwidow, _) => (0.0, 0.0, 5.0),
        (Antlion, _) => (0.0, 2.0, 3.5),
        (Hornbeetle, _) => (0.0, 1.5, 1.5),
        (Leafbeetle, _) => (0.0, 1.5, 3.0),
        (Stagbeetle, _) => (0.0, 3.5, 3.5),
        (Weevil, _) => (0.0, 1.5, 3.0),
        (Cavespider, _) => (0.0, 1.0, 4.0),
        (Moltencrawler, _) => (0.0, 5.5, 6.0),
        (Mosscrawler, _) => (0.0, 6.5, 6.0),
        (Sandcrawler, _) => (0.0, 6.5, 6.0),
        (Dagonite, _) => (0.0, 8.5, 6.0),
        (Emberfly, _) => (0.0, 3.0, 4.0),
    }
    .into();

    let (mount_mat, orientation) = mount_mat(body, computed_skeleton, skeleton);
    Transform {
        position: mount_mat.mul_point(mount_point),
        orientation,
        scale: Vec3::one(),
    }
}
