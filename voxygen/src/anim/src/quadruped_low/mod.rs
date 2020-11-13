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

pub type Body = comp::quadruped_low::Body;

skeleton_impls!(struct QuadrupedLowSkeleton {
    + head_upper,
    + head_lower,
    + jaw,
    + chest,
    + tail_front,
    + tail_rear,
    + foot_fl,
    + foot_fr,
    + foot_bl,
    + foot_br,
});

impl Skeleton for QuadrupedLowSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 10;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"quadruped_low_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_low_compute_mats")]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
    ) -> Vec3<f32> {
        let chest_mat = base_mat * Mat4::<f32>::from(self.chest);
        let tail_front = chest_mat * Mat4::<f32>::from(self.tail_front);
        let head_lower_mat = chest_mat * Mat4::<f32>::from(self.head_lower);
        let head_upper_mat = head_lower_mat * Mat4::<f32>::from(self.head_upper);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_upper_mat),
            make_bone(head_lower_mat),
            make_bone(head_upper_mat * Mat4::<f32>::from(self.jaw)),
            make_bone(chest_mat),
            make_bone(tail_front),
            make_bone(tail_front * Mat4::<f32>::from(self.tail_rear)),
            make_bone(chest_mat * Mat4::<f32>::from(self.foot_fl)),
            make_bone(chest_mat * Mat4::<f32>::from(self.foot_fr)),
            make_bone(chest_mat * Mat4::<f32>::from(self.foot_bl)),
            make_bone(chest_mat * Mat4::<f32>::from(self.foot_br)),
        ];
        Vec3::default()
    }
}

pub struct SkeletonAttr {
    head_upper: (f32, f32),
    head_lower: (f32, f32),
    jaw: (f32, f32),
    chest: (f32, f32),
    tail_front: (f32, f32),
    tail_rear: (f32, f32),
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
    lean: (f32, f32),
    scaler: f32,
    tempo: f32,
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::QuadrupedLow(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head_upper: (0.0, 0.0),
            head_lower: (0.0, 0.0),
            jaw: (0.0, 0.0),
            chest: (0.0, 0.0),
            tail_front: (0.0, 0.0),
            tail_rear: (0.0, 0.0),
            feet_f: (0.0, 0.0, 0.0),
            feet_b: (0.0, 0.0, 0.0),
            lean: (0.0, 0.0),
            scaler: 0.0,
            tempo: 0.0,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::quadruped_low::Species::*;
        Self {
            head_upper: match (body.species, body.body_type) {
                (Crocodile, _) => (1.5, 2.0),
                (Alligator, _) => (0.5, 2.0),
                (Salamander, _) => (0.5, 2.5),
                (Monitor, _) => (4.5, 1.0),
                (Asp, _) => (6.0, 5.5),
                (Tortoise, _) => (5.0, 1.0),
                (Rocksnapper, _) => (6.0, 0.5),
                (Pangolin, _) => (-0.5, 8.0),
                (Maneater, _) => (7.0, 11.5),
                (Sandshark, _) => (8.5, 0.5),
                (Hakulaq, _) => (8.0, 10.0),
            },
            head_lower: match (body.species, body.body_type) {
                (Crocodile, _) => (8.0, 0.0),
                (Alligator, _) => (9.0, 0.25),
                (Salamander, _) => (9.0, 0.0),
                (Monitor, _) => (10.0, 2.0),
                (Asp, _) => (9.0, 2.5),
                (Tortoise, _) => (12.0, -3.5),
                (Rocksnapper, _) => (12.0, -9.0),
                (Pangolin, _) => (8.0, -9.0),
                (Maneater, _) => (1.0, 4.5),
                (Sandshark, _) => (13.5, -10.5),
                (Hakulaq, _) => (10.5, 1.0),
            },
            jaw: match (body.species, body.body_type) {
                (Crocodile, _) => (2.5, -3.0),
                (Alligator, _) => (2.5, -2.0),
                (Salamander, _) => (0.0, -2.0),
                (Monitor, _) => (-2.0, -1.0),
                (Asp, _) => (-3.0, -2.0),
                (Tortoise, _) => (-3.5, -2.0),
                (Rocksnapper, _) => (-5.0, -1.5),
                (Pangolin, _) => (0.0, 0.0),
                (Maneater, _) => (-1.0, 4.0),
                (Sandshark, _) => (-8.0, -5.5),
                (Hakulaq, _) => (-6.5, -4.0),
            },
            chest: match (body.species, body.body_type) {
                (Crocodile, _) => (0.0, 5.0),
                (Alligator, _) => (0.0, 5.0),
                (Salamander, _) => (0.0, 5.0),
                (Monitor, _) => (0.0, 5.0),
                (Asp, _) => (0.0, 8.0),
                (Tortoise, _) => (0.0, 11.0),
                (Rocksnapper, _) => (0.0, 18.5),
                (Pangolin, _) => (0.0, 7.0),
                (Maneater, _) => (0.0, 12.0),
                (Sandshark, _) => (0.0, 20.0),
                (Hakulaq, _) => (0.0, 13.5),
            },
            tail_rear: match (body.species, body.body_type) {
                (Crocodile, _) => (-12.5, -1.0),
                (Alligator, _) => (-13.0, -1.0),
                (Salamander, _) => (-8.0, 0.0),
                (Monitor, _) => (-12.0, 0.0),
                (Asp, _) => (-14.0, -2.0),
                (Tortoise, _) => (-10.0, -1.5),
                (Rocksnapper, _) => (-14.5, -2.0),
                (Pangolin, _) => (-7.0, -3.0),
                (Maneater, _) => (-15.0, 4.0),
                (Sandshark, _) => (-10.0, 0.5),
                (Hakulaq, _) => (-9.0, -2.0),
            },
            tail_front: match (body.species, body.body_type) {
                (Crocodile, _) => (-6.0, 0.0),
                (Alligator, _) => (-5.0, 0.0),
                (Salamander, _) => (-7.5, 0.0),
                (Monitor, _) => (-6.5, 0.0),
                (Asp, _) => (-6.0, -2.0),
                (Tortoise, _) => (-13.0, -3.5),
                (Rocksnapper, _) => (-13.5, -6.5),
                (Pangolin, _) => (-7.5, -0.5),
                (Maneater, _) => (-1.0, 4.0),
                (Sandshark, _) => (-13.0, -8.0),
                (Hakulaq, _) => (-6.0, -5.5),
            },
            feet_f: match (body.species, body.body_type) {
                (Crocodile, _) => (3.5, 6.0, -1.0),
                (Alligator, _) => (4.5, 4.25, -1.0),
                (Salamander, _) => (5.0, 5.0, -2.0),
                (Monitor, _) => (3.0, 5.0, 0.0),
                (Asp, _) => (1.5, 4.0, -1.0),
                (Tortoise, _) => (5.5, 6.5, -3.0),
                (Rocksnapper, _) => (7.5, 5.0, -8.5),
                (Pangolin, _) => (5.5, 5.5, -1.0),
                (Maneater, _) => (4.5, 4.0, -5.5),
                (Sandshark, _) => (5.5, 2.0, -8.0),
                (Hakulaq, _) => (4.5, 2.0, -4.5),
            },
            feet_b: match (body.species, body.body_type) {
                (Crocodile, _) => (3.5, -6.0, -1.0),
                (Alligator, _) => (4.5, -5.5, -1.0),
                (Salamander, _) => (4.0, -6.0, -2.0),
                (Monitor, _) => (2.5, -6.5, 0.0),
                (Asp, _) => (2.5, -5.5, -1.0),
                (Tortoise, _) => (5.5, -11.5, -3.0),
                (Rocksnapper, _) => (8.0, -12.0, -9.5),
                (Pangolin, _) => (6.5, -3.5, -1.0),
                (Maneater, _) => (4.5, -2.5, -3.0),
                (Sandshark, _) => (3.5, -15.0, -14.0),
                (Hakulaq, _) => (3.5, -8.0, -4.5),
            },
            lean: match (body.species, body.body_type) {
                (Pangolin, _) => (0.4, 0.0),
                _ => (0.0, 1.0),
            },
            scaler: match (body.species, body.body_type) {
                (Crocodile, _) => (1.3),
                (Alligator, _) => (1.5),
                (Salamander, _) => (1.4),
                (Monitor, _) => (1.1),
                (Asp, _) => (1.4),
                (Tortoise, _) => (1.0),
                (Rocksnapper, _) => (1.4),
                (Pangolin, _) => (1.3),
                (Maneater, _) => (1.4),
                (Sandshark, _) => (1.0),
                (Hakulaq, _) => (1.0),
            },
            tempo: match (body.species, body.body_type) {
                (Crocodile, _) => (0.8),
                (Alligator, _) => (0.8),
                (Salamander, _) => (1.0),
                (Monitor, _) => (1.3),
                (Asp, _) => (1.0),
                (Tortoise, _) => (0.9),
                (Rocksnapper, _) => (0.9),
                (Pangolin, _) => (1.15),
                (Maneater, _) => (1.0),
                (Sandshark, _) => (1.0),
                (Hakulaq, _) => (1.0),
            },
        }
    }
}
