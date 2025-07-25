pub mod alpha;
pub mod aura;
pub mod breathe;
pub mod combomelee;
pub mod dash;
pub mod feed;
pub mod fly;
pub mod idle;
pub mod run;
pub mod selfbuff;
pub mod shockwave;
pub mod shoot;
pub mod stunned;
pub mod summon;
pub mod swim;

// Reexports
pub use self::{
    alpha::AlphaAnimation, aura::AuraAnimation, breathe::BreatheAnimation,
    combomelee::ComboAnimation, dash::DashAnimation, feed::FeedAnimation, fly::FlyAnimation,
    idle::IdleAnimation, run::RunAnimation, selfbuff::SelfBuffAnimation,
    shockwave::ShockwaveAnimation, shoot::ShootAnimation, stunned::StunnedAnimation,
    summon::SummonAnimation, swim::SwimAnimation,
};

use super::{FigureBoneData, Skeleton, vek::*};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::bird_large::Body;

skeleton_impls!(struct BirdLargeSkeleton ComputedBirdLargeSkeleton {
    + head
    + beak
    + neck
    + chest
    + tail_front
    + tail_rear
    + wing_in_l
    + wing_in_r
    + wing_mid_l
    + wing_mid_r
    + wing_out_l
    + wing_out_r
    + leg_l
    + leg_r
    + foot_l
    + foot_r
});

impl Skeleton for BirdLargeSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;
    type ComputedSkeleton = ComputedBirdLargeSkeleton;

    const BONE_COUNT: usize = ComputedBirdLargeSkeleton::BONE_COUNT;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"bird_large_compute_mats\0";

    #[cfg_attr(
        feature = "be-dyn-lib",
        unsafe(export_name = "bird_large_compute_mats")
    )]

    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Self::ComputedSkeleton {
        let base_mat = base_mat * Mat4::scaling_3d(SkeletonAttr::from(&body).scaler / 8.0);

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
        let leg_l_mat = base_mat * Mat4::<f32>::from(self.leg_l);
        let leg_r_mat = base_mat * Mat4::<f32>::from(self.leg_r);
        let foot_l_mat = leg_l_mat * Mat4::<f32>::from(self.foot_l);
        let foot_r_mat = leg_r_mat * Mat4::<f32>::from(self.foot_r);

        let computed_skeleton = ComputedBirdLargeSkeleton {
            head: head_mat,
            beak: beak_mat,
            neck: neck_mat,
            chest: chest_mat,
            tail_front: tail_front_mat,
            tail_rear: tail_rear_mat,
            wing_in_l: wing_in_l_mat,
            wing_in_r: wing_in_r_mat,
            wing_mid_l: wing_mid_l_mat,
            wing_mid_r: wing_mid_r_mat,
            wing_out_l: wing_out_l_mat,
            wing_out_r: wing_out_r_mat,
            leg_l: leg_l_mat,
            leg_r: leg_r_mat,
            foot_l: foot_l_mat,
            foot_r: foot_r_mat,
        };

        computed_skeleton.set_figure_bone_data(buf);
        computed_skeleton
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
    feed: f32,
    wyvern: bool,
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
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
            feed: 0.0,
            wyvern: false,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::bird_large::Species::*;
        Self {
            chest: match (body.species, body.body_type) {
                (Phoenix, _) => (2.5, 16.0),
                (Cockatrice, _) => (2.5, 16.0),
                (Roc, _) => (2.5, 27.5),
                (FlameWyvern, _) => (2.5, 20.5),
                (CloudWyvern, _) => (2.5, 20.5),
                (FrostWyvern, _) => (2.5, 20.5),
                (SeaWyvern, _) => (2.5, 20.5),
                (WealdWyvern, _) => (2.5, 20.5),
            },
            neck: match (body.species, body.body_type) {
                (Phoenix, _) => (2.5, -5.5),
                (Cockatrice, _) => (5.0, -1.5),
                (Roc, _) => (9.5, -1.5),
                (FlameWyvern, _) => (11.0, -0.5),
                (CloudWyvern, _) => (11.0, -0.5),
                (FrostWyvern, _) => (11.0, -0.5),
                (SeaWyvern, _) => (11.0, -0.5),
                (WealdWyvern, _) => (11.0, -0.5),
            },
            head: match (body.species, body.body_type) {
                (Phoenix, _) => (6.0, 12.0),
                (Cockatrice, _) => (8.0, 4.5),
                (Roc, _) => (17.0, -3.5),
                (FlameWyvern, _) => (10.0, -1.5),
                (CloudWyvern, _) => (10.0, -1.5),
                (FrostWyvern, _) => (10.0, -1.5),
                (SeaWyvern, _) => (10.0, 2.5),
                (WealdWyvern, _) => (10.0, -1.5),
            },
            beak: match (body.species, body.body_type) {
                (Phoenix, _) => (5.0, 3.0),
                (Cockatrice, _) => (2.0, -3.0),
                (Roc, _) => (0.0, -3.0),
                (FlameWyvern, _) => (-3.0, 2.0),
                (CloudWyvern, _) => (-3.0, 2.0),
                (FrostWyvern, _) => (-3.0, 2.0),
                (SeaWyvern, _) => (-3.0, 2.0),
                (WealdWyvern, _) => (-3.0, 2.0),
            },
            tail_front: match (body.species, body.body_type) {
                (Phoenix, _) => (-9.5, -1.0),
                (Cockatrice, _) => (-5.0, -2.5),
                (Roc, _) => (-7.5, -3.5),
                (FlameWyvern, _) => (-10.0, -5.0),
                (CloudWyvern, _) => (-10.0, -5.0),
                (FrostWyvern, _) => (-10.0, -5.0),
                (SeaWyvern, _) => (-10.0, -5.0),
                (WealdWyvern, _) => (-10.0, -5.0),
            },
            tail_rear: match (body.species, body.body_type) {
                (Phoenix, _) => (-11.0, 0.0),
                (Cockatrice, _) => (-8.0, -3.0),
                (Roc, _) => (-8.0, -3.0),
                (FlameWyvern, _) => (-11.0, -1.0),
                (CloudWyvern, _) => (-11.0, -1.0),
                (FrostWyvern, _) => (-11.0, -1.0),
                (SeaWyvern, _) => (-11.0, -1.0),
                (WealdWyvern, _) => (-11.0, -1.0),
            },
            wing_in: match (body.species, body.body_type) {
                (Phoenix, _) => (3.0, 2.5, 2.0),
                (Cockatrice, _) => (3.5, 7.0, 3.5),
                (Roc, _) => (5.5, 7.5, -1.0),
                (FlameWyvern, _) => (6.5, 11.5, -2.0),
                (CloudWyvern, _) => (3.5, 11.5, -1.5),
                (FrostWyvern, _) => (5.0, 10.5, -1.5),
                (SeaWyvern, _) => (4.0, 11.5, -0.0),
                (WealdWyvern, _) => (5.0, 11.5, -1.0),
            },
            wing_mid: match (body.species, body.body_type) {
                (Phoenix, _) => (10.0, 1.0, 0.0),
                (Cockatrice, _) => (6.0, 0.0, 0.0),
                (Roc, _) => (12.0, 1.0, -0.5),
                (FlameWyvern, _) => (19.0, 11.5, 1.0),
                (CloudWyvern, _) => (19.0, 10.5, 1.0),
                (FrostWyvern, _) => (18.5, 11.5, 0.5),
                (SeaWyvern, _) => (19.0, 11.5, 0.5),
                (WealdWyvern, _) => (19.0, 11.5, 0.0),
            },
            wing_out: match (body.species, body.body_type) {
                (Phoenix, _) => (7.0, 2.0, 1.5),
                (Cockatrice, _) => (4.0, -1.0, 1.0),
                (Roc, _) => (10.0, -2.0, 0.0),
                (FlameWyvern, _) => (11.0, -1.0, 0.0),
                (CloudWyvern, _) => (11.0, -2.0, 0.0),
                (FrostWyvern, _) => (10.0, -1.5, 0.5),
                (SeaWyvern, _) => (12.0, -1.0, 0.0),
                (WealdWyvern, _) => (16.0, -4.0, -1.0),
            },
            leg: match (body.species, body.body_type) {
                (Phoenix, _) => (4.0, 1.5, 12.0),
                (Cockatrice, _) => (3.5, 2.5, 13.0),
                (Roc, _) => (5.5, -1.5, 17.5),
                (FlameWyvern, _) => (5.5, 2.0, 15.5),
                (CloudWyvern, _) => (5.5, 2.0, 15.5),
                (FrostWyvern, _) => (5.5, 2.0, 15.5),
                (SeaWyvern, _) => (5.5, 2.0, 15.5),
                (WealdWyvern, _) => (5.5, 2.0, 15.5),
            },
            foot: match (body.species, body.body_type) {
                (Phoenix, _) => (0.5, -0.5, -2.5),
                (Cockatrice, _) => (0.5, -3.0, -3.0),
                (Roc, _) => (2.5, -2.5, -5.5),
                (FlameWyvern, _) => (0.5, 0.0, -3.5),
                (CloudWyvern, _) => (0.5, 0.0, -3.5),
                (FrostWyvern, _) => (0.5, 0.0, -3.5),
                (SeaWyvern, _) => (0.5, 0.0, -3.5),
                (WealdWyvern, _) => (0.5, 0.0, -3.5),
            },
            scaler: match (body.species, body.body_type) {
                (Phoenix, _) => 1.0,
                (Cockatrice, _) => 1.0,
                (Roc, _) => 1.0,
                (FlameWyvern, _)
                | (CloudWyvern, _)
                | (FrostWyvern, _)
                | (SeaWyvern, _)
                | (WealdWyvern, _) => 1.0,
            },
            feed: match (body.species, body.body_type) {
                (Phoenix, _) => -0.65,
                (Cockatrice, _) => -0.5,
                (Roc, _) => -0.4,
                (FlameWyvern, _)
                | (CloudWyvern, _)
                | (FrostWyvern, _)
                | (SeaWyvern, _)
                | (WealdWyvern, _) => -0.65,
            },
            wyvern: matches!(
                (body.species, body.body_type),
                (FlameWyvern, _)
                    | (CloudWyvern, _)
                    | (FrostWyvern, _)
                    | (SeaWyvern, _)
                    | (WealdWyvern, _)
            ),
        }
    }
}

pub fn mount_mat(
    body: &Body,
    computed_skeleton: &ComputedBirdLargeSkeleton,
    skeleton: &BirdLargeSkeleton,
) -> (Mat4<f32>, Quaternion<f32>) {
    use comp::bird_large::Species::*;

    match (body.species, body.body_type) {
        (SeaWyvern | FlameWyvern | Phoenix | Cockatrice, _) => {
            (computed_skeleton.chest, skeleton.chest.orientation)
        },
        _ => (
            computed_skeleton.neck,
            skeleton.chest.orientation * skeleton.neck.orientation,
        ),
    }
}

pub fn mount_transform(
    body: &Body,
    computed_skeleton: &ComputedBirdLargeSkeleton,
    skeleton: &BirdLargeSkeleton,
) -> Transform<f32, f32, f32> {
    use comp::bird_large::Species::*;

    let mount_point = match (body.species, body.body_type) {
        (Phoenix, _) => (0.0, 0.5, 7.5),
        (Cockatrice, _) => (0.0, 5.0, 6.5),
        (Roc, _) => (0.0, 5.5, 6.5),
        (FlameWyvern, _) => (0.0, 9.5, 7.0),
        (CloudWyvern, _) => (0.0, -0.5, 6.5),
        (FrostWyvern, _) => (0.0, 0.5, 6.0),
        (SeaWyvern, _) => (0.0, 8.0, 7.0),
        (WealdWyvern, _) => (0.0, 0.0, 9.0),
    }
    .into();

    let (mount_mat, orientation) = mount_mat(body, computed_skeleton, skeleton);
    Transform {
        position: mount_mat.mul_point(mount_point),
        orientation,
        scale: Vec3::one(),
    }
}
