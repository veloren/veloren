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

pub type Body = comp::quadruped_medium::Body;

skeleton_impls!(struct QuadrupedMediumSkeleton {
    + head_upper,
    + head_lower,
    + jaw,
    + tail,
    + torso_front,
    + torso_back,
    + ears,
    + leg_fl,
    + leg_fr,
    + leg_bl,
    + leg_br,
    + foot_fl,
    + foot_fr,
    + foot_bl,
    + foot_br,
});

impl Skeleton for QuadrupedMediumSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 15;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"quadruped_medium_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_medium_compute_mats")]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
    ) -> Vec3<f32> {
        let torso_front_mat = base_mat * Mat4::<f32>::from(self.torso_front);
        let torso_back_mat = torso_front_mat * Mat4::<f32>::from(self.torso_back);
        let head_lower_mat = torso_front_mat * Mat4::<f32>::from(self.head_lower);
        let leg_fl_mat = torso_front_mat * Mat4::<f32>::from(self.leg_fl);
        let leg_fr_mat = torso_front_mat * Mat4::<f32>::from(self.leg_fr);
        let leg_bl_mat = torso_back_mat * Mat4::<f32>::from(self.leg_bl);
        let leg_br_mat = torso_back_mat * Mat4::<f32>::from(self.leg_br);
        let head_upper_mat = head_lower_mat * Mat4::<f32>::from(self.head_upper);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_upper_mat),
            make_bone(head_lower_mat),
            make_bone(head_upper_mat * Mat4::<f32>::from(self.jaw)),
            make_bone(torso_back_mat * Mat4::<f32>::from(self.tail)),
            make_bone(torso_front_mat),
            make_bone(torso_back_mat),
            make_bone(head_upper_mat * Mat4::<f32>::from(self.ears)),
            make_bone(leg_fl_mat),
            make_bone(leg_fr_mat),
            make_bone(leg_bl_mat),
            make_bone(leg_br_mat),
            make_bone(leg_fl_mat * Mat4::<f32>::from(self.foot_fl)),
            make_bone(leg_fr_mat * Mat4::<f32>::from(self.foot_fr)),
            make_bone(leg_bl_mat * Mat4::<f32>::from(self.foot_bl)),
            make_bone(leg_br_mat * Mat4::<f32>::from(self.foot_br)),
        ];
        Vec3::default()
    }
}

pub struct SkeletonAttr {
    head_upper: (f32, f32),
    head_lower: (f32, f32),
    jaw: (f32, f32),
    tail: (f32, f32),
    torso_back: (f32, f32),
    torso_front: (f32, f32),
    ears: (f32, f32),
    leg_f: (f32, f32, f32),
    leg_b: (f32, f32, f32),
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
    scaler: f32,
    dampen: f32,
    maximize: f32,
    tempo: f32,
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::QuadrupedMedium(body) => Ok(SkeletonAttr::from(body)),
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
            tail: (0.0, 0.0),
            torso_back: (0.0, 0.0),
            torso_front: (0.0, 0.0),
            ears: (0.0, 0.0),
            leg_f: (0.0, 0.0, 0.0),
            leg_b: (0.0, 0.0, 0.0),
            feet_f: (0.0, 0.0, 0.0),
            feet_b: (0.0, 0.0, 0.0),
            scaler: 0.0,
            dampen: 0.0,
            maximize: 0.0,
            tempo: 0.0,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::quadruped_medium::Species::*;
        Self {
            head_upper: match (body.species, body.body_type) {
                (Grolgar, _) => (0.0, -1.0),
                (Saber, _) => (0.0, -3.0),
                (Tuskram, _) => (0.0, 1.0),
                (Lion, _) => (2.5, 2.0),
                (Tarasque, _) => (-4.0, 3.5),
                (Tiger, _) => (2.0, 1.0),
                (Wolf, _) => (-0.5, 3.0),
                (Frostfang, _) => (1.0, -2.0),
                (Mouflon, _) => (-2.5, 6.0),
                (Catoblepas, _) => (-1.0, -6.5),
                (Bonerattler, _) => (-1.0, 2.5),
            },
            head_lower: match (body.species, body.body_type) {
                (Grolgar, _) => (1.0, -1.0),
                (Saber, _) => (1.0, 0.0),
                (Tuskram, _) => (1.0, 1.0),
                (Lion, _) => (0.5, 1.0),
                (Tarasque, _) => (0.5, -4.0),
                (Tiger, _) => (0.0, 0.0),
                (Wolf, _) => (-4.5, 2.0),
                (Frostfang, _) => (2.0, 1.5),
                (Mouflon, _) => (-1.0, 0.5),
                (Catoblepas, _) => (19.5, -2.0),
                (Bonerattler, _) => (7.0, -1.5),
            },
            jaw: match (body.species, body.body_type) {
                (Grolgar, _) => (7.0, 1.5),
                (Saber, _) => (2.5, -2.0),
                (Tuskram, _) => (5.5, -4.0),
                (Lion, _) => (3.5, -4.5),
                (Tarasque, _) => (9.0, -10.0),
                (Tiger, _) => (3.5, -4.0),
                (Wolf, _) => (5.0, -3.0),
                (Frostfang, _) => (4.0, -3.0),
                (Mouflon, _) => (10.5, -4.0),
                (Catoblepas, _) => (1.0, -4.0),
                (Bonerattler, _) => (3.0, -3.0),
            },
            tail: match (body.species, body.body_type) {
                (Grolgar, _) => (-11.5, -0.5),
                (Saber, _) => (-11.0, 1.0),
                (Tuskram, _) => (-9.0, 2.0),
                (Lion, _) => (-11.0, 1.0),
                (Tarasque, _) => (-11.0, 0.0),
                (Tiger, _) => (-13.5, -7.0),
                (Wolf, _) => (-11.0, 0.0),
                (Frostfang, _) => (-7.0, -3.5),
                (Mouflon, _) => (-10.5, 3.0),
                (Catoblepas, _) => (-8.5, -2.0),
                (Bonerattler, _) => (-10.0, 1.5),
            },
            torso_front: match (body.species, body.body_type) {
                (Grolgar, _) => (10.0, 13.0),
                (Saber, _) => (14.0, 14.0),
                (Tuskram, _) => (10.0, 14.5),
                (Lion, _) => (10.0, 14.0),
                (Tarasque, _) => (11.5, 18.5),
                (Tiger, _) => (10.0, 14.0),
                (Wolf, _) => (12.0, 13.0),
                (Frostfang, _) => (9.0, 11.5),
                (Mouflon, _) => (11.0, 13.5),
                (Catoblepas, _) => (7.5, 19.5),
                (Bonerattler, _) => (6.0, 12.5),
            },
            torso_back: match (body.species, body.body_type) {
                (Grolgar, _) => (-10.0, 1.5),
                (Saber, _) => (-13.5, 0.0),
                (Tuskram, _) => (-12.5, -2.0),
                (Lion, _) => (-12.0, -0.5),
                (Tarasque, _) => (-14.0, -1.0),
                (Tiger, _) => (-13.0, 0.0),
                (Wolf, _) => (-12.5, 1.0),
                (Frostfang, _) => (-10.5, 0.0),
                (Mouflon, _) => (-8.5, -0.5),
                (Catoblepas, _) => (-8.5, -4.5),
                (Bonerattler, _) => (-5.0, 0.0),
            },
            ears: match (body.species, body.body_type) {
                (Grolgar, _) => (5.0, 8.0),
                (Saber, _) => (3.0, 5.5),
                (Tuskram, _) => (5.5, 12.0),
                (Lion, _) => (2.0, 3.5),
                (Tarasque, _) => (11.0, -3.0),
                (Tiger, _) => (2.5, 4.0),
                (Wolf, _) => (3.0, 2.5),
                (Frostfang, _) => (2.0, 3.5),
                (Mouflon, _) => (2.5, 5.0),
                (Catoblepas, _) => (11.0, -3.0),
                (Bonerattler, _) => (2.0, 3.5),
            },
            leg_f: match (body.species, body.body_type) {
                (Grolgar, _) => (-7.0, 4.0, 0.0),
                (Saber, _) => (7.0, -4.0, -3.5),
                (Tuskram, _) => (6.0, -6.5, -0.5),
                (Lion, _) => (6.5, -6.5, -2.0),
                (Tarasque, _) => (7.0, -8.0, -6.0),
                (Tiger, _) => (6.0, -5.0, -3.0),
                (Wolf, _) => (4.5, -6.5, -1.0),
                (Frostfang, _) => (5.5, -5.5, -2.0),
                (Mouflon, _) => (4.0, -5.0, -5.0),
                (Catoblepas, _) => (7.0, 2.0, -6.0),
                (Bonerattler, _) => (5.5, 5.0, -4.0),
            },
            leg_b: match (body.species, body.body_type) {
                (Grolgar, _) => (6.0, -6.5, -5.5),
                (Saber, _) => (6.0, -7.0, -3.5),
                (Tuskram, _) => (5.0, -5.5, -3.5),
                (Lion, _) => (6.0, -6.0, -2.0),
                (Tarasque, _) => (6.0, -6.5, -6.5),
                (Tiger, _) => (6.0, -7.5, -3.0),
                (Wolf, _) => (5.0, -6.5, -2.5),
                (Frostfang, _) => (3.5, -4.5, -2.0),
                (Mouflon, _) => (3.5, -8.0, -4.5),
                (Catoblepas, _) => (6.0, -2.5, -2.5),
                (Bonerattler, _) => (6.0, -8.0, -4.0),
            },
            feet_f: match (body.species, body.body_type) {
                (Grolgar, _) => (0.0, -9.0, -7.0),
                (Saber, _) => (1.0, -3.5, -2.5),
                (Tuskram, _) => (0.5, 0.5, -9.0),
                (Lion, _) => (0.0, 0.0, -7.0),
                (Tarasque, _) => (1.0, 0.0, -3.0),
                (Tiger, _) => (0.5, 0.0, -5.0),
                (Wolf, _) => (0.5, 0.0, -2.0),
                (Frostfang, _) => (0.5, 1.5, -3.5),
                (Mouflon, _) => (-0.5, -0.5, -1.5),
                (Catoblepas, _) => (1.0, 4.0, -3.0),
                (Bonerattler, _) => (-0.5, -3.0, -2.5),
            },
            feet_b: match (body.species, body.body_type) {
                (Grolgar, _) => (0.0, 0.0, -5.0),
                (Saber, _) => (1.0, -1.0, -1.0),
                (Tuskram, _) => (0.5, 0.0, -3.0),
                (Lion, _) => (0.5, 0.5, -5.5),
                (Tarasque, _) => (1.5, -1.0, -2.5),
                (Tiger, _) => (1.0, 0.5, -4.0),
                (Wolf, _) => (0.0, -1.0, -1.5),
                (Frostfang, _) => (0.0, -1.5, -3.5),
                (Mouflon, _) => (-1.0, 0.0, -2.5),
                (Catoblepas, _) => (0.5, 0.5, -3.0),
                (Bonerattler, _) => (0.0, 3.0, -2.5),
            },
            scaler: match (body.species, body.body_type) {
                (Grolgar, _) => (1.3),
                (Saber, _) => (0.9),
                (Tuskram, _) => (1.2),
                (Lion, _) => (1.3),
                (Tarasque, _) => (1.3),
                (Tiger, _) => (1.2),
                (Wolf, _) => (1.0),
                (Frostfang, _) => (1.0),
                (Mouflon, _) => (1.0),
                (Catoblepas, _) => (1.3),
                (Bonerattler, _) => (1.0),
            },
            dampen: match (body.species, body.body_type) {
                (Grolgar, _) => (0.5),
                (Saber, _) => (0.5),
                (Tuskram, _) => (0.6),
                (Lion, _) => (0.8),
                (Tarasque, _) => (0.6),
                (Tiger, _) => (0.6),
                (Wolf, _) => (1.0),
                (Frostfang, _) => (1.0),
                (Mouflon, _) => (1.0),
                (Catoblepas, _) => (0.6),
                (Bonerattler, _) => (0.6),
            },
            maximize: match (body.species, body.body_type) {
                (Grolgar, _) => (2.0),
                (Saber, _) => (1.5),
                (Tuskram, _) => (1.0),
                (Lion, _) => (1.1),
                (Tarasque, _) => (1.8),
                (Tiger, _) => (1.8),
                (Wolf, _) => (1.0),
                (Frostfang, _) => (1.2),
                (Mouflon, _) => (1.1),
                (Catoblepas, _) => (0.9),
                (Bonerattler, _) => (0.8),
            },
            tempo: match (body.species, body.body_type) {
                (Grolgar, _) => (0.95),
                (Saber, _) => (1.1),
                (Tuskram, _) => (0.9),
                (Lion, _) => (0.95),
                (Tarasque, _) => (0.95),
                (Tiger, _) => (1.0),
                (Wolf, _) => (1.1),
                (Frostfang, _) => (1.0),
                (Mouflon, _) => (0.85),
                (Catoblepas, _) => (0.8),
                (Bonerattler, _) => (1.0),
            },
        }
    }
}
