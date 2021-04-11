pub mod feed;
pub mod fly;
pub mod idle;
pub mod run;

// Reexports
pub use self::{feed::FeedAnimation, fly::FlyAnimation, idle::IdleAnimation, run::RunAnimation};

use super::{make_bone, vek::*, FigureBoneData, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::bird_large::Body;

skeleton_impls!(struct BirdLargeSkeleton {
    + head,
    + beak,
    + neck,
    + chest,
    + tail_front,
    + tail_rear,
    + wing_in_l,
    + wing_in_r,
    + wing_mid_l,
    + wing_mid_r,
    + wing_out_l,
    + wing_out_r,
    + leg_l,
    + leg_r,
    + foot_l,
    + foot_r,
});

impl Skeleton for BirdLargeSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 16;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"bird_large_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "bird_large_compute_mats")]

    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
    ) -> Vec3<f32> {
        let chest_mat = base_mat * Mat4::<f32>::from(self.chest);
        let neck_mat = chest_mat * Mat4::<f32>::from(self.neck);
        let head_mat = neck_mat * Mat4::<f32>::from(self.head);
        let beak_mat = head_mat * Mat4::<f32>::from(self.beak);
        let tail_front_mat = chest_mat * Mat4::<f32>::from(self.tail_front);
        let tail_rear_mat = tail_front_mat * Mat4::<f32>::from(self.tail_rear);
        let wing_in_l_mat = chest_mat * Mat4::<f32>::from(self.wing_in_l);
        let wing_in_r_mat = chest_mat * Mat4::<f32>::from(self.wing_in_r);
        let wing_mid_l_mat = wing_in_l_mat * Mat4::<f32>::from(self.wing_mid_l);
        let wing_mid_r_mat = wing_in_r_mat * Mat4::<f32>::from(self.wing_mid_r);
        let wing_out_l_mat = wing_mid_l_mat * Mat4::<f32>::from(self.wing_out_l);
        let wing_out_r_mat = wing_mid_r_mat * Mat4::<f32>::from(self.wing_out_r);
        let leg_l_mat = chest_mat * Mat4::<f32>::from(self.leg_l);
        let leg_r_mat = chest_mat * Mat4::<f32>::from(self.leg_r);
        let foot_l_mat = leg_l_mat * Mat4::<f32>::from(self.foot_l);
        let foot_r_mat = leg_r_mat * Mat4::<f32>::from(self.foot_r);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_mat),
            make_bone(beak_mat),
            make_bone(neck_mat),
            make_bone(chest_mat),
            make_bone(tail_front_mat),
            make_bone(tail_rear_mat),
            make_bone(wing_in_l_mat),
            make_bone(wing_in_r_mat),
            make_bone(wing_mid_l_mat),
            make_bone(wing_mid_r_mat),
            make_bone(wing_out_l_mat),
            make_bone(wing_out_r_mat),
            make_bone(leg_l_mat),
            make_bone(leg_r_mat),
            make_bone(foot_l_mat),
            make_bone(foot_r_mat),
        ];
        Vec3::default()
    }
}

pub struct SkeletonAttr {
    chest: (f32, f32),
    neck: (f32, f32),
    head: (f32, f32),
    beak: (f32, f32),
    tail_front: (f32, f32),
    tail_rear: (f32, f32),
    wing_in: (f32, f32, f32),
    wing_mid: (f32, f32, f32),
    wing_out: (f32, f32, f32),
    leg: (f32, f32, f32),
    foot: (f32, f32, f32),
    scaler: f32,
    wings_angle: f32,
    flight_angle: f32,
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::BirdLarge(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            chest: (0.0, 0.0),
            neck: (0.0, 0.0),
            head: (0.0, 0.0),
            beak: (0.0, 0.0),
            tail_front: (0.0, 0.0),
            tail_rear: (0.0, 0.0),
            wing_in: (0.0, 0.0, 0.0),
            wing_mid: (0.0, 0.0, 0.0),
            wing_out: (0.0, 0.0, 0.0),
            leg: (0.0, 0.0, 0.0),
            foot: (0.0, 0.0, 0.0),
            scaler: 0.0,
            wings_angle: 0.0,
            flight_angle: 0.0,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::bird_large::Species::*;
        Self {
            chest: match (body.species, body.body_type) {
                (Phoenix, _) => (2.5, 8.0),
                (Cockatrice, _) => (2.5, 16.0),
            },
            neck: match (body.species, body.body_type) {
                (Phoenix, _) => (0.5, 3.0),
                (Cockatrice, _) => (5.0, -1.5),
            },
            head: match (body.species, body.body_type) {
                (Phoenix, _) => (2.0, 2.0),
                (Cockatrice, _) => (8.0, 4.5),
            },
            beak: match (body.species, body.body_type) {
                (Phoenix, _) => (2.0, 1.0),
                (Cockatrice, _) => (2.0, -3.0),
            },
            tail_front: match (body.species, body.body_type) {
                (Phoenix, _) => (-5.5, -2.0),
                (Cockatrice, _) => (-5.0, -2.5),
            },
            tail_rear: match (body.species, body.body_type) {
                (Phoenix, _) => (-3.0, -3.0),
                (Cockatrice, _) => (-8.0, -3.0),
            },
            wing_in: match (body.species, body.body_type) {
                (Phoenix, _) => (3.0, 2.5, 3.0),
                (Cockatrice, _) => (3.5, 7.0, 3.5),
            },
            wing_mid: match (body.species, body.body_type) {
                (Phoenix, _) => (6.5, -1.0, 0.0),
                (Cockatrice, _) => (6.0, 0.0, 0.0),
            },
            wing_out: match (body.species, body.body_type) {
                (Phoenix, _) => (0.5, -1.0, 0.0),
                (Cockatrice, _) => (4.0, -1.0, 1.0),
            },
            leg: match (body.species, body.body_type) {
                (Phoenix, _) => (2.5, -2.5, -3.5),
                (Cockatrice, _) => (2.5, 2.5, -3.5),
            },
            foot: match (body.species, body.body_type) {
                (Phoenix, _) => (0.0, -0.5, -0.5),
                (Cockatrice, _) => (1.5, -3.0, -3.0),
            },
            scaler: match (body.species, body.body_type) {
                (Phoenix, _) => (1.0),
                (Cockatrice, _) => (1.0),
            },
            wings_angle: match (body.species, body.body_type) {
                (Phoenix, _) => (1.3),
                (Cockatrice, _) => (0.9),
            },
            flight_angle: match (body.species, body.body_type) {
                (Phoenix, _) => (-0.5),
                (Cockatrice, _) => (1.0),
            },
        }
    }
}
