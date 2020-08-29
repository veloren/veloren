pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{make_bone, vek::*, FigureBoneData, Skeleton};
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
    ) -> Vec3<f32> {
        let chest_front_mat = base_mat * Mat4::<f32>::from(self.chest_front);
        let neck_mat = chest_front_mat * Mat4::<f32>::from(self.neck);
        let head_mat = neck_mat * Mat4::<f32>::from(self.head);
        let chest_back_mat = chest_front_mat * Mat4::<f32>::from(self.chest_back);
        let tail_front_mat = chest_back_mat * Mat4::<f32>::from(self.tail_front);
        let leg_l_mat = chest_front_mat * Mat4::<f32>::from(self.leg_l);
        let leg_r_mat = chest_front_mat * Mat4::<f32>::from(self.leg_r);

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
        Vec3::default()
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
    hand_l: (f32, f32, f32),
    hand_r: (f32, f32, f32),
    leg_l: (f32, f32, f32),
    leg_r: (f32, f32, f32),
    foot_l: (f32, f32, f32),
    foot_r: (f32, f32, f32),
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
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
            hand_l: (0.0, 0.0, 0.0),
            hand_r: (0.0, 0.0, 0.0),
            leg_l: (0.0, 0.0, 0.0),
            leg_r: (0.0, 0.0, 0.0),
            foot_l: (0.0, 0.0, 0.0),
            foot_r: (0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::theropod::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Archaeos, _) => (6.5, 3.0),
                (Odontotyrannos, _) => (5.0, 1.0),
            },
            jaw: match (body.species, body.body_type) {
                (Archaeos, _) => (0.0, 6.0),
                (Odontotyrannos, _) => (-1.0, 3.0),
            },
            neck: match (body.species, body.body_type) {
                (Archaeos, _) => (0.0, 6.0),
                (Odontotyrannos, _) => (-1.0, 3.0),
            },
            chest_front: match (body.species, body.body_type) {
                (Archaeos, _) => (0.0, 6.0),
                (Odontotyrannos, _) => (-1.0, 3.0),
            },
            chest_back: match (body.species, body.body_type) {
                (Archaeos, _) => (0.0, 6.0),
                (Odontotyrannos, _) => (-1.0, 3.0),
            },
            tail_front: match (body.species, body.body_type) {
                (Archaeos, _) => (-8.0, -1.0),
                (Odontotyrannos, _) => (-7.0, -1.0),
            },
            tail_back: match (body.species, body.body_type) {
                (Archaeos, _) => (-8.0, -1.0),
                (Odontotyrannos, _) => (-7.0, -1.0),
            },
            hand_l: match (body.species, body.body_type) {
                (Archaeos, _) => (-8.0, -1.0, 0.0),
                (Odontotyrannos, _) => (-7.0, -1.0, 0.0),
            },
            hand_r: match (body.species, body.body_type) {
                (Archaeos, _) => (8.0, -1.0, 0.0),
                (Odontotyrannos, _) => (7.0, -1.0, 0.0),
            },
            leg_l: match (body.species, body.body_type) {
                (Archaeos, _) => (-8.0, -1.0, 0.0),
                (Odontotyrannos, _) => (-7.0, -1.0, 0.0),
            },
            leg_r: match (body.species, body.body_type) {
                (Archaeos, _) => (8.0, -1.0, 0.0),
                (Odontotyrannos, _) => (7.0, -1.0, 0.0),
            },
            foot_l: match (body.species, body.body_type) {
                (Archaeos, _) => (-8.0, -1.0, 0.0),
                (Odontotyrannos, _) => (-7.0, -1.0, 0.0),
            },
            foot_r: match (body.species, body.body_type) {
                (Archaeos, _) => (8.0, -1.0, 0.0),
                (Odontotyrannos, _) => (7.0, -1.0, 0.0),
            },
        }
    }
}
