pub mod alpha;
pub mod beta;
pub mod dash;
pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{
    alpha::AlphaAnimation, beta::BetaAnimation, dash::DashAnimation, idle::IdleAnimation,
    jump::JumpAnimation, run::RunAnimation,
};

use super::{make_bone, vek::*, FigureBoneData, Offsets, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::theropod::Body;

skeleton_impls!(struct TheropodSkeleton {
    + head,
    + jaw,
    + neck,
    + chest_front,
    + chest_back,
    + tail_front,
    + tail_back,
    + hand_l,
    + hand_r,
    + leg_l,
    + leg_r,
    + foot_l,
    + foot_r,
});

impl Skeleton for TheropodSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 13;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"theropod_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "theropod_compute_mats")]

    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets {
        let base_mat = base_mat * Mat4::scaling_3d(SkeletonAttr::from(&body).scaler / 11.0);

        let chest_front_mat = base_mat * Mat4::<f32>::from(self.chest_front);
        let neck_mat = chest_front_mat * Mat4::<f32>::from(self.neck);
        let head_mat = neck_mat * Mat4::<f32>::from(self.head);
        let chest_back_mat = chest_front_mat * Mat4::<f32>::from(self.chest_back);
        let tail_front_mat = chest_back_mat * Mat4::<f32>::from(self.tail_front);
        let leg_l_mat = chest_back_mat * Mat4::<f32>::from(self.leg_l);
        let leg_r_mat = chest_back_mat * Mat4::<f32>::from(self.leg_r);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_mat),
            make_bone(head_mat * Mat4::<f32>::from(self.jaw)),
            make_bone(neck_mat),
            make_bone(chest_front_mat),
            make_bone(chest_back_mat),
            make_bone(tail_front_mat),
            make_bone(tail_front_mat * Mat4::<f32>::from(self.tail_back)),
            make_bone(chest_front_mat * Mat4::<f32>::from(self.hand_l)),
            make_bone(chest_front_mat * Mat4::<f32>::from(self.hand_r)),
            make_bone(leg_l_mat),
            make_bone(leg_r_mat),
            make_bone(leg_l_mat * Mat4::<f32>::from(self.foot_l)),
            make_bone(leg_r_mat * Mat4::<f32>::from(self.foot_r)),
        ];
        Offsets {
            lantern: None,
            viewpoint: Some((head_mat * Vec4::new(0.0, 2.0, 0.0, 1.0)).xyz()),
            // TODO: see quadruped_medium for how to animate this
            mount_bone: Transform {
                position: comp::Body::Theropod(body)
                    .mount_offset()
                    .into_tuple()
                    .into(),
                ..Default::default()
            },
            primary_trail_mat: None,
            secondary_trail_mat: None,
        }
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
                (Sandraptor, _) => 1.1,
                (Snowraptor, _) => 1.1,
                (Woodraptor, _) => 1.1,
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
