pub mod alpha;
pub mod beam;
pub mod beta;
pub mod blink;
pub mod charge;
pub mod chargemelee;
pub mod dash;
pub mod equip;
pub mod idle;
pub mod jump;
pub mod leapmelee;
pub mod run;
pub mod selfbuff;
pub mod shockwave;
pub mod shoot;
pub mod spin;
pub mod spinmelee;
pub mod stunned;
pub mod summon;
pub mod wield;

// Reexports
pub use self::{
    alpha::AlphaAnimation, beam::BeamAnimation, beta::BetaAnimation, blink::BlinkAnimation,
    charge::ChargeAnimation, chargemelee::ChargeMeleeAnimation, dash::DashAnimation,
    equip::EquipAnimation, idle::IdleAnimation, jump::JumpAnimation, leapmelee::LeapAnimation,
    run::RunAnimation, selfbuff::SelfBuffAnimation, shockwave::ShockwaveAnimation,
    shoot::ShootAnimation, spin::SpinAnimation, spinmelee::SpinMeleeAnimation,
    stunned::StunnedAnimation, summon::SummonAnimation, wield::WieldAnimation,
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
    control_l,
    control_r,
    weapon_l,
    weapon_r,
    leg_control_l,
    leg_control_r,
    arm_control_l,
    arm_control_r,
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
    ) -> [Vec3<f32>; 2] {
        let upper_torso = Mat4::<f32>::from(self.upper_torso);

        let torso_mat = base_mat * Mat4::<f32>::from(self.torso);
        let upper_torso_mat = torso_mat * upper_torso;
        let control_mat = Mat4::<f32>::from(self.control);
        let control_l_mat = Mat4::<f32>::from(self.control_l);
        let control_r_mat = Mat4::<f32>::from(self.control_r);
        let weapon_l_mat = control_mat * Mat4::<f32>::from(self.weapon_l);
        let weapon_r_mat = control_mat * Mat4::<f32>::from(self.weapon_r);
        let lower_torso_mat = upper_torso_mat * Mat4::<f32>::from(self.lower_torso);

        let leg_l = Mat4::<f32>::from(self.leg_l);
        let leg_r = Mat4::<f32>::from(self.leg_r);

        let leg_control_l = lower_torso_mat * Mat4::<f32>::from(self.leg_control_l);
        let leg_control_r = lower_torso_mat * Mat4::<f32>::from(self.leg_control_r);

        let arm_control_l = upper_torso_mat * Mat4::<f32>::from(self.arm_control_l);
        let arm_control_r = upper_torso_mat * Mat4::<f32>::from(self.arm_control_r);

        let head_mat = upper_torso_mat * Mat4::<f32>::from(self.head);
        let hand_l_mat = Mat4::<f32>::from(self.hand_l);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_mat),
            make_bone(head_mat * Mat4::<f32>::from(self.jaw)),
            make_bone(upper_torso_mat),
            make_bone(lower_torso_mat),
            make_bone(lower_torso_mat * Mat4::<f32>::from(self.tail)),
            make_bone(upper_torso_mat * weapon_l_mat * Mat4::<f32>::from(self.main)),
            make_bone(upper_torso_mat * weapon_r_mat * Mat4::<f32>::from(self.second)),
            make_bone(arm_control_l * Mat4::<f32>::from(self.shoulder_l)),
            make_bone(arm_control_r * Mat4::<f32>::from(self.shoulder_r)),
            make_bone(
                arm_control_l * weapon_l_mat * control_l_mat * Mat4::<f32>::from(self.hand_l),
            ),
            make_bone(
                arm_control_r * weapon_r_mat * control_r_mat * Mat4::<f32>::from(self.hand_r),
            ),
            make_bone(leg_control_l * leg_l),
            make_bone(leg_control_r * leg_r),
            make_bone(leg_control_l * Mat4::<f32>::from(self.foot_l)),
            make_bone(leg_control_r * Mat4::<f32>::from(self.foot_r)),
            // FIXME: Should this be control_l_mat?
            make_bone(upper_torso_mat * control_mat * hand_l_mat * Mat4::<f32>::from(self.hold)),
        ];
        [Vec3::default(), Vec3::default()]
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
    scaler: f32,
    tempo: f32,
    grip: (f32, f32),
    shl: (f32, f32, f32, f32, f32, f32),
    shr: (f32, f32, f32, f32, f32, f32),
    sc: (f32, f32, f32, f32, f32, f32),
    hhl: (f32, f32, f32, f32, f32, f32),
    hhr: (f32, f32, f32, f32, f32, f32),
    hc: (f32, f32, f32, f32, f32, f32),
    sthl: (f32, f32, f32, f32, f32, f32),
    sthr: (f32, f32, f32, f32, f32, f32),
    stc: (f32, f32, f32, f32, f32, f32),
    bhl: (f32, f32, f32, f32, f32, f32),
    bhr: (f32, f32, f32, f32, f32, f32),
    bc: (f32, f32, f32, f32, f32, f32),
    beast: bool,
    float: bool,
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
            scaler: 0.0,
            tempo: 0.0,
            grip: (0.0, 0.0),
            shl: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            shr: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            sc: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            hhl: (0.0, 0.0, 10.0, 0.0, 0.0, 0.0),
            hhr: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            hc: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            sthl: (0.0, 0.0, 10.0, 0.0, 0.0, 0.0),
            sthr: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            stc: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            bhl: (0.0, 0.0, 10.0, 0.0, 0.0, 0.0),
            bhr: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            bc: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            beast: false,
            float: false,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::biped_large::{BodyType::*, Species::*};
        Self {
            head: match (body.species, body.body_type) {
                (Ogre, Male) => (5.0, 6.0),
                (Ogre, Female) => (1.0, 7.5),
                (Cyclops, _) => (4.5, 7.5),
                (Wendigo, _) => (3.0, 7.5),
                (Troll, _) => (6.0, 10.0),
                (Dullahan, _) => (3.0, 6.0),
                (Werewolf, _) => (11.5, 1.0),
                (Occultsaurok, _) => (6.0, 3.5),
                (Mightysaurok, _) => (6.0, 3.5),
                (Slysaurok, _) => (6.0, 3.5),
                (Mindflayer, _) => (5.0, 5.5),
                (Minotaur, _) => (6.0, 3.0),
                (Tidalwarrior, _) => (6.5, 5.0),
                (Yeti, _) => (8.5, 4.0),
                (Harvester, _) => (6.0, 11.0),
                (Blueoni, _) => (10.5, -3.0),
                (Redoni, _) => (10.5, -3.0),
            },
            jaw: match (body.species, body.body_type) {
                (Ogre, _) => (0.0, 0.0),
                (Cyclops, _) => (0.0, 0.0),
                (Wendigo, _) => (0.0, 0.0),
                (Troll, _) => (2.0, -4.0),
                (Dullahan, _) => (0.0, 0.0),
                (Werewolf, _) => (5.0, -4.5),
                (Occultsaurok, _) => (1.0, -2.5),
                (Mightysaurok, _) => (1.0, -2.5),
                (Slysaurok, _) => (1.0, -2.5),
                (Mindflayer, _) => (0.0, 0.0),
                (Minotaur, _) => (2.0, -4.0),
                (Tidalwarrior, _) => (0.0, 0.0),
                (Yeti, _) => (-5.0, -5.0),
                (Harvester, _) => (-2.0, -7.0),
                (Blueoni, _) => (0.0, 3.5),
                (Redoni, _) => (0.0, 3.5),
            },
            upper_torso: match (body.species, body.body_type) {
                (Ogre, Male) => (0.0, 27.5),
                (Ogre, Female) => (0.0, 28.0),
                (Cyclops, _) => (-2.0, 27.0),
                (Wendigo, _) => (-1.0, 29.0),
                (Troll, _) => (-1.0, 26.5),
                (Dullahan, _) => (0.0, 29.0),
                (Werewolf, _) => (3.0, 26.0),
                (Occultsaurok, _) => (3.0, 24.0),
                (Mightysaurok, _) => (3.0, 24.0),
                (Slysaurok, _) => (3.0, 24.0),
                (Mindflayer, _) => (0.0, 30.5),
                (Minotaur, _) => (-1.0, 31.5),
                (Tidalwarrior, _) => (-1.0, 25.0),
                (Yeti, _) => (-1.0, 23.5),
                (Harvester, _) => (-1.0, 18.0),
                (Blueoni, _) => (-1.0, 26.5),
                (Redoni, _) => (-1.0, 26.5),
            },
            lower_torso: match (body.species, body.body_type) {
                (Ogre, Male) => (1.0, -7.0),
                (Ogre, Female) => (0.0, -6.0),
                (Cyclops, _) => (1.0, -4.5),
                (Wendigo, _) => (-1.5, -6.0),
                (Troll, _) => (1.0, -10.5),
                (Dullahan, _) => (0.0, -6.5),
                (Werewolf, _) => (1.0, -10.0),
                (Occultsaurok, _) => (0.0, -5.0),
                (Mightysaurok, _) => (0.0, -5.0),
                (Slysaurok, _) => (0.0, -6.0),
                (Mindflayer, _) => (3.5, -10.0),
                (Minotaur, _) => (1.5, -8.5),
                (Tidalwarrior, _) => (0.0, -9.5),
                (Yeti, _) => (0.0, -6.5),
                (Harvester, _) => (-1.0, -4.5),
                (Blueoni, _) => (0.0, -8.5),
                (Redoni, _) => (0.0, -8.5),
            },
            tail: match (body.species, body.body_type) {
                (Werewolf, _) => (-5.5, -2.0),
                (Occultsaurok, _) => (-4.5, -6.0),
                (Mightysaurok, _) => (-4.5, -6.0),
                (Slysaurok, _) => (-4.5, -6.0),
                (Minotaur, _) => (-3.0, -6.0),
                _ => (0.0, 0.0),
            },
            shoulder: match (body.species, body.body_type) {
                (Ogre, Male) => (12.0, 0.5, 3.0),
                (Ogre, Female) => (8.0, 0.5, 2.0),
                (Cyclops, _) => (9.5, 2.5, 4.5),
                (Wendigo, _) => (9.0, 0.5, 2.5),
                (Troll, _) => (11.0, 0.5, 4.5),
                (Dullahan, _) => (14.0, 0.5, 3.5),
                (Werewolf, _) => (9.0, 4.0, -3.0),
                (Occultsaurok, _) => (7.5, 1.0, 1.5),
                (Mightysaurok, _) => (7.5, 1.0, 1.5),
                (Slysaurok, _) => (7.5, 1.0, 1.5),
                (Mindflayer, _) => (8.0, 0.5, -1.0),
                (Minotaur, _) => (10.0, 1.0, -1.0),
                (Tidalwarrior, _) => (14.0, -0.5, 2.0),
                (Yeti, _) => (10.5, 1.0, -2.5),
                (Harvester, _) => (8.0, 1.0, -1.5),
                (Blueoni, _) => (11.0, 2.0, -5.5),
                (Redoni, _) => (11.0, 2.0, -5.5),
            },
            hand: match (body.species, body.body_type) {
                (Ogre, Male) => (14.5, 0.0, -4.0),
                (Ogre, Female) => (9.0, 0.5, -4.5),
                (Cyclops, _) => (10.0, 2.0, -0.5),
                (Wendigo, _) => (12.0, 0.0, -3.5),
                (Troll, _) => (11.5, 0.0, -5.5),
                (Dullahan, _) => (14.5, 0.0, -2.5),
                (Werewolf, _) => (10.0, 2.5, -11.0),
                (Occultsaurok, _) => (8.0, 1.5, -5.5),
                (Mightysaurok, _) => (8.0, 1.5, -5.5),
                (Slysaurok, _) => (8.0, 1.5, -5.5),
                (Mindflayer, _) => (9.0, 0.5, -4.5),
                (Minotaur, _) => (12.5, 0.5, -7.0),
                (Tidalwarrior, _) => (15.5, -0.5, -3.0),
                (Yeti, _) => (12.0, 1.5, -6.0),
                (Harvester, _) => (11.5, 1.5, -5.5),
                (Blueoni, _) => (13.5, 0.5, -8.0),
                (Redoni, _) => (13.5, 0.5, -8.0),
            },
            leg: match (body.species, body.body_type) {
                (Ogre, Male) => (0.0, 0.0, -4.0),
                (Ogre, Female) => (0.0, 0.0, -2.0),
                (Cyclops, _) => (0.0, -1.0, -5.0),
                (Wendigo, _) => (2.0, 2.0, -2.5),
                (Troll, _) => (5.0, 0.0, -6.0),
                (Dullahan, _) => (0.0, 0.0, -5.0),
                (Werewolf, _) => (4.5, 1.0, -5.0),
                (Occultsaurok, _) => (3.0, 0.5, -4.0),
                (Mightysaurok, _) => (3.0, 0.5, -4.0),
                (Slysaurok, _) => (3.0, 0.5, -4.0),
                (Mindflayer, _) => (6.0, -2.0, 6.5),
                (Minotaur, _) => (5.0, 0.0, -10.0),
                (Tidalwarrior, _) => (2.5, 0.0, -5.5),
                (Yeti, _) => (4.0, 0.0, -5.5),
                (Harvester, _) => (3.5, 1.0, -4.0),
                (Blueoni, _) => (4.5, 2.0, -5.5),
                (Redoni, _) => (4.5, 2.0, -5.5),
            },
            foot: match (body.species, body.body_type) {
                (Ogre, Male) => (4.0, 1.0, -12.0),
                (Ogre, Female) => (4.0, 0.5, -13.5),
                (Cyclops, _) => (4.0, 1.5, -17.0),
                (Wendigo, _) => (5.0, 2.5, -17.0),
                (Troll, _) => (6.0, 1.5, -13.0),
                (Dullahan, _) => (4.0, 2.5, -14.0),
                (Werewolf, _) => (5.5, 3.0, -6.5),
                (Occultsaurok, _) => (3.5, 3.5, -10.0),
                (Mightysaurok, _) => (3.5, 3.5, -10.0),
                (Slysaurok, _) => (3.5, 3.5, -10.0),
                (Mindflayer, _) => (4.5, 1.5, -16.0),
                (Minotaur, _) => (6.0, 4.5, -17.5),
                (Tidalwarrior, _) => (3.5, 0.5, -10.5),
                (Yeti, _) => (4.5, 0.5, -12.5),
                (Harvester, _) => (4.5, 0.5, -9.5),
                (Blueoni, _) => (5.0, 5.0, -12.5),
                (Redoni, _) => (5.0, 5.0, -12.5),
            },
            scaler: match (body.species, body.body_type) {
                (Ogre, Male) => 1.12,
                (Ogre, Female) => 1.12,
                (Cyclops, _) => 1.28,
                (Wendigo, _) => 1.1,
                (Troll, _) => 1.1,
                (Dullahan, _) => 1.12,
                (Werewolf, _) => 1.0,
                (Occultsaurok, _) => 1.0,
                (Mightysaurok, _) => 1.0,
                (Slysaurok, _) => 1.0,
                (Mindflayer, _) => 1.5,
                (Minotaur, _) => 1.7,
                (Tidalwarrior, _) => 1.7,
                (Yeti, _) => 1.2,
                (Harvester, _) => 1.2,
                (Blueoni, _) => 1.2,
                (Redoni, _) => 1.2,
            },
            tempo: match (body.species, body.body_type) {
                (Ogre, Male) => 0.9,
                (Ogre, Female) => 0.9,
                (Cyclops, _) => 0.8,
                (Troll, _) => 0.9,
                (Dullahan, _) => 0.8,
                (Minotaur, _) => 0.8,
                _ => 1.0,
            },
            grip: match (body.species, body.body_type) {
                (Ogre, Male) => (13.0, 0.0),
                (Ogre, Female) => (8.0, 0.0),
                (Cyclops, _) => (12.0, 0.0),
                (Wendigo, _) => (15.0, 0.0),
                (Troll, _) => (12.0, 0.0),
                (Dullahan, _) => (15.0, 0.0),
                (Werewolf, _) => (13.0, 0.0),
                (Occultsaurok, _) => (10.0, 0.0),
                (Mightysaurok, _) => (10.0, 0.0),
                (Slysaurok, _) => (10.0, 0.0),
                (Mindflayer, _) => (12.0, 2.5),
                (Minotaur, _) => (14.0, 0.0),
                (Tidalwarrior, _) => (14.0, 0.0),
                (Yeti, _) => (12.5, 0.0),
                (Harvester, _) => (7.5, 0.0),
                (Blueoni, _) => (12.5, 0.0),
                (Redoni, _) => (12.5, 0.0),
            },
            shl: match (body.species, body.body_type) {
                (Dullahan, _) => (-4.75, -11.0, 8.5, 1.47, -0.2, 0.0),
                (Mightysaurok, _) => (-1.75, -9.0, 3.5, 1.47, -0.2, 0.0),
                _ => (-4.75, -1.0, 2.5, 1.47, -0.2, 0.0),
            },
            shr: match (body.species, body.body_type) {
                (Dullahan, _) => (5.75, -11.5, 4.5, 1.47, 0.3, 0.0),
                (Mightysaurok, _) => (2.75, -9.5, -0.5, 1.47, 0.3, 0.0),
                _ => (3.75, -1.5, -0.5, 1.47, 0.3, 0.0),
            },
            sc: match (body.species, body.body_type) {
                (Dullahan, _) => (-7.0, 17.0, -16.0, -0.1, 0.0, 0.0),
                (Mightysaurok, _) => (-7.0, 15.0, -11.0, -0.1, 0.0, 0.0),
                _ => (-7.0, 7.0, -10.0, -0.1, 0.0, 0.0),
            },
            hhl: match (body.species, body.body_type) {
                (Ogre, Male) => (-9.0, -10.0, 23.0, 1.57, -0.57, 0.0),
                (Cyclops, _) => (-6.0, -10.0, 17.0, 1.57, -0.57, 0.0),
                (Troll, _) => (-6.0, -10.0, 17.0, 1.57, -0.57, 0.0),
                (Yeti, _) => (-6.0, -10.0, 17.0, 1.57, -0.57, 0.0),
                (Blueoni, _) => (-6.0, -10.0, 17.0, 1.57, -0.57, 0.0),
                (Redoni, _) => (-6.0, -10.0, 17.0, 1.57, -0.57, 0.0),
                _ => (-6.0, -10.0, 17.0, 1.57, -0.57, 0.0),
            },
            hhr: match (body.species, body.body_type) {
                (Ogre, Male) => (-5.0, -13.0, 0.0, 1.57, -0.57, 0.0),
                (Cyclops, _) => (-6.0, -10.0, 0.0, 1.57, -0.57, 0.0),
                (Troll, _) => (-6.0, -10.0, 0.0, 1.57, -0.57, 0.0),
                (Yeti, _) => (-6.0, -10.0, 0.0, 1.57, -0.57, 0.0),
                (Blueoni, _) => (-6.0, -10.0, 0.0, 1.57, -0.57, 0.0),
                (Redoni, _) => (-6.0, -10.0, 0.0, 1.57, -0.57, 0.0),
                _ => (-6.0, -10.0, 0.0, 1.57, -0.57, 0.0),
            },
            hc: match (body.species, body.body_type) {
                (Ogre, Male) => (11.5, 9.0, -13.0, -0.57, -1.57, 1.0),
                (Cyclops, _) => (8.5, 6.0, -12.0, -0.57, -1.57, 1.0),
                (Troll, _) => (8.5, 6.0, -12.0, -0.57, -1.57, 1.0),
                (Yeti, _) => (8.5, 6.0, -12.0, -0.57, -1.57, 1.0),
                (Blueoni, _) => (8.5, 6.0, -12.0, -0.57, -1.57, 1.0),
                (Redoni, _) => (8.5, 6.0, -12.0, -0.57, -1.57, 1.0),
                _ => (8.5, 6.0, -12.0, -0.57, -1.57, 1.0),
            },
            sthl: match (body.species, body.body_type) {
                (Ogre, Female) => (-1.0, -5.0, 12.0, 1.27, 0.0, 0.0),
                (Occultsaurok, _) => (-1.0, -7.0, 12.0, 1.27, 0.0, 0.0),
                (Mindflayer, _) => (1.0, -10.5, 7.0, 1.27, 0.0, 0.0),
                _ => (11.0, 5.0, -4.0, 1.27, 0.0, 0.0),
            },
            sthr: match (body.species, body.body_type) {
                (Ogre, Female) => (5.0, -3.5, 18.0, 1.57, 0.8, 0.0),
                (Occultsaurok, _) => (7.0, -3.5, 18.0, 1.57, 0.8, 0.0),
                (Mindflayer, _) => (7.0, -9.0, 13.0, 1.57, 0.8, 0.0),
                _ => (17.0, 7.5, 2.0, 1.57, 0.8, 0.0),
            },
            stc: match (body.species, body.body_type) {
                (Ogre, Female) => (-10.0, 7.0, -23.0, -0.3, 0.15, 0.0),
                (Occultsaurok, _) => (-10.0, 7.0, -22.0, -0.3, 0.15, 0.0),
                (Mindflayer, _) => (-10.0, 12.5, -22.0, -0.3, 0.15, 0.0),
                _ => (-18.0, 1.0, -2.0, -0.3, 0.15, 0.0),
            },
            bhl: match (body.species, body.body_type) {
                (Slysaurok, _) => (-1.0, -12.0, 1.0, 1.57, 0.0, 0.0),
                _ => (3.0, 2.5, 0.0, 1.2, -0.6, -0.3),
            },
            bhr: match (body.species, body.body_type) {
                (Slysaurok, _) => (0.0, -6.0, -2.0, 1.57, 0.0, 0.0),
                _ => (5.9, 5.5, -5.0, 1.2, -0.6, -0.3),
            },
            bc: match (body.species, body.body_type) {
                (Slysaurok, _) => (1.0, 13.0, -8.0, 0.0, 1.2, -0.6),
                _ => (-7.0, 3.0, -8.0, 0.0, 0.0, 0.0),
            },
            beast: matches!((body.species, body.body_type), (Werewolf, _)),
            float: matches!((body.species, body.body_type), (Mindflayer, _)),
        }
    }
}
