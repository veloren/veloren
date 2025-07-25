pub mod combomelee;
pub mod dash;
pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{
    combomelee::ComboAnimation, dash::DashAnimation, idle::IdleAnimation, jump::JumpAnimation,
    run::RunAnimation,
};

use super::{FigureBoneData, Skeleton, vek::*};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::theropod::Body;

skeleton_impls!(struct TheropodSkeleton ComputedTheropodSkeleton {
    + head
    + jaw
    + neck
    + chest_front
    + chest_back
    + tail_front
    + tail_back
    + hand_l
    + hand_r
    + leg_l
    + leg_r
    + foot_l
    + foot_r
});

impl Skeleton for TheropodSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;
    type ComputedSkeleton = ComputedTheropodSkeleton;

    const BONE_COUNT: usize = ComputedTheropodSkeleton::BONE_COUNT;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"theropod_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "theropod_compute_mats"))]

    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Self::ComputedSkeleton {
        let base_mat = base_mat * Mat4::scaling_3d(SkeletonAttr::from(&body).scaler / 11.0);

        let chest_front_mat = base_mat * Mat4::<f32>::from(self.chest_front);
        let neck_mat = chest_front_mat * Mat4::<f32>::from(self.neck);
        let head_mat = neck_mat * Mat4::<f32>::from(self.head);
        let chest_back_mat = chest_front_mat * Mat4::<f32>::from(self.chest_back);
        let tail_front_mat = chest_back_mat * Mat4::<f32>::from(self.tail_front);
        let leg_l_mat = chest_back_mat * Mat4::<f32>::from(self.leg_l);
        let leg_r_mat = chest_back_mat * Mat4::<f32>::from(self.leg_r);

        let computed_skeleton = ComputedTheropodSkeleton {
            head: head_mat,
            jaw: head_mat * Mat4::<f32>::from(self.jaw),
            neck: neck_mat,
            chest_front: chest_front_mat,
            chest_back: chest_back_mat,
            tail_front: tail_front_mat,
            tail_back: tail_front_mat * Mat4::<f32>::from(self.tail_back),
            hand_l: chest_front_mat * Mat4::<f32>::from(self.hand_l),
            hand_r: chest_front_mat * Mat4::<f32>::from(self.hand_r),
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
    neck: (f32, f32),
    jaw: (f32, f32),
    chest_front: (f32, f32),
    chest_back: (f32, f32),
    tail_front: (f32, f32),
    tail_back: (f32, f32),
    hand: (f32, f32, f32),
    leg: (f32, f32, f32),
    foot: (f32, f32, f32),
    scaler: f32,
    steady_wings: bool,
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::Theropod(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            neck: (0.0, 0.0),
            jaw: (0.0, 0.0),
            chest_front: (0.0, 0.0),
            chest_back: (0.0, 0.0),
            tail_front: (0.0, 0.0),
            tail_back: (0.0, 0.0),
            hand: (0.0, 0.0, 0.0),
            leg: (0.0, 0.0, 0.0),
            foot: (0.0, 0.0, 0.0),
            scaler: 0.0,
            steady_wings: false,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::theropod::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Archaeos, _) => (8.0, 4.0),
                (Odonto, _) => (2.0, 2.0),
                (Sandraptor, _) => (8.0, 5.0),
                (Snowraptor, _) => (8.0, 5.0),
                (Woodraptor, _) => (8.0, 5.0),
                (Sunlizard, _) => (6.5, 3.5),
                (Yale, _) => (7.0, 14.0),
                (Dodarock, _) => (2.0, 1.5),
                (Ntouka, _) => (2.0, 2.5),
                (Axebeak, _) => (11.5, 8.5),
            },
            jaw: match (body.species, body.body_type) {
                (Archaeos, _) => (1.0, -7.0),
                (Odonto, _) => (2.0, -7.0),
                (Sandraptor, _) => (0.0, -4.0),
                (Snowraptor, _) => (0.0, -4.0),
                (Woodraptor, _) => (0.0, -4.0),
                (Sunlizard, _) => (2.0, -2.5),
                (Yale, _) => (2.0, -9.5),
                (Dodarock, _) => (0.0, -5.0),
                (Ntouka, _) => (0.0, -4.0),
                (Axebeak, _) => (2.5, -4.0),
            },
            neck: match (body.species, body.body_type) {
                (Archaeos, _) => (4.5, -2.0),
                (Odonto, _) => (4.0, 0.0),
                (Sandraptor, _) => (4.0, 2.5),
                (Snowraptor, _) => (4.0, 2.5),
                (Woodraptor, _) => (4.0, 2.5),
                (Sunlizard, _) => (2.5, 1.5),
                (Yale, _) => (2.0, 4.0),
                (Dodarock, _) => (5.0, -1.0),
                (Ntouka, _) => (4.0, 0.0),
                (Axebeak, _) => (-5.5, 0.0),
            },
            chest_front: match (body.species, body.body_type) {
                (Archaeos, _) => (0.0, 20.0),
                (Odonto, _) => (0.0, 13.0),
                (Sandraptor, _) => (0.0, 15.5),
                (Snowraptor, _) => (0.0, 15.5),
                (Woodraptor, _) => (0.0, 15.5),
                (Sunlizard, _) => (0.0, 14.0),
                (Yale, _) => (0.0, 19.5),
                (Dodarock, _) => (0.0, 12.0),
                (Ntouka, _) => (0.0, 13.0),
                (Axebeak, _) => (0.0, 12.0),
            },
            chest_back: match (body.species, body.body_type) {
                (Archaeos, _) => (-5.5, -1.0),
                (Odonto, _) => (-5.0, 2.0),
                (Sandraptor, _) => (-3.0, 0.5),
                (Snowraptor, _) => (-3.0, 0.5),
                (Woodraptor, _) => (-3.0, 0.5),
                (Sunlizard, _) => (-2.0, 0.0),
                (Yale, _) => (-3.0, -1.0),
                (Dodarock, _) => (-4.5, -2.0),
                (Ntouka, _) => (-4.5, 1.0),
                (Axebeak, _) => (-5.0, 0.0),
            },
            tail_front: match (body.species, body.body_type) {
                (Archaeos, _) => (-9.0, -1.5),
                (Odonto, _) => (-7.0, -1.0),
                (Sandraptor, _) => (-9.5, -1.0),
                (Snowraptor, _) => (-9.5, -1.0),
                (Woodraptor, _) => (-9.5, -1.0),
                (Sunlizard, _) => (-8.5, -2.0),
                (Yale, _) => (-9.5, -4.0),
                (Dodarock, _) => (-4.5, -4.5),
                (Ntouka, _) => (-9.5, -3.5),
                (Axebeak, _) => (-5.5, 4.5),
            },
            tail_back: match (body.species, body.body_type) {
                (Archaeos, _) => (-8.0, -0.5),
                (Odonto, _) => (-8.0, 0.5),
                (Sandraptor, _) => (-10.5, 0.5),
                (Snowraptor, _) => (-10.5, 1.0),
                (Woodraptor, _) => (-10.5, 0.5),
                (Sunlizard, _) => (-10.0, -0.5),
                (Yale, _) => (-5.0, -2.5),
                (Dodarock, _) => (-8.5, -2.0),
                (Ntouka, _) => (-9.5, -2.0),
                (Axebeak, _) => (-10.0, 3.0),
            },
            hand: match (body.species, body.body_type) {
                (Archaeos, _) => (3.0, 0.0, -4.0),
                (Odonto, _) => (3.5, 3.0, -4.0),
                (Sandraptor, _) => (2.5, 3.0, 1.0),
                (Snowraptor, _) => (2.5, 3.0, 1.0),
                (Woodraptor, _) => (2.5, 3.0, 1.0),
                (Sunlizard, _) => (2.5, 1.5, -0.5),
                (Yale, _) => (3.0, 2.0, -0.5),
                (Dodarock, _) => (3.5, 3.0, -5.0),
                (Ntouka, _) => (3.5, 3.0, -4.0),
                (Axebeak, _) => (1.5, -10.5, 9.5),
            },
            leg: match (body.species, body.body_type) {
                (Archaeos, _) => (2.5, -3.0, -4.0),
                (Odonto, _) => (3.5, -2.5, -4.0),
                (Sandraptor, _) => (1.5, -2.5, -3.0),
                (Snowraptor, _) => (1.5, -2.5, -3.0),
                (Woodraptor, _) => (1.5, -2.5, -3.0),
                (Sunlizard, _) => (2.5, -2.5, -3.0),
                (Yale, _) => (3.0, -3.5, -4.0),
                (Dodarock, _) => (3.5, 1.5, -4.0),
                (Ntouka, _) => (4.5, -5.5, -4.0),
                (Axebeak, _) => (2.5, -0.5, 0.0),
            },
            foot: match (body.species, body.body_type) {
                (Archaeos, _) => (3.0, -0.5, -7.0),
                (Odonto, _) => (4.0, -6.5, -3.0),
                (Sandraptor, _) => (2.0, 0.0, -3.0),
                (Snowraptor, _) => (2.0, 0.0, -3.0),
                (Woodraptor, _) => (2.0, 0.0, -3.0),
                (Sunlizard, _) => (1.0, -0.5, -2.5),
                (Yale, _) => (1.5, 1.0, -3.5),
                (Dodarock, _) => (1.5, -1.0, -3.5),
                (Ntouka, _) => (1.5, -1.0, -2.5),
                (Axebeak, _) => (2.5, 2.5, -7.0),
            },
            scaler: match (body.species, body.body_type) {
                (Archaeos, _) => 2.93,
                (Odonto, _) => 2.93,
                (Sandraptor, _) => 0.88,
                (Snowraptor, _) => 0.85,
                (Woodraptor, _) => 0.9,
                (Sunlizard, _) => 1.1,
                (Yale, _) => 1.26,
                (Dodarock, _) => 1.1,
                (Ntouka, _) => 2.93,
                (Axebeak, _) => 1.1,
            },
            steady_wings: matches!((body.species, body.body_type), (Axebeak, _)),
        }
    }
}

pub fn mount_mat(
    body: &Body,
    computed_skeleton: &ComputedTheropodSkeleton,
    skeleton: &TheropodSkeleton,
) -> (Mat4<f32>, Quaternion<f32>) {
    use comp::theropod::Species::*;

    match (body.species, body.body_type) {
        (Archaeos, _) => (computed_skeleton.neck, skeleton.neck.orientation),
        (Odonto, _) => (computed_skeleton.head, skeleton.head.orientation),
        (Yale | Dodarock, _) => (
            computed_skeleton.chest_back,
            skeleton.chest_back.orientation,
        ),
        _ => (
            computed_skeleton.chest_front,
            skeleton.chest_front.orientation,
        ),
    }
}

pub fn mount_transform(
    body: &Body,
    computed_skeleton: &ComputedTheropodSkeleton,
    skeleton: &TheropodSkeleton,
) -> Transform<f32, f32, f32> {
    use comp::theropod::Species::*;

    let mount_point = match (body.species, body.body_type) {
        (Archaeos, _) => (0.0, 2.5, 6.0),
        (Odonto, _) => (0.0, 10.0, 2.0),
        (Sandraptor, _) => (0.0, -2.0, 5.0),
        (Snowraptor, _) => (0.0, -2.0, 5.0),
        (Woodraptor, _) => (0.0, -2.0, 5.0),
        (Sunlizard, _) => (0.0, -2.0, 3.5),
        (Yale, _) => (0.0, -2.5, 5.5),
        (Ntouka, _) => (0.0, -4.0, 7.5),
        (Dodarock, _) => (0.0, 3.5, 5.0),
        (Axebeak, _) => (0.0, -3.5, 6.5),
    }
    .into();

    let (mount_mat, orientation) = mount_mat(body, computed_skeleton, skeleton);
    Transform {
        position: mount_mat.mul_point(mount_point),
        orientation,
        scale: Vec3::one(),
    }
}
