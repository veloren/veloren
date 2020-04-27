pub mod alpha;
pub mod beta;
pub mod block;
pub mod blockidle;
pub mod charge;
pub mod climb;
pub mod dash;
pub mod equip;
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
    dash::DashAnimation, equip::EquipAnimation, gliding::GlidingAnimation, idle::IdleAnimation,
    jump::JumpAnimation, roll::RollAnimation, run::RunAnimation, shoot::ShootAnimation,
    sit::SitAnimation, spin::SpinAnimation, stand::StandAnimation, swim::SwimAnimation,
    wield::WieldAnimation,
};

use super::{Bone, Skeleton};
use crate::render::FigureBoneData;
use common::comp;

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

    fn bone_count(&self) -> usize { 15 }

    fn compute_matrices(&self) -> [FigureBoneData; 16] {
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
        [
            FigureBoneData::new(torso_mat * chest_mat * head_mat),
            FigureBoneData::new(torso_mat * chest_mat),
            FigureBoneData::new(torso_mat * chest_mat * self.belt.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat * self.back.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat * shorts_mat),
            FigureBoneData::new(torso_mat * chest_mat * control_mat * l_control_mat * l_hand_mat),
            FigureBoneData::new(torso_mat * chest_mat * control_mat * r_control_mat * r_hand_mat),
            FigureBoneData::new(torso_mat * self.l_foot.compute_base_matrix()),
            FigureBoneData::new(torso_mat * self.r_foot.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat * self.l_shoulder.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat * self.r_shoulder.compute_base_matrix()),
            FigureBoneData::new(torso_mat * self.glider.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat * control_mat * l_control_mat * main_mat),
            FigureBoneData::new(torso_mat * chest_mat * control_mat * r_control_mat * second_mat),
            FigureBoneData::new(
                torso_mat * chest_mat * shorts_mat * self.lantern.compute_base_matrix(),
            ),
            FigureBoneData::default(),
        ]
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
        use comp::humanoid::{BodyType::*, Race::*};
        match (body.race, body.body_type) {
            (Orc, Male) => 0.95,
            (Orc, Female) => 0.8,
            (Human, Male) => 0.8,
            (Human, Female) => 0.75,
            (Elf, Male) => 0.85,
            (Elf, Female) => 0.8,
            (Dwarf, Male) => 0.7,
            (Dwarf, Female) => 0.65,
            (Undead, Male) => 0.8,
            (Undead, Female) => 0.75,
            (Danari, Male) => 0.58,
            (Danari, Female) => 0.58,
        }
    }
}

impl<'a> From<&'a comp::humanoid::Body> for SkeletonAttr {
    fn from(body: &'a comp::humanoid::Body) -> Self {
        use comp::humanoid::{BodyType::*, Race::*};
        Self {
            scaler: SkeletonAttr::calculate_scale(body),
            head_scale: match (body.race, body.body_type) {
                (Orc, Male) => 0.9,
                (Orc, Female) => 1.0,
                (Human, Male) => 1.0,
                (Human, Female) => 1.0,
                (Elf, Male) => 0.95,
                (Elf, Female) => 1.0,
                (Dwarf, Male) => 1.0,
                (Dwarf, Female) => 1.0,
                (Undead, Male) => 1.0,
                (Undead, Female) => 1.0,
                (Danari, Male) => 1.15,
                (Danari, Female) => 1.15,
            },
            head: match (body.race, body.body_type) {
                (Orc, Male) => (0.0, 14.0),
                (Orc, Female) => (0.0, 14.0),
                (Human, Male) => (0.0, 14.5),
                (Human, Female) => (0.0, 14.0),
                (Elf, Male) => (0.0, 14.5),
                (Elf, Female) => (0.0, 14.5),
                (Dwarf, Male) => (0.0, 14.5),
                (Dwarf, Female) => (0.0, 14.0),
                (Undead, Male) => (0.5, 14.5),
                (Undead, Female) => (0.5, 14.5),
                (Danari, Male) => (0.5, 14.0),
                (Danari, Female) => (0.5, 14.0),
            },
            chest: match (body.race, body.body_type) {
                (_, _) => (0.0, 7.0),
            },
            belt: match (body.race, body.body_type) {
                (_, _) => (0.0, -2.0),
            },
            back: match (body.race, body.body_type) {
                (_, _) => (-3.1, 7.25),
            },
            shorts: match (body.race, body.body_type) {
                (_, _) => (0.0, -5.0),
            },
            hand: match (body.race, body.body_type) {
                (_, _) => (7.0, -0.25, 5.0),
            },
            foot: match (body.race, body.body_type) {
                (_, _) => (3.4, -0.1, 8.0),
            },
            shoulder: match (body.race, body.body_type) {
                (_, _) => (5.0, 0.0, 5.0),
            },
            lantern: match (body.race, body.body_type) {
                (_, _) => (5.0, 2.5, 5.5),
            },
        }
    }
}
