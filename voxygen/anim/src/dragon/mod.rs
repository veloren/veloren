pub mod fly;
pub mod idle;
pub mod run;

// Reexports
pub use self::{fly::FlyAnimation, idle::IdleAnimation, run::RunAnimation};

use super::{make_bone, vek::*, FigureBoneData, Offsets, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::dragon::Body;

skeleton_impls!(struct DragonSkeleton {
    + head_upper,
    + head_lower,
    + jaw,
    + chest_front,
    + chest_rear,
    + tail_front,
    + tail_rear,
    + wing_in_l,
    + wing_in_r,
    + wing_out_l,
    + wing_out_r,
    + foot_fl,
    + foot_fr,
    + foot_bl,
    + foot_br,
});

impl Skeleton for DragonSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 15;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"dragon_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "dragon_compute_mats")]

    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets {
        let base_mat = base_mat * Mat4::scaling_3d(1.0);
        let chest_front_mat = base_mat * Mat4::<f32>::from(self.chest_front);
        let chest_rear_mat = chest_front_mat * Mat4::<f32>::from(self.chest_rear);
        let head_lower_mat = chest_front_mat * Mat4::<f32>::from(self.head_lower);
        let wing_in_l_mat = chest_front_mat * Mat4::<f32>::from(self.wing_in_l);
        let wing_in_r_mat = chest_front_mat * Mat4::<f32>::from(self.wing_in_r);
        let tail_front_mat = chest_rear_mat * Mat4::<f32>::from(self.tail_front);
        let head_upper_mat = head_lower_mat * Mat4::<f32>::from(self.head_upper);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_upper_mat),
            make_bone(head_lower_mat),
            make_bone(head_upper_mat * Mat4::<f32>::from(self.jaw)),
            make_bone(chest_front_mat),
            make_bone(chest_rear_mat),
            make_bone(tail_front_mat),
            make_bone(tail_front_mat * Mat4::<f32>::from(self.tail_rear)),
            make_bone(wing_in_l_mat),
            make_bone(wing_in_r_mat),
            make_bone(wing_in_l_mat * Mat4::<f32>::from(self.wing_out_l)),
            make_bone(wing_in_r_mat * Mat4::<f32>::from(self.wing_out_r)),
            make_bone(chest_front_mat * Mat4::<f32>::from(self.foot_fl)),
            make_bone(chest_front_mat * Mat4::<f32>::from(self.foot_fr)),
            make_bone(chest_rear_mat * Mat4::<f32>::from(self.foot_bl)),
            make_bone(chest_rear_mat * Mat4::<f32>::from(self.foot_br)),
        ];
        Offsets {
            lantern: None,
            viewpoint: Some((head_upper_mat * Vec4::new(0.0, 8.0, 0.0, 1.0)).xyz()),
            // TODO: see quadruped_medium for how to animate this
            mount_bone: Transform {
                position: comp::Body::Dragon(body).mount_offset().into_tuple().into(),
                ..Default::default()
            },
            primary_trail_mat: None,
            secondary_trail_mat: None,
        }
    }
}

pub struct SkeletonAttr {
    head_upper: (f32, f32),
    head_lower: (f32, f32),
    jaw: (f32, f32),
    chest_front: (f32, f32),
    chest_rear: (f32, f32),
    tail_front: (f32, f32),
    tail_rear: (f32, f32),
    wing_in: (f32, f32, f32),
    wing_out: (f32, f32, f32),
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
    height: f32,
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::Dragon(body) => Ok(SkeletonAttr::from(body)),
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
            chest_front: (0.0, 0.0),
            chest_rear: (0.0, 0.0),
            tail_front: (0.0, 0.0),
            tail_rear: (0.0, 0.0),
            wing_in: (0.0, 0.0, 0.0),
            wing_out: (0.0, 0.0, 0.0),
            feet_f: (0.0, 0.0, 0.0),
            feet_b: (0.0, 0.0, 0.0),
            height: (0.0),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::dragon::Species::*;
        Self {
            head_upper: match (body.species, body.body_type) {
                (Reddragon, _) => (2.5, 4.5),
            },
            head_lower: match (body.species, body.body_type) {
                (Reddragon, _) => (7.5, 3.5),
            },
            jaw: match (body.species, body.body_type) {
                (Reddragon, _) => (6.5, -5.0),
            },
            chest_front: match (body.species, body.body_type) {
                (Reddragon, _) => (0.0, 15.0),
            },
            chest_rear: match (body.species, body.body_type) {
                (Reddragon, _) => (-6.5, 0.0),
            },
            tail_front: match (body.species, body.body_type) {
                (Reddragon, _) => (-6.5, 1.5),
            },
            tail_rear: match (body.species, body.body_type) {
                (Reddragon, _) => (-11.5, -1.0),
            },
            wing_in: match (body.species, body.body_type) {
                (Reddragon, _) => (2.5, -16.5, 0.0),
            },
            wing_out: match (body.species, body.body_type) {
                (Reddragon, _) => (23.0, 0.5, 4.0),
            },
            feet_f: match (body.species, body.body_type) {
                (Reddragon, _) => (6.0, 1.0, -13.0),
            },
            feet_b: match (body.species, body.body_type) {
                (Reddragon, _) => (6.0, -2.0, -10.5),
            },
            height: match (body.species, body.body_type) {
                (Reddragon, _) => 1.0,
            },
        }
    }
}
