pub mod alpha;
pub mod beam;
pub mod beta;
pub mod block;
pub mod charge;
pub mod chargeswing;
pub mod climb;
pub mod dance;
pub mod dash;
pub mod equip;
pub mod glidewield;
pub mod gliding;
pub mod idle;
pub mod jump;
pub mod leapmelee;
pub mod repeater;
pub mod roll;
pub mod run;
pub mod shockwave;
pub mod shoot;
pub mod sit;
pub mod sneak;
pub mod spin;
pub mod spinmelee;
pub mod stand;
pub mod swim;
pub mod swimwield;
pub mod wield;

// Reexports
pub use self::{
    alpha::AlphaAnimation, beam::BeamAnimation, beta::BetaAnimation, block::BlockAnimation,
    charge::ChargeAnimation, chargeswing::ChargeswingAnimation, climb::ClimbAnimation,
    dance::DanceAnimation, dash::DashAnimation, equip::EquipAnimation,
    glidewield::GlideWieldAnimation, gliding::GlidingAnimation, idle::IdleAnimation,
    jump::JumpAnimation, leapmelee::LeapAnimation, repeater::RepeaterAnimation,
    roll::RollAnimation, run::RunAnimation, shockwave::ShockwaveAnimation, shoot::ShootAnimation,
    sit::SitAnimation, sneak::SneakAnimation, spin::SpinAnimation, spinmelee::SpinMeleeAnimation,
    stand::StandAnimation, swim::SwimAnimation, swimwield::SwimWieldAnimation,
    wield::WieldAnimation,
};
use super::{make_bone, vek::*, FigureBoneData, Skeleton};
use common::comp;
use core::convert::TryFrom;
use std::f32::consts::PI;

pub type Body = comp::humanoid::Body;

skeleton_impls!(struct CharacterSkeleton {
    + head,
    + chest,
    + belt,
    + back,
    + shorts,
    + hand_l,
    + hand_r,
    + foot_l,
    + foot_r,
    + shoulder_l,
    + shoulder_r,
    + glider,
    + main,
    + second,
    + lantern,
    + hold,
    torso,
    control,
    control_l,
    control_r,
});

impl Skeleton for CharacterSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 16;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"character_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_compute_mats")]

    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
    ) -> Vec3<f32> {
        let torso_mat = base_mat * Mat4::<f32>::from(self.torso);
        let chest_mat = torso_mat * Mat4::<f32>::from(self.chest);
        let head_mat = chest_mat * Mat4::<f32>::from(self.head);
        let shorts_mat = chest_mat * Mat4::<f32>::from(self.shorts);
        let control_mat = chest_mat * Mat4::<f32>::from(self.control);
        let control_l_mat = control_mat * Mat4::<f32>::from(self.control_l);
        let control_r_mat = control_mat * Mat4::<f32>::from(self.control_r);

        let hand_l_mat = Mat4::<f32>::from(self.hand_l);
        let lantern_mat = Mat4::<f32>::from(self.lantern);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_mat),
            make_bone(chest_mat),
            make_bone(chest_mat * Mat4::<f32>::from(self.belt)),
            make_bone(chest_mat * Mat4::<f32>::from(self.back)),
            make_bone(shorts_mat),
            make_bone(control_l_mat * hand_l_mat),
            make_bone(control_r_mat * Mat4::<f32>::from(self.hand_r)),
            make_bone(torso_mat * Mat4::<f32>::from(self.foot_l)),
            make_bone(torso_mat * Mat4::<f32>::from(self.foot_r)),
            make_bone(chest_mat * Mat4::<f32>::from(self.shoulder_l)),
            make_bone(chest_mat * Mat4::<f32>::from(self.shoulder_r)),
            make_bone(chest_mat * Mat4::<f32>::from(self.glider)),
            make_bone(control_l_mat * Mat4::<f32>::from(self.main)),
            make_bone(control_r_mat * Mat4::<f32>::from(self.second)),
            make_bone(shorts_mat * lantern_mat),
            // FIXME: Should this be control_l_mat?
            make_bone(control_mat * hand_l_mat * Mat4::<f32>::from(self.hold)),
        ];
        // NOTE: lantern_mat.cols.w = lantern_mat * Vec4::unit_w()
        (head_mat * lantern_mat.cols.w).xyz()
    }
}

pub struct SkeletonAttr {
    scaler: f32,
    head_scale: f32,
    head: (f32, f32),
    chest: (f32, f32),
    belt: (f32, f32),
    back: (f32, f32),
    shorts: (f32, f32),
    hand: (f32, f32, f32),
    foot: (f32, f32, f32),
    shoulder: (f32, f32, f32),
    lantern: (f32, f32, f32),
    shl: (f32, f32, f32, f32, f32, f32),
    shr: (f32, f32, f32, f32, f32, f32),
    sc: (f32, f32, f32, f32, f32, f32),
    hhl: (f32, f32, f32, f32, f32, f32),
    hhr: (f32, f32, f32, f32, f32, f32),
    hc: (f32, f32, f32, f32, f32, f32),
    sthl: (f32, f32, f32, f32, f32, f32),
    sthr: (f32, f32, f32, f32, f32, f32),
    stc: (f32, f32, f32, f32, f32, f32),
    ahl: (f32, f32, f32, f32, f32, f32),
    ahr: (f32, f32, f32, f32, f32, f32),
    ac: (f32, f32, f32, f32, f32, f32),
    bhl: (f32, f32, f32, f32, f32, f32),
    bhr: (f32, f32, f32, f32, f32, f32),
    bc: (f32, f32, f32, f32, f32, f32),
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            scaler: 0.0,
            head_scale: 0.0,
            head: (0.0, 0.0),
            chest: (0.0, 0.0),
            belt: (0.0, 0.0),
            back: (0.0, 0.0),
            shorts: (0.0, 0.0),
            hand: (0.0, 0.0, 0.0),
            foot: (0.0, 0.0, 0.0),
            shoulder: (0.0, 0.0, 0.0),
            lantern: (0.0, 0.0, 0.0),
            shl: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            shr: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            sc: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            hhl: (0.0, 0.0, 10.0, 0.0, 0.0, 0.0),
            hhr: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            hc: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            sthl: (0.0, 0.0, 10.0, 0.0, 0.0, 0.0),
            sthr: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            stc: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            ahl: (0.0, 0.0, 10.0, 0.0, 0.0, 0.0),
            ahr: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            ac: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            bhl: (0.0, 0.0, 10.0, 0.0, 0.0, 0.0),
            bhr: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            bc: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        }
    }
}

impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::Humanoid(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    #[allow(clippy::match_single_binding)] // TODO: Pending review in #587
    fn from(body: &'a Body) -> Self {
        use comp::humanoid::{BodyType::*, Species::*};
        Self {
            scaler: comp::Body::Humanoid(*body).scale(),
            head_scale: match (body.species, body.body_type) {
                (Orc, Male) => 0.9,
                (Orc, Female) => 0.9,
                (Human, Male) => 0.9,
                (Human, Female) => 0.9,
                (Elf, Male) => 0.9,
                (Elf, Female) => 0.9,
                (Dwarf, Male) => 1.0,
                (Dwarf, Female) => 1.0,
                (Undead, Male) => 0.9,
                (Undead, Female) => 0.9,
                (Danari, Male) => 1.15,
                (Danari, Female) => 1.15,
            },
            head: match (body.species, body.body_type) {
                (Orc, Male) => (-2.0, 13.5),
                (Orc, Female) => (-2.0, 13.0),
                (Human, Male) => (-2.3, 13.0),
                (Human, Female) => (-2.0, 13.0),
                (Elf, Male) => (-2.5, 13.0),
                (Elf, Female) => (-1.0, 13.0),
                (Dwarf, Male) => (-2.0, 14.0),
                (Dwarf, Female) => (-2.0, 13.5),
                (Undead, Male) => (-1.5, 13.0),
                (Undead, Female) => (-1.5, 14.0),
                (Danari, Male) => (-1.5, 12.5),
                (Danari, Female) => (-1.5, 13.5),
            },
            chest: match (body.species, body.body_type) {
                (_, _) => (0.0, 8.0),
            },
            belt: match (body.species, body.body_type) {
                (_, _) => (0.0, -2.0),
            },
            back: match (body.species, body.body_type) {
                (_, _) => (-3.1, 7.25),
            },
            shorts: match (body.species, body.body_type) {
                (_, _) => (0.0, -5.0),
            },
            hand: match (body.species, body.body_type) {
                (_, _) => (7.0, -0.25, 0.5),
            },
            foot: match (body.species, body.body_type) {
                (_, _) => (3.4, 0.5, 2.0),
            },
            shoulder: match (body.species, body.body_type) {
                (_, _) => (5.0, 0.0, 5.0),
            },
            lantern: match (body.species, body.body_type) {
                (_, _) => (5.0, 2.5, 5.5),
            },
            shl: match (body.species, body.body_type) {
                (_, _) => (-0.75, -1.0, 0.5, 1.47, -0.2, 0.0),
            },
            shr: match (body.species, body.body_type) {
                (_, _) => (0.75, -1.5, -2.5, 1.47, 0.3, 0.0),
            },
            sc: match (body.species, body.body_type) {
                (_, _) => (-7.0, 7.0, 2.0, -0.1, 0.0, 0.0),
            },
            hhl: match (body.species, body.body_type) {
                (_, _) => (-0.5, -1.0, 10.0, 4.71, 0.0, 0.0),
            },
            hhr: match (body.species, body.body_type) {
                (_, _) => (0.0, 0.0, 0.0, 4.71, 0.0, 0.0),
            },
            hc: match (body.species, body.body_type) {
                (_, _) => (6.0, 7.0, 1.0, -0.3, -1.57, 3.64),
            },
            sthl: match (body.species, body.body_type) {
                (_, _) => (0.0, 0.0, 1.0, 1.27, 0.0, 0.0),
            },
            sthr: match (body.species, body.body_type) {
                (_, _) => (0.0, 0.0, 7.0, 1.57, 0.2, 0.0),
            },
            stc: match (body.species, body.body_type) {
                (_, _) => (-5.0, 5.0, -1.0, -0.3, 0.15, 0.0),
            },
            ahl: match (body.species, body.body_type) {
                (_, _) => (-0.5, -1.0, 7.0, 1.17, PI, 0.0),
            },
            ahr: match (body.species, body.body_type) {
                (_, _) => (0.0, -1.0, 1.0, -2.0, 0.0, PI),
            },
            ac: match (body.species, body.body_type) {
                (_, _) => (-8.0, 11.0, 3.0, 2.0, 0.0, 0.0),
            },
            bhl: match (body.species, body.body_type) {
                (_, _) => (0.0, -4.0, 1.0, 1.57, 0.0, 0.0),
            },
            bhr: match (body.species, body.body_type) {
                (_, _) => (1.0, 2.0, -2.0, 1.57, 0.0, 0.0),
            },
            bc: match (body.species, body.body_type) {
                (_, _) => (-5.0, 9.0, 1.0, 0.0, 1.2, -0.6),
            },
        }
    }
}
