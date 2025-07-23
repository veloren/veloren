pub mod fly;
pub mod idle;
pub mod run;

// Reexports
pub use self::{fly::FlyAnimation, idle::IdleAnimation, run::RunAnimation};

use super::{FigureBoneData, Skeleton, vek::*};
use common::comp;
use core::convert::TryFrom;

pub type Body = comp::dragon::Body;

skeleton_impls!(struct DragonSkeleton ComputedDragonSkeleton {
    + head_upper
    + head_lower
    + jaw
    + chest_front
    + chest_rear
    + tail_front
    + tail_rear
    + wing_in_l
    + wing_in_r
    + wing_out_l
    + wing_out_r
    + foot_fl
    + foot_fr
    + foot_bl
    + foot_br
});

impl Skeleton for DragonSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;
    type ComputedSkeleton = ComputedDragonSkeleton;

    const BONE_COUNT: usize = ComputedDragonSkeleton::BONE_COUNT;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"dragon_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "dragon_compute_mats"))]

    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        _body: Self::Body,
    ) -> Self::ComputedSkeleton {
        let base_mat = base_mat * Mat4::scaling_3d(1.0);
        let chest_front_mat = base_mat * Mat4::<f32>::from(self.chest_front);
        let chest_rear_mat = chest_front_mat * Mat4::<f32>::from(self.chest_rear);
        let head_lower_mat = chest_front_mat * Mat4::<f32>::from(self.head_lower);
        let wing_in_l_mat = chest_front_mat * Mat4::<f32>::from(self.wing_in_l);
        let wing_in_r_mat = chest_front_mat * Mat4::<f32>::from(self.wing_in_r);
        let tail_front_mat = chest_rear_mat * Mat4::<f32>::from(self.tail_front);
        let head_upper_mat = head_lower_mat * Mat4::<f32>::from(self.head_upper);

        let computed_skeleton = ComputedDragonSkeleton {
            head_upper: head_upper_mat,
            head_lower: head_lower_mat,
            jaw: head_upper_mat * Mat4::<f32>::from(self.jaw),
            chest_front: chest_front_mat,
            chest_rear: chest_rear_mat,
            tail_front: tail_front_mat,
            tail_rear: tail_front_mat * Mat4::<f32>::from(self.tail_rear),
            wing_in_l: wing_in_l_mat,
            wing_in_r: wing_in_r_mat,
            wing_out_l: wing_in_l_mat * Mat4::<f32>::from(self.wing_out_l),
            wing_out_r: wing_in_r_mat * Mat4::<f32>::from(self.wing_out_r),
            foot_fl: chest_front_mat * Mat4::<f32>::from(self.foot_fl),
            foot_fr: chest_front_mat * Mat4::<f32>::from(self.foot_fr),
            foot_bl: chest_rear_mat * Mat4::<f32>::from(self.foot_bl),
            foot_br: chest_rear_mat * Mat4::<f32>::from(self.foot_br),
        };

        computed_skeleton.set_figure_bone_data(buf);
        computed_skeleton
    }
}

pub fn mount_mat(
    computed_skeleton: &ComputedDragonSkeleton,
    skeleton: &DragonSkeleton,
) -> (Mat4<f32>, Quaternion<f32>) {
    (
        computed_skeleton.chest_front,
        skeleton.chest_front.orientation,
    )
}

pub fn mount_transform(
    body: &Body,
    computed_skeleton: &ComputedDragonSkeleton,
    skeleton: &DragonSkeleton,
) -> Transform<f32, f32, f32> {
    use comp::dragon::Species::*;

    let mount_point = match (body.species, body.body_type) {
        (Reddragon, _) => (0.0, 0.5, 5.5),
    }
    .into();

    let (mount_mat, orientation) = mount_mat(computed_skeleton, skeleton);
    Transform {
        position: mount_mat.mul_point(mount_point),
        orientation,
        scale: Vec3::one(),
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
