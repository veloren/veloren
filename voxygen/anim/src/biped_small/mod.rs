pub mod alpha;
pub mod beam;
pub mod dash;
pub mod idle;
pub mod leapmelee;
pub mod run;
pub mod shockwave;
pub mod shoot;
pub mod spinmelee;
pub mod stunned;
pub mod summon;
pub mod wield;

// Reexports
pub use self::{
    alpha::AlphaAnimation, beam::BeamAnimation, dash::DashAnimation, idle::IdleAnimation,
    leapmelee::LeapAnimation, run::RunAnimation, shockwave::ShockwaveAnimation,
    shoot::ShootAnimation, spinmelee::SpinMeleeAnimation, stunned::StunnedAnimation,
    summon::SummonAnimation, wield::WieldAnimation,
};

use super::{make_bone, vek::*, FigureBoneData, Offsets, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::biped_small::Body;

skeleton_impls!(struct BipedSmallSkeleton {
    + head,
    + chest,
    + pants,
    + tail,
    + main,
    + hand_l,
    + hand_r,
    + foot_l,
    + foot_r,
    control,
    control_l,
    control_r,

});

impl Skeleton for BipedSmallSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 9;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"biped_small_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_compute_mats")]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
        body: Self::Body,
    ) -> Offsets {
        let base_mat = base_mat * Mat4::scaling_3d(SkeletonAttr::from(&body).scaler / 11.0);

        let chest_mat = base_mat * Mat4::<f32>::from(self.chest);
        let pants_mat = chest_mat * Mat4::<f32>::from(self.pants);
        let control_mat = chest_mat * Mat4::<f32>::from(self.control);
        let control_l_mat = Mat4::<f32>::from(self.control_l);
        let control_r_mat = Mat4::<f32>::from(self.control_r);
        let head_mat = chest_mat * Mat4::<f32>::from(self.head);
        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_mat),
            make_bone(chest_mat),
            make_bone(pants_mat),
            make_bone(pants_mat * Mat4::<f32>::from(self.tail)),
            make_bone(control_mat * Mat4::<f32>::from(self.main)),
            make_bone(control_mat * control_l_mat * Mat4::<f32>::from(self.hand_l)),
            make_bone(control_mat * control_r_mat * Mat4::<f32>::from(self.hand_r)),
            make_bone(base_mat * Mat4::<f32>::from(self.foot_l)),
            make_bone(base_mat * Mat4::<f32>::from(self.foot_r)),
        ];
        Offsets {
            lantern: None,
            viewpoint: Some((head_mat * Vec4::new(0.0, 0.0, 0.0, 1.0)).xyz()),
            // TODO: see quadruped_medium for how to animate this
            mount_bone: Transform {
                position: comp::Body::BipedSmall(body)
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
    chest: (f32, f32),
    pants: (f32, f32),
    tail: (f32, f32),
    hand: (f32, f32, f32),
    foot: (f32, f32, f32),
    grip: (f32, f32, f32),
    scaler: f32,
}

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::BipedSmall(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            chest: (0.0, 0.0),
            pants: (0.0, 0.0),
            tail: (0.0, 0.0),
            hand: (0.0, 0.0, 0.0),
            foot: (0.0, 0.0, 0.0),
            grip: (0.0, 0.0, 0.0),
            scaler: 0.0,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::biped_small::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Gnome, _) => (-1.0, 9.0),
                (Sahagin, _) => (7.0, -3.5),
                (Adlet, _) => (0.0, 7.0),
                (Gnarling, _) => (0.0, 6.0),
                (Mandragora, _) => (-1.0, 9.0),
                (Kappa, _) => (8.0, 3.5),
                (Cactid, _) => (0.0, 7.0),
                (Gnoll, _) => (5.5, -1.0),
                (Haniwa, _) => (0.0, 7.0),
                (Myrmidon, _) => (0.0, 8.0),
                (Husk, _) => (0.5, 8.5),
                (Boreal, _) => (-0.5, 13.0),
            },
            chest: match (body.species, body.body_type) {
                (Gnome, _) => (0.0, 9.0),
                (Sahagin, _) => (0.0, 15.0),
                (Adlet, _) => (0.0, 11.0),
                (Gnarling, _) => (0.0, 7.5),
                (Mandragora, _) => (0.0, 4.0),
                (Kappa, _) => (0.0, 14.5),
                (Cactid, _) => (0.0, 7.0),
                (Gnoll, _) => (0.0, 15.5),
                (Haniwa, _) => (0.0, 11.0),
                (Myrmidon, _) => (0.0, 11.0),
                (Husk, _) => (0.0, 13.0),
                (Boreal, _) => (0.0, 12.0),
            },
            pants: match (body.species, body.body_type) {
                (Gnome, _) => (0.0, -3.0),
                (Sahagin, _) => (0.5, -7.0),
                (Adlet, _) => (0.0, -3.0),
                (Gnarling, _) => (0.0, -3.0),
                (Mandragora, _) => (0.0, 0.0),
                (Kappa, _) => (0.0, -3.0),
                (Cactid, _) => (0.0, -3.0),
                (Gnoll, _) => (0.5, -7.5),
                (Haniwa, _) => (0.0, -3.5),
                (Myrmidon, _) => (0.0, -3.0),
                (Husk, _) => (-1.0, -3.0),
                (Boreal, _) => (1.5, -5.0),
            },
            tail: match (body.species, body.body_type) {
                (Gnome, _) => (0.0, 0.0),
                (Sahagin, _) => (-2.5, -2.0),
                (Adlet, _) => (-4.5, -2.0),
                (Gnarling, _) => (-2.0, 1.5),
                (Mandragora, _) => (0.0, -1.0),
                (Kappa, _) => (0.0, -4.0),
                (Cactid, _) => (0.0, 0.0),
                (Gnoll, _) => (-2.5, -2.0),
                (Haniwa, _) => (-4.5, -2.0),
                (Myrmidon, _) => (-2.5, -1.0),
                (Husk, _) => (0.0, 0.0),
                (Boreal, _) => (0.0, 0.0),
            },
            hand: match (body.species, body.body_type) {
                (Gnome, _) => (4.0, 0.5, -1.0),
                (Sahagin, _) => (3.5, 3.5, -2.0),
                (Adlet, _) => (4.5, -0.5, 2.0),
                (Gnarling, _) => (4.0, 0.0, 1.5),
                (Mandragora, _) => (4.0, -0.5, 4.0),
                (Kappa, _) => (4.0, 3.5, -0.5),
                (Cactid, _) => (4.0, 0.5, -1.0),
                (Gnoll, _) => (3.5, 0.5, -1.0),
                (Haniwa, _) => (4.25, -1.0, 1.5),
                (Myrmidon, _) => (3.5, 1.5, 2.0),
                (Husk, _) => (4.0, 0.0, 1.0),
                (Boreal, _) => (5.0, 0.5, 5.0),
            },
            foot: match (body.species, body.body_type) {
                (Gnome, _) => (3.0, 0.0, 4.0),
                (Sahagin, _) => (3.0, 1.0, 8.0),
                (Adlet, _) => (3.0, 0.5, 7.0),
                (Gnarling, _) => (2.5, 1.0, 5.0),
                (Mandragora, _) => (3.0, 0.0, 4.0),
                (Kappa, _) => (3.0, 3.0, 9.0),
                (Cactid, _) => (3.0, 0.0, 5.0),
                (Gnoll, _) => (3.0, 1.0, 7.0),
                (Haniwa, _) => (3.0, 0.5, 8.0),
                (Myrmidon, _) => (3.0, 0.5, 7.0),
                (Husk, _) => (4.0, 0.5, 7.0),
                (Boreal, _) => (3.0, 0.0, 9.0),
            },
            grip: match (body.species, body.body_type) {
                (Gnome, _) => (0.0, 0.0, 5.0),
                (Sahagin, _) => (1.0, 0.0, 13.0),
                (Adlet, _) => (0.0, 0.0, 7.0),
                (Gnarling, _) => (0.0, 0.0, 7.0),
                (Mandragora, _) => (0.0, 0.0, 7.0),
                (Kappa, _) => (0.75, 1.0, 12.0),
                (Cactid, _) => (0.0, 0.0, 8.0),
                (Gnoll, _) => (1.0, 0.0, 9.0),
                (Haniwa, _) => (0.0, 0.5, 8.0),
                (Myrmidon, _) => (0.0, 0.0, 8.0),
                (Husk, _) => (0.0, 0.0, 8.0),
                (Boreal, _) => (1.0, 0.0, 5.0),
            },
            scaler: match (body.species, body.body_type) {
                (Gnome, _) => 0.8,
                (Sahagin, _) => 1.05,
                (Adlet, _) => 1.0,
                (Gnarling, _) => 0.8,
                (Mandragora, _) => 0.8,
                (Kappa, _) => 0.8,
                (Cactid, _) => 0.8,
                (Gnoll, _) => 0.8,
                (Haniwa, _) => 1.12,
                (Myrmidon, _) => 1.24,
                (Husk, _) => 1.12,
                (Boreal, _) => 1.0,
            },
        }
    }
}
