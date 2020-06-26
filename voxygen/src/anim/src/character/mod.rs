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
pub mod roll;
pub mod run;
pub mod shoot;
pub mod sit;
pub mod spin;
pub mod stand;
pub mod swim;
pub mod wield;

// Reexports
pub use self::{
    alpha::AlphaAnimation, beta::BetaAnimation, block::BlockAnimation,
    blockidle::BlockIdleAnimation, charge::ChargeAnimation, climb::ClimbAnimation,
    dance::DanceAnimation, dash::DashAnimation, equip::EquipAnimation,
    glidewield::GlideWieldAnimation, gliding::GlidingAnimation, idle::IdleAnimation,
    jump::JumpAnimation, roll::RollAnimation, run::RunAnimation, shoot::ShootAnimation,
    sit::SitAnimation, spin::SpinAnimation, stand::StandAnimation, swim::SwimAnimation,
    wield::WieldAnimation,
};

use super::{Bone, FigureBoneData, Skeleton};
use common::comp;
use vek::{Vec3, Vec4};

#[derive(Clone, Default)]
pub struct CharacterSkeleton {
    head: Bone,
    chest: Bone,
    belt: Bone,
    back: Bone,
    shorts: Bone,
    l_hand: Bone,
    r_hand: Bone,
    l_foot: Bone,
    r_foot: Bone,
    l_shoulder: Bone,
    r_shoulder: Bone,
    glider: Bone,
    main: Bone,
    second: Bone,
    lantern: Bone,
    hold: Bone,
    torso: Bone,
    control: Bone,
    l_control: Bone,
    r_control: Bone,
}

impl CharacterSkeleton {
    pub fn new() -> Self { Self::default() }
}

impl Skeleton for CharacterSkeleton {
    type Attr = SkeletonAttr;

    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"character_compute_mats\0";

    fn bone_count(&self) -> usize { 16 }

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_compute_mats")]

    fn compute_matrices_inner(&self) -> ([FigureBoneData; 16], Vec3<f32>) {
        let chest_mat = self.chest.compute_base_matrix();
        let torso_mat = self.torso.compute_base_matrix();
        let l_hand_mat = self.l_hand.compute_base_matrix();
        let r_hand_mat = self.r_hand.compute_base_matrix();
        let control_mat = self.control.compute_base_matrix();
        let l_control_mat = self.l_control.compute_base_matrix();
        let r_control_mat = self.r_control.compute_base_matrix();
        let main_mat = self.main.compute_base_matrix();
        let second_mat = self.second.compute_base_matrix();
        let shorts_mat = self.shorts.compute_base_matrix();
        let head_mat = self.head.compute_base_matrix();

        let lantern_final_mat =
            torso_mat * chest_mat * shorts_mat * self.lantern.compute_base_matrix();

        (
            [
                FigureBoneData::new(torso_mat * chest_mat * head_mat),
                FigureBoneData::new(torso_mat * chest_mat),
                FigureBoneData::new(torso_mat * chest_mat * self.belt.compute_base_matrix()),
                FigureBoneData::new(torso_mat * chest_mat * self.back.compute_base_matrix()),
                FigureBoneData::new(torso_mat * chest_mat * shorts_mat),
                FigureBoneData::new(
                    torso_mat * chest_mat * control_mat * l_control_mat * l_hand_mat,
                ),
                FigureBoneData::new(
                    torso_mat * chest_mat * control_mat * r_control_mat * r_hand_mat,
                ),
                FigureBoneData::new(torso_mat * self.l_foot.compute_base_matrix()),
                FigureBoneData::new(torso_mat * self.r_foot.compute_base_matrix()),
                FigureBoneData::new(torso_mat * chest_mat * self.l_shoulder.compute_base_matrix()),
                FigureBoneData::new(torso_mat * chest_mat * self.r_shoulder.compute_base_matrix()),
                FigureBoneData::new(torso_mat * chest_mat * self.glider.compute_base_matrix()),
                FigureBoneData::new(torso_mat * chest_mat * control_mat * l_control_mat * main_mat),
                FigureBoneData::new(
                    torso_mat * chest_mat * control_mat * r_control_mat * second_mat,
                ),
                FigureBoneData::new(lantern_final_mat),
                FigureBoneData::new(
                    torso_mat
                        * chest_mat
                        * control_mat
                        * l_hand_mat
                        * self.hold.compute_base_matrix(),
                ),
            ],
            (lantern_final_mat * Vec4::new(0.0, 0.0, 0.0, 1.0)).xyz(),
        )
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.head.interpolate(&target.head, dt);
        self.chest.interpolate(&target.chest, dt);
        self.belt.interpolate(&target.belt, dt);
        self.back.interpolate(&target.back, dt);
        self.shorts.interpolate(&target.shorts, dt);
        self.l_hand.interpolate(&target.l_hand, dt);
        self.r_hand.interpolate(&target.r_hand, dt);
        self.l_foot.interpolate(&target.l_foot, dt);
        self.r_foot.interpolate(&target.r_foot, dt);
        self.l_shoulder.interpolate(&target.l_shoulder, dt);
        self.r_shoulder.interpolate(&target.r_shoulder, dt);
        self.glider.interpolate(&target.glider, dt);
        self.main.interpolate(&target.main, dt);
        self.second.interpolate(&target.second, dt);
        self.lantern.interpolate(&target.lantern, dt);
        self.hold.interpolate(&target.hold, dt);
        self.torso.interpolate(&target.torso, dt);
        self.control.interpolate(&target.control, dt);
        self.l_control.interpolate(&target.l_control, dt);
        self.r_control.interpolate(&target.r_control, dt);
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

impl SkeletonAttr {
    pub fn calculate_scale(body: &comp::humanoid::Body) -> f32 {
        use comp::humanoid::{BodyType::*, Species::*};
        match (body.species, body.body_type) {
            (Orc, Male) => 1.14,
            (Orc, Female) => 1.02,
            (Human, Male) => 1.02,
            (Human, Female) => 0.96,
            (Elf, Male) => 1.02,
            (Elf, Female) => 0.96,
            (Dwarf, Male) => 0.84,
            (Dwarf, Female) => 0.78,
            (Undead, Male) => 0.96,
            (Undead, Female) => 0.9,
            (Danari, Male) => 0.696,
            (Danari, Female) => 0.696,
        }
    }
}

impl<'a> From<&'a comp::humanoid::Body> for SkeletonAttr {
    #[allow(clippy::match_single_binding)] // TODO: Pending review in #587
    fn from(body: &'a comp::humanoid::Body) -> Self {
        use comp::humanoid::{BodyType::*, Species::*};
        Self {
            scaler: SkeletonAttr::calculate_scale(body),
            head_scale: match (body.species, body.body_type) {
                (Orc, Male) => 0.9,
                (Orc, Female) => 1.0,
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
