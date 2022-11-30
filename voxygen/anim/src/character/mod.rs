pub mod alpha;
pub mod beam;
pub mod beta;
pub mod block;
pub mod chargeswing;
pub mod climb;
pub mod collect;
pub mod combomelee;
pub mod consume;
pub mod dance;
pub mod dash;
pub mod divemelee;
pub mod equip;
pub mod finishermelee;
pub mod glidewield;
pub mod gliding;
pub mod idle;
pub mod jump;
pub mod leapmelee;
pub mod mount;
pub mod music;
pub mod rapidmelee;
pub mod repeater;
pub mod ripostemelee;
pub mod roll;
pub mod run;
pub mod selfbuff;
pub mod shockwave;
pub mod shoot;
pub mod sit;
pub mod sneak;
pub mod sneakequip;
pub mod sneakwield;
pub mod spin;
pub mod spinmelee;
pub mod staggered;
pub mod stand;
pub mod stunned;
pub mod swim;
pub mod swimwield;
pub mod talk;
pub mod wallrun;
pub mod wield;

// Reexports
pub use self::{
    alpha::AlphaAnimation, beam::BeamAnimation, beta::BetaAnimation, block::BlockAnimation,
    chargeswing::ChargeswingAnimation, climb::ClimbAnimation, collect::CollectAnimation,
    combomelee::ComboAnimation, consume::ConsumeAnimation, dance::DanceAnimation,
    dash::DashAnimation, divemelee::DiveMeleeAnimation, equip::EquipAnimation,
    finishermelee::FinisherMeleeAnimation, glidewield::GlideWieldAnimation,
    gliding::GlidingAnimation, idle::IdleAnimation, jump::JumpAnimation, leapmelee::LeapAnimation,
    mount::MountAnimation, music::MusicAnimation, rapidmelee::RapidMeleeAnimation,
    repeater::RepeaterAnimation, ripostemelee::RiposteMeleeAnimation, roll::RollAnimation,
    run::RunAnimation, selfbuff::SelfBuffAnimation, shockwave::ShockwaveAnimation,
    shoot::ShootAnimation, sit::SitAnimation, sneak::SneakAnimation,
    sneakequip::SneakEquipAnimation, sneakwield::SneakWieldAnimation, spin::SpinAnimation,
    spinmelee::SpinMeleeAnimation, staggered::StaggeredAnimation, stand::StandAnimation,
    stunned::StunnedAnimation, swim::SwimAnimation, swimwield::SwimWieldAnimation,
    talk::TalkAnimation, wallrun::WallrunAnimation, wield::WieldAnimation,
};
use super::{make_bone, vek::*, FigureBoneData, Offsets, Skeleton, TrailSource};
use common::comp;
use core::{convert::TryFrom, f32::consts::PI};

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
    :: // Begin non-bone fields
    holding_lantern: bool,
    main_weapon_trail: bool,
    off_weapon_trail: bool,
    // Cannot exist at same time as weapon trails. Since gliding and attacking are mutually exclusive, should never be a concern.
    glider_trails: bool,
});

impl CharacterSkeleton {
    pub fn new(holding_lantern: bool) -> Self {
        Self {
            holding_lantern,
            ..Self::default()
        }
    }
}

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
        body: Self::Body,
    ) -> Offsets {
        // TODO: extract scaler from body to it's own method so we can call that
        // directly instead of going through SkeletonAttr? (note todo also
        // appiles to other body variant animations)
        let base_mat = base_mat * Mat4::scaling_3d(SkeletonAttr::from(&body).scaler / 11.0);

        let torso_mat = base_mat * Mat4::<f32>::from(self.torso);
        let chest_mat = torso_mat * Mat4::<f32>::from(self.chest);
        let head_mat = chest_mat * Mat4::<f32>::from(self.head);
        let shorts_mat = chest_mat * Mat4::<f32>::from(self.shorts);
        let control_mat = chest_mat * Mat4::<f32>::from(self.control);
        let control_l_mat = control_mat * Mat4::<f32>::from(self.control_l);
        let control_r_mat = control_mat * Mat4::<f32>::from(self.control_r);
        let hand_r_mat = control_r_mat * Mat4::<f32>::from(self.hand_r);

        let hand_l_mat = Mat4::<f32>::from(self.hand_l);
        let lantern_mat = if self.holding_lantern {
            hand_r_mat
        } else {
            shorts_mat
        } * Mat4::<f32>::from(self.lantern);
        let main_mat = control_l_mat * Mat4::<f32>::from(self.main);
        let second_mat = control_r_mat * Mat4::<f32>::from(self.second);
        let glider_mat = chest_mat * Mat4::<f32>::from(self.glider);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(head_mat),
            make_bone(chest_mat),
            make_bone(chest_mat * Mat4::<f32>::from(self.belt)),
            make_bone(chest_mat * Mat4::<f32>::from(self.back)),
            make_bone(shorts_mat),
            make_bone(control_l_mat * hand_l_mat),
            make_bone(hand_r_mat),
            make_bone(torso_mat * Mat4::<f32>::from(self.foot_l)),
            make_bone(torso_mat * Mat4::<f32>::from(self.foot_r)),
            make_bone(chest_mat * Mat4::<f32>::from(self.shoulder_l)),
            make_bone(chest_mat * Mat4::<f32>::from(self.shoulder_r)),
            make_bone(glider_mat),
            make_bone(main_mat),
            make_bone(second_mat),
            make_bone(lantern_mat),
            // FIXME: Should this be control_l_mat?
            make_bone(control_mat * hand_l_mat * Mat4::<f32>::from(self.hold)),
        ];
        let weapon_trails = self.main_weapon_trail || self.off_weapon_trail;
        Offsets {
            lantern: Some((lantern_mat * Vec4::new(0.0, 0.5, -6.0, 1.0)).xyz()),
            viewpoint: Some((head_mat * Vec4::new(0.0, 0.0, 4.0, 1.0)).xyz()),
            // TODO: see quadruped_medium for how to animate this
            mount_bone: Transform {
                position: comp::Body::Humanoid(body)
                    .mount_offset()
                    .into_tuple()
                    .into(),
                ..Default::default()
            },
            primary_trail_mat: if weapon_trails {
                self.main_weapon_trail
                    .then_some((main_mat, TrailSource::Weapon))
            } else {
                self.glider_trails
                    .then_some((glider_mat, TrailSource::GliderLeft))
            },
            secondary_trail_mat: if weapon_trails {
                self.off_weapon_trail
                    .then_some((second_mat, TrailSource::Weapon))
            } else {
                self.glider_trails
                    .then_some((glider_mat, TrailSource::GliderRight))
            },
        }
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

impl<'a> TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::Humanoid(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::humanoid::{BodyType::*, Species::*};
        Self {
            scaler: body.scaler(),
            head_scale: match (body.species, body.body_type) {
                (Orc, Male) => 0.9,
                (Orc, Female) => 0.9,
                (Human, Male) => 0.9,
                (Human, Female) => 0.9,
                (Elf, Male) => 0.9,
                (Elf, Female) => 0.9,
                (Dwarf, Male) => 1.0,
                (Dwarf, Female) => 1.0,
                (Draugr, Male) => 0.9,
                (Draugr, Female) => 0.9,
                (Danari, Male) => 1.15,
                (Danari, Female) => 1.15,
            },
            head: match (body.species, body.body_type) {
                (Orc, Male) => (-2.0, 9.0),
                (Orc, Female) => (-2.0, 9.5),
                (Human, Male) => (-2.3, 9.5),
                (Human, Female) => (-2.0, 9.5),
                (Elf, Male) => (-2.5, 9.5),
                (Elf, Female) => (-1.0, 9.5),
                (Dwarf, Male) => (-2.0, 10.0),
                (Dwarf, Female) => (-2.0, 9.5),
                (Draugr, Male) => (-1.5, 8.5),
                (Draugr, Female) => (-1.5, 9.5),
                (Danari, Male) => (-1.5, 7.0),
                (Danari, Female) => (-1.5, 7.0),
            },
            chest: (0.0, 8.0),
            belt: (0.0, -2.0),
            back: (-3.1, 7.25),
            shorts: (0.0, -5.0),
            hand: (7.0, -0.25, 0.5),
            foot: (3.4, 0.5, 2.0),
            shoulder: (5.0, 0.0, 5.0),
            lantern: (5.0, 2.5, 5.5),
            shl: (-0.75, -1.0, 0.5, 1.47, -0.2, 0.0),
            shr: (0.75, -1.5, -2.5, 1.47, 0.3, 0.0),
            sc: (-6.0, 6.0, 0.0, -0.5, 0.0, 0.0),
            hhl: (0.1, 0.0, 11.0, 4.71, 0.0, PI),
            hhr: (0.0, 0.0, 0.0, 4.71, 0.0, PI),
            hc: (6.0, 7.0, 1.0, -0.3, -PI / 2.0, 3.64),
            sthl: (0.0, 0.0, 6.0, 1.97, 0.0, 0.0),
            sthr: (0.0, 0.0, 0.0, 1.27, 0.2, 0.0),
            stc: (-5.0, 7.0, -2.0, -0.3, 0.15, 0.0),
            ahl: (-0.5, -1.0, 7.0, 1.17, PI, 0.0),
            ahr: (0.0, -1.0, 1.0, -2.0, 0.0, PI),
            ac: (-8.0, 11.0, 3.0, 2.0, 0.0, 0.0),
            bhl: (0.0, -4.0, 1.0, PI / 2.0, 0.0, 0.0),
            bhr: (1.0, 2.0, -2.0, PI / 2.0, 0.0, 0.0),
            bc: (-5.0, 9.0, 1.0, 0.0, 1.2, -0.6),
        }
    }
}
