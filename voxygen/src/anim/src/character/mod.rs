pub mod alpha;
pub mod beta;
pub mod block;
pub mod blockidle;
pub mod charge;
pub mod climb;
pub mod dance;
pub mod dash;
pub mod equip;
pub mod glidewield;
pub mod gliding;
pub mod idle;
pub mod jump;
pub mod leapmelee;
pub mod roll;
pub mod run;
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
    alpha::AlphaAnimation, beta::BetaAnimation, block::BlockAnimation,
    blockidle::BlockIdleAnimation, charge::ChargeAnimation, climb::ClimbAnimation,
    dance::DanceAnimation, dash::DashAnimation, equip::EquipAnimation,
    glidewield::GlideWieldAnimation, gliding::GlidingAnimation, idle::IdleAnimation,
    jump::JumpAnimation, leapmelee::LeapAnimation, roll::RollAnimation, run::RunAnimation,
    shoot::ShootAnimation, sit::SitAnimation, sneak::SneakAnimation, spin::SpinAnimation,
    spinmelee::SpinMeleeAnimation, stand::StandAnimation, swim::SwimAnimation,
    swimwield::SwimWieldAnimation, wield::WieldAnimation,
};

use super::{make_bone, vek::*, FigureBoneData, Skeleton};
use common::comp;
use core::convert::TryFrom;

pub type Body = comp::humanoid::Body;

skeleton_impls!(struct CharacterSkeleton {
    + head,
    + chest,
    + belt,
    + back,
    + shorts,
    + l_hand,
    + r_hand,
    + l_foot,
    + r_foot,
    + l_shoulder,
    + r_shoulder,
    + glider,
    + main,
    + second,
    + lantern,
    + hold,
    torso,
    control,
    l_control,
    r_control,
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
        let l_control_mat = control_mat * Mat4::<f32>::from(self.l_control);
        let r_control_mat = control_mat * Mat4::<f32>::from(self.r_control);

        let l_hand_mat = Mat4::<f32>::from(self.l_hand);
        let lantern_mat = Mat4::<f32>::from(self.lantern);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_mat),
            make_bone(chest_mat),
            make_bone(chest_mat * Mat4::<f32>::from(self.belt)),
            make_bone(chest_mat * Mat4::<f32>::from(self.back)),
            make_bone(shorts_mat),
            make_bone(l_control_mat * l_hand_mat),
            make_bone(r_control_mat * Mat4::<f32>::from(self.r_hand)),
            make_bone(torso_mat * Mat4::<f32>::from(self.l_foot)),
            make_bone(torso_mat * Mat4::<f32>::from(self.r_foot)),
            make_bone(chest_mat * Mat4::<f32>::from(self.l_shoulder)),
            make_bone(chest_mat * Mat4::<f32>::from(self.r_shoulder)),
            make_bone(chest_mat * Mat4::<f32>::from(self.glider)),
            make_bone(l_control_mat * Mat4::<f32>::from(self.main)),
            make_bone(r_control_mat * Mat4::<f32>::from(self.second)),
            make_bone(shorts_mat * lantern_mat),
            // FIXME: Should this be l_control_mat?
            make_bone(control_mat * l_hand_mat * Mat4::<f32>::from(self.hold)),
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
            scaler: body.scale(),
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
                (Orc, Male) => (0.0, 13.5),
                (Orc, Female) => (0.0, 13.0),
                (Human, Male) => (0.3, 13.0),
                (Human, Female) => (0.0, 13.0),
                (Elf, Male) => (0.5, 13.0),
                (Elf, Female) => (1.0, 13.0),
                (Dwarf, Male) => (0.0, 14.0),
                (Dwarf, Female) => (0.0, 13.5),
                (Undead, Male) => (0.5, 13.0),
                (Undead, Female) => (0.5, 14.0),
                (Danari, Male) => (0.5, 12.5),
                (Danari, Female) => (0.5, 13.5),
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
        }
    }
}
