pub mod alpha;
pub mod beta;
pub mod idle;
pub mod jump;
pub mod run;
pub mod wield;
pub mod shoot;
pub mod charge;
pub mod dash;
pub mod spin;
pub mod spinmelee;

// Reexports
pub use self::{
    alpha::AlphaAnimation, beta::BetaAnimation, idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation,
    wield::WieldAnimation, charge::ChargeAnimation, shoot::ShootAnimation, dash::DashAnimation, spin::SpinAnimation,
    spinmelee::SpinMeleeAnimation,
};

use super::{make_bone, vek::*, FigureBoneData, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::biped_large::Body;

skeleton_impls!(struct BipedLargeSkeleton {
    + head,
    + jaw,
    + upper_torso,
    + lower_torso,
    + tail,
    + main,
    + second,
    + shoulder_l,
    + shoulder_r,
    + hand_l,
    + hand_r,
    + leg_l,
    + leg_r,
    + foot_l,
    + foot_r,
    + hold,
    torso,
    control,
});

impl Skeleton for BipedLargeSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 16;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"biped_large_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_compute_mats")]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
    ) -> Vec3<f32> {
        let upper_torso = Mat4::<f32>::from(self.upper_torso);

        let torso_mat = base_mat * Mat4::<f32>::from(self.torso);
        let upper_torso_mat = torso_mat * upper_torso;
        let lower_torso_mat = upper_torso_mat * Mat4::<f32>::from(self.lower_torso);
        let head_mat = upper_torso_mat * Mat4::<f32>::from(self.head);
        let control_mat = upper_torso_mat * Mat4::<f32>::from(self.control);
        let hand_l_mat = Mat4::<f32>::from(self.hand_l);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_mat),
            make_bone(head_mat * Mat4::<f32>::from(self.jaw)),
            make_bone(upper_torso_mat),
            make_bone(lower_torso_mat),
            make_bone(lower_torso_mat * Mat4::<f32>::from(self.tail)),
            make_bone(control_mat * Mat4::<f32>::from(self.main)),
            make_bone(control_mat * Mat4::<f32>::from(self.second)),
            make_bone(upper_torso_mat * Mat4::<f32>::from(self.shoulder_l)),
            make_bone(upper_torso_mat * Mat4::<f32>::from(self.shoulder_r)),
            make_bone(control_mat * Mat4::<f32>::from(self.hand_l)),
            make_bone(control_mat * Mat4::<f32>::from(self.hand_r)),
            make_bone(lower_torso_mat * Mat4::<f32>::from(self.leg_l)),
            make_bone(lower_torso_mat * Mat4::<f32>::from(self.leg_r)),
            make_bone(base_mat * Mat4::<f32>::from(self.foot_l)),
            make_bone(base_mat * Mat4::<f32>::from(self.foot_r)),
            // FIXME: Should this be l_control_mat?
            make_bone(control_mat * hand_l_mat * Mat4::<f32>::from(self.hold)),
        ];
        Vec3::default()
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    jaw: (f32, f32),
    upper_torso: (f32, f32),
    lower_torso: (f32, f32),
    tail: (f32, f32),
    shoulder: (f32, f32, f32),
    hand: (f32, f32, f32),
    leg: (f32, f32, f32),
    foot: (f32, f32, f32),
    beast: bool,
    beast_ori: f32,
    beast_head: (f32, f32),
    beast_foot: (f32, f32, f32),
    beast_hand: (f32, f32, f32),
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::BipedLarge(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            jaw: (0.0, 0.0),
            upper_torso: (0.0, 0.0),
            lower_torso: (0.0, 0.0),
            tail: (0.0, 0.0),
            shoulder: (0.0, 0.0, 0.0),
            hand: (0.0, 0.0, 0.0),
            leg: (0.0, 0.0, 0.0),
            foot: (0.0, 0.0, 0.0),
            beast: false,
            beast_ori: 0.0,
            beast_head: (0.0, 0.0),
            beast_foot: (0.0, 0.0, 0.0),
            beast_hand: (0.0, 0.0, 0.0),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::biped_large::{BodyType::*, Species::*};
        Self {
            head: match (body.species, body.body_type) {
                (Ogre, Male) => (3.0, 9.0),
                (Ogre, Female) => (1.0, 7.5),
                (Cyclops, _) => (4.5, 7.5),
                (Wendigo, _) => (3.0, 13.5),
                (Troll, _) => (6.0, 10.0),
                (Dullahan, _) => (3.0, 6.0),
                (Werewolf, _) => (19.0, 1.0),
                (Occultlizardman, _) => (6.0, 3.5),
                (Mightylizardman, _) => (6.0, 3.5),
                (Slylizardman, _) => (6.0, 3.5),
            },
            jaw: match (body.species, body.body_type) {
                (Ogre, _) => (0.0, 0.0),
                (Cyclops, _) => (0.0, 0.0),
                (Wendigo, _) => (0.0, 0.0),
                (Troll, _) => (2.0, -4.0),
                (Dullahan, _) => (0.0, 0.0),
                (Werewolf, _) => (-2.5, -4.5),
                (Occultlizardman, _) => (1.0, -2.5),
                (Mightylizardman, _) => (1.0, -2.5),
                (Slylizardman, _) => (1.0, -2.5),
            },
            upper_torso: match (body.species, body.body_type) {
                (Ogre, Male) => (0.0, 28.0),
                (Ogre, Female) => (0.0, 28.0),
                (Cyclops, _) => (-2.0, 27.0),
                (Wendigo, _) => (-1.0, 29.0),
                (Troll, _) => (-1.0, 27.5),
                (Dullahan, _) => (0.0, 29.0),
                (Werewolf, _) => (3.0, 26.5),
                (Occultlizardman, _) => (3.0, 23.0),
                (Mightylizardman, _) => (3.0, 23.0),
                (Slylizardman, _) => (3.0, 23.0),
            },
            lower_torso: match (body.species, body.body_type) {
                (Ogre, Male) => (1.0, -7.0),
                (Ogre, Female) => (0.0, -6.0),
                (Cyclops, _) => (1.0, -4.5),
                (Wendigo, _) => (-1.5, -6.0),
                (Troll, _) => (1.0, -10.5),
                (Dullahan, _) => (0.0, -6.5),
                (Werewolf, _) => (1.0, -10.0),
                (Occultlizardman, _) => (0.0, -6.0),
                (Mightylizardman, _) => (0.0, -6.0),
                (Slylizardman, _) => (0.0, -6.0),
            },
            tail: match (body.species, body.body_type) {
                (Ogre, _) => (0.0, 0.0),
                (Cyclops, _) => (0.0, 0.0),
                (Wendigo, _) => (0.0, 0.0),
                (Troll, _) => (0.0, 0.0),
                (Dullahan, _) => (0.0, 0.0),
                (Werewolf, _) => (-5.5, -2.0),
                (Occultlizardman, _) => (-4.5, -6.0),
                (Mightylizardman, _) => (-4.5, -6.0),
                (Slylizardman, _) => (-4.5, -6.0),
            },
            shoulder: match (body.species, body.body_type) {
                (Ogre, Male) => (12.0, 0.5, 0.0),
                (Ogre, Female) => (8.0, 0.5, -1.0),
                (Cyclops, _) => (9.5, 2.5, 2.5),
                (Wendigo, _) => (9.0, 0.5, -0.5),
                (Troll, _) => (11.0, 0.5, -1.5),
                (Dullahan, _) => (14.0, 0.5, 4.5),
                (Werewolf, _) => (9.0, 4.0, -6.5),
                (Occultlizardman, _) => (7.5, 1.0, -1.5),
                (Mightylizardman, _) => (7.5, 1.0, -1.5),
                (Slylizardman, _) => (7.5, 1.0, -1.5),
            },
            hand: match (body.species, body.body_type) {
                (Ogre, Male) => (14.5, 0.0, -2.0),
                (Ogre, Female) => (9.0, 0.5, -4.5),
                (Cyclops, _) => (10.0, 2.0, -0.5),
                (Wendigo, _) => (12.0, 0.0, -0.5),
                (Troll, _) => (11.5, 0.0, -1.5),
                (Dullahan, _) => (14.5, 0.0, -2.5),
                (Werewolf, _) => (10.0, 2.5, -11.0),
                (Occultlizardman, _) => (8.0, 1.5, -5.5),
                (Mightylizardman, _) => (8.0, 1.5, -5.5),
                (Slylizardman, _) => (8.0, 1.5, -5.5),
            },
            leg: match (body.species, body.body_type) {
                (Ogre, Male) => (0.0, 0.0, -4.0),
                (Ogre, Female) => (0.0, 0.0, -2.0),
                (Cyclops, _) => (0.0, 0.0, -5.0),
                (Wendigo, _) => (2.0, 2.0, -2.5),
                (Troll, _) => (5.0, 0.0, -6.0),
                (Dullahan, _) => (0.0, 0.0, -5.0),
                (Werewolf, _) => (4.5, 0.5, -3.0),
                (Occultlizardman, _) => (3.0, 0.5, -6.0),
                (Mightylizardman, _) => (3.0, 0.5, -6.0),
                (Slylizardman, _) => (3.0, 0.5, -6.0),
            },
            foot: match (body.species, body.body_type) {
                (Ogre, Male) => (4.0, 2.5, 8.0),
                (Ogre, Female) => (4.0, 0.5, 8.0),
                (Cyclops, _) => (4.0, 0.5, 5.0),
                (Wendigo, _) => (5.0, 0.5, 6.0),
                (Troll, _) => (6.0, 0.5, 4.0),
                (Dullahan, _) => (4.0, 2.5, 8.0),
                (Werewolf, _) => (5.5, 6.5, 6.0),
                (Occultlizardman, _) => (3.5, 4.0, 5.0),
                (Mightylizardman, _) => (3.5, 4.0, 5.0),
                (Slylizardman, _) => (3.5, 4.0, 5.0),
            },
            beast: match (body.species, body.body_type) {
                (Werewolf, _) => (true),
                _ => (false),
            },
            beast_ori: match (body.species, body.body_type) {
                (Werewolf, _) => (0.45),
                _ => (0.0),
            },
            beast_head: match (body.species, body.body_type) {
                (Werewolf, _) => (-3.0, 4.0),
                _ => (0.0, 0.0),
            },
            beast_foot: match (body.species, body.body_type) {
                (Werewolf, _) => (0.0, -11.0, 0.0),
                _ => (0.0, 0.0, 0.0),
            },
            beast_hand: match (body.species, body.body_type) {
                (Werewolf, _) => (0.0, 2.0, 1.0),
                _ => (0.0, 0.0, 0.0),
            },
        }
    }
}
