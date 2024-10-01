pub mod basic;
pub mod boost;
pub mod climb;
pub mod collect;
pub mod consume;
pub mod crawl;
pub mod dance;
pub mod equip;
pub mod glidewield;
pub mod gliding;
pub mod idle;
pub mod jump;
pub mod mount;
pub mod multi;
pub mod music;
pub mod pet;
pub mod roll;
pub mod run;
pub mod sit;
pub mod sleep;
pub mod sneak;
pub mod sneakequip;
pub mod sneakwield;
pub mod staggered;
pub mod stand;
pub mod steer;
pub mod stunned;
pub mod swim;
pub mod swimwield;
pub mod talk;
pub mod wallrun;
pub mod wield;

// Reexports
pub use self::{
    basic::{BasicAction, BasicActionDependency},
    boost::BoostAnimation,
    climb::ClimbAnimation,
    collect::CollectAnimation,
    consume::ConsumeAnimation,
    crawl::CrawlAnimation,
    dance::DanceAnimation,
    equip::EquipAnimation,
    glidewield::GlideWieldAnimation,
    gliding::GlidingAnimation,
    idle::IdleAnimation,
    jump::JumpAnimation,
    mount::MountAnimation,
    multi::{MultiAction, MultiActionDependency},
    music::MusicAnimation,
    pet::PetAnimation,
    roll::RollAnimation,
    run::RunAnimation,
    sit::SitAnimation,
    sleep::SleepAnimation,
    sneak::SneakAnimation,
    sneakequip::SneakEquipAnimation,
    sneakwield::SneakWieldAnimation,
    staggered::StaggeredAnimation,
    stand::StandAnimation,
    steer::SteerAnimation,
    stunned::StunnedAnimation,
    swim::SwimAnimation,
    swimwield::SwimWieldAnimation,
    talk::TalkAnimation,
    wallrun::WallrunAnimation,
    wield::WieldAnimation,
};
use super::{FigureBoneData, Offsets, Skeleton, TrailSource, make_bone, vek::*};
use common::comp::{
    self,
    tool::{Hands, ToolKind},
};
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
    // The offset from the back that carried weapons should be given to avoid clipping due to, say, a backpack
    back_carry_offset: f32,
    main_weapon_trail: bool,
    off_weapon_trail: bool,
    // Cannot exist at same time as weapon trails. Since gliding and attacking are mutually exclusive, should never be a concern.
    glider_trails: bool,
});

impl CharacterSkeleton {
    pub fn new(holding_lantern: bool, back_carry_offset: f32) -> Self {
        Self {
            holding_lantern,
            back_carry_offset,
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

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "character_compute_mats"))]
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

        // Offset from the mounted bone's origin.
        // Note: This could be its own bone if we need to animate it independently.
        let mount_position = (chest_mat * Vec4::from_point(Vec3::new(5.5, 0.0, 6.5)))
            .homogenized()
            .xyz();
        // NOTE: We apply the ori from base_mat externally so we don't need to worry
        // about it here for now.
        let mount_orientation =
            self.torso.orientation * self.chest.orientation * Quaternion::rotation_y(0.4);

        let weapon_trails = self.main_weapon_trail || self.off_weapon_trail;
        Offsets {
            lantern: Some((lantern_mat * Vec4::new(0.0, 0.5, -6.0, 1.0)).xyz()),
            viewpoint: Some((head_mat * Vec4::new(0.0, 0.0, 4.0, 1.0)).xyz()),
            mount_bone: Transform {
                position: mount_position,
                orientation: mount_orientation,
                scale: Vec3::one(),
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
            ..Default::default()
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

impl CharacterSkeleton {
    /// Animate tools (main and secondary) on the character's back, taking in
    /// account backpack offsets.
    pub fn do_tools_on_back(
        &mut self,
        hands: (Option<Hands>, Option<Hands>),
        active_tool_kind: Option<ToolKind>,
        second_tool_kind: Option<ToolKind>,
    ) {
        match (hands, active_tool_kind, second_tool_kind) {
            ((Some(Hands::Two), _), tool, _) | ((None, Some(Hands::Two)), _, tool) => match tool {
                Some(ToolKind::Bow) => {
                    self.main.position = Vec3::new(0.0, -5.0 - self.back_carry_offset, 6.0);
                    self.main.orientation =
                        Quaternion::rotation_y(2.5) * Quaternion::rotation_z(PI / 2.0);
                },
                Some(ToolKind::Staff) | Some(ToolKind::Sceptre) => {
                    self.main.position = Vec3::new(2.0, -5.0 - self.back_carry_offset, -1.0);
                    self.main.orientation =
                        Quaternion::rotation_y(-0.5) * Quaternion::rotation_z(PI / 2.0);
                },
                Some(ToolKind::Shield) => {
                    self.main.position = Vec3::new(-2.0, -3.0 - self.back_carry_offset, 1.0);
                    self.main.orientation =
                        Quaternion::rotation_y(-0.75) * Quaternion::rotation_z(PI / 2.0);
                },
                _ => {
                    self.main.position = Vec3::new(-7.0, -5.0 - self.back_carry_offset, 15.0);
                    self.main.orientation =
                        Quaternion::rotation_y(2.5) * Quaternion::rotation_z(PI / 2.0);
                },
            },
            ((_, _), _, _) => {},
        }

        match hands {
            (Some(Hands::One), _) => match active_tool_kind {
                Some(ToolKind::Dagger) => {
                    self.main.position = Vec3::new(5.0, 1.0 - self.back_carry_offset, 2.0);
                    self.main.orientation =
                        Quaternion::rotation_x(-1.35 * PI) * Quaternion::rotation_z(2.0 * PI);
                },
                Some(ToolKind::Axe) | Some(ToolKind::Hammer) | Some(ToolKind::Sword) => {
                    self.main.position = Vec3::new(-4.0, -4.5 - self.back_carry_offset, 10.0);
                    self.main.orientation =
                        Quaternion::rotation_y(2.5) * Quaternion::rotation_z(PI / 2.0);
                },
                Some(ToolKind::Shield) => {
                    self.main.position = Vec3::new(-2.0, -4.0 - self.back_carry_offset, 3.0);
                    self.main.orientation =
                        Quaternion::rotation_y(0.25 * PI) * Quaternion::rotation_z(-1.5 * PI);
                },
                Some(ToolKind::Throwable) => {
                    self.main.position = Vec3::new(-6.0, 0.0, -4.0);
                    self.main.scale = Vec3::zero();
                },
                _ => {},
            },
            (_, _) => {},
        }
        match hands {
            (None | Some(Hands::One), Some(Hands::One)) => match second_tool_kind {
                Some(ToolKind::Dagger) => {
                    self.second.position = Vec3::new(-5.0, 1.0 - self.back_carry_offset, 2.0);
                    self.second.orientation =
                        Quaternion::rotation_x(-1.35 * PI) * Quaternion::rotation_z(-2.0 * PI);
                },
                Some(ToolKind::Axe) | Some(ToolKind::Hammer) | Some(ToolKind::Sword) => {
                    self.second.position = Vec3::new(4.0, -5.0 - self.back_carry_offset, 10.0);
                    self.second.orientation =
                        Quaternion::rotation_y(-2.5) * Quaternion::rotation_z(-PI / 2.0);
                },
                Some(ToolKind::Shield) => {
                    self.second.position = Vec3::new(1.5, -4.0 - self.back_carry_offset, 3.0);
                    self.second.orientation =
                        Quaternion::rotation_y(-0.25 * PI) * Quaternion::rotation_z(1.5 * PI);
                },
                Some(ToolKind::Throwable) => {
                    self.second.position = Vec3::new(6.0, 0.0, -4.0);
                    self.second.scale = Vec3::zero();
                },
                _ => {},
            },
            (_, _) => {},
        }
    }

    /// If we're holding a lantern, animate hold the lantern in a reasonable
    /// position.
    pub fn do_hold_lantern(
        &mut self,
        s_a: &SkeletonAttr,
        anim_time: f32,
        acc_vel: f32,
        speednorm: f32,
        impact: f32,
        tilt: f32,
    ) {
        let lab = 2.0 / s_a.scaler;

        let short = ((5.0 / (1.5 + 3.5 * ((acc_vel * lab * 1.6 + PI * 0.5).sin()).powi(2))).sqrt())
            * ((acc_vel * lab * 1.6 + PI * 0.5).sin());

        let shorte = ((1.0 / (0.8 + 0.2 * ((acc_vel * lab * 1.6).sin()).powi(2))).sqrt())
            * ((acc_vel * lab * 1.6).sin());

        self.lantern.position = Vec3::new(s_a.lantern.0, s_a.lantern.1, s_a.lantern.2);
        self.lantern.orientation = Quaternion::rotation_x(shorte * 0.7 * speednorm.powi(2) + 0.4)
            * Quaternion::rotation_y(shorte * 0.4 * speednorm.powi(2));
        self.lantern.scale = Vec3::one() * 0.65;
        self.hold.scale = Vec3::one() * 0.0;

        if self.holding_lantern {
            self.hand_r.position = Vec3::new(
                s_a.hand.0 + 1.0,
                s_a.hand.1 + 2.0 - impact * 0.2,
                s_a.hand.2 + 12.0 + impact * -0.1,
            );
            self.hand_r.orientation = Quaternion::rotation_x(2.25) * Quaternion::rotation_z(0.9);
            self.shoulder_r.orientation = Quaternion::rotation_x(short * -0.15 + 2.0);

            let fast = (anim_time * 8.0).sin();
            let fast2 = (anim_time * 6.0 + 8.0).sin();

            self.lantern.position = Vec3::new(-0.5, -0.5, -2.5);
            self.lantern.orientation = self.hand_r.orientation.inverse()
                * Quaternion::rotation_x(
                    (fast + 0.5) * 1.0 * speednorm + (tilt.abs() * 2.0).min(PI * 0.5),
                )
                * Quaternion::rotation_y(tilt * 1.0 * fast + tilt * 1.0 + fast2 * speednorm * 0.25);
        }
    }
}

pub fn hammer_start(next: &mut CharacterSkeleton, s_a: &SkeletonAttr) {
    next.main.position = Vec3::new(0.0, 0.0, 0.0);
    next.main.orientation = Quaternion::rotation_z(0.0);
    next.hand_l.position = Vec3::new(s_a.hhl.0, s_a.hhl.1 + 3.0, s_a.hhl.2 - 1.0);
    next.hand_l.orientation = Quaternion::rotation_x(s_a.hhl.3)
        * Quaternion::rotation_y(s_a.hhl.4)
        * Quaternion::rotation_z(s_a.hhl.5);
    next.hand_r.position = Vec3::new(s_a.hhr.0, s_a.hhr.1 + 3.0, s_a.hhr.2 + 1.0);
    next.hand_r.orientation = Quaternion::rotation_x(s_a.hhr.3)
        * Quaternion::rotation_y(s_a.hhr.4)
        * Quaternion::rotation_z(s_a.hhr.5);

    next.control.position = Vec3::new(s_a.hc.0 - 1.0, s_a.hc.1, s_a.hc.2 - 3.0);
    next.control.orientation = Quaternion::rotation_x(s_a.hc.3)
        * Quaternion::rotation_y(s_a.hc.4)
        * Quaternion::rotation_z(s_a.hc.5);
}

pub fn twist_back(next: &mut CharacterSkeleton, move1: f32, c: f32, h: f32, b: f32, s: f32) {
    next.chest.orientation.rotate_z(move1 * c);
    next.head.orientation.rotate_z(move1 * -h);
    next.belt.orientation.rotate_z(move1 * -b);
    next.shorts.orientation.rotate_z(move1 * -s);
}

pub fn twist_forward(next: &mut CharacterSkeleton, move2: f32, c: f32, h: f32, b: f32, s: f32) {
    next.chest.orientation.rotate_z(move2 * -c);
    next.head.orientation.rotate_z(move2 * h);
    next.belt.orientation.rotate_z(move2 * b);
    next.shorts.orientation.rotate_z(move2 * s);
}

pub fn dual_wield_start(next: &mut CharacterSkeleton) {
    next.main.position = Vec3::new(0.0, 0.0, 0.0);
    next.main.orientation = Quaternion::rotation_z(0.0);
    next.second.position = Vec3::new(0.0, 0.0, 0.0);
    next.second.orientation = Quaternion::rotation_z(0.0);

    next.control_l.position =
        next.hand_l.position * Vec3::new(0.5, 0.5, 0.3) + Vec3::new(-4.0, 0.0, 0.0);
    next.control_l.orientation = Quaternion::lerp(
        next.hand_l.orientation,
        Quaternion::rotation_x(PI * -0.5),
        0.65,
    );
    next.hand_l.position = Vec3::new(0.0, -2.0, 0.0);
    next.hand_l.orientation = Quaternion::rotation_x(PI * 0.5);

    next.control_r.position =
        next.hand_r.position * Vec3::new(0.5, 0.5, 0.3) + Vec3::new(4.0, 0.0, 0.0);
    next.control_r.orientation = Quaternion::lerp(
        next.hand_r.orientation,
        Quaternion::rotation_x(PI * -0.5),
        0.65,
    );
    next.hand_r.position = Vec3::new(0.0, -2.0, 0.0);
    next.hand_r.orientation = Quaternion::rotation_x(PI * 0.5);
}
