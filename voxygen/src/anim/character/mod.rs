pub mod attack;
pub mod block;
pub mod cidle;
pub mod cjump;
pub mod crun;
pub mod gliding;
pub mod idle;
pub mod jump;
pub mod roll;
pub mod run;

// Reexports
pub use self::attack::AttackAnimation;
pub use self::block::BlockAnimation;
pub use self::cidle::CidleAnimation;
pub use self::cjump::CjumpAnimation;
pub use self::crun::CrunAnimation;
pub use self::gliding::GlidingAnimation;
pub use self::idle::IdleAnimation;
pub use self::jump::JumpAnimation;
pub use self::roll::RollAnimation;
pub use self::run::RunAnimation;

use super::{Bone, Skeleton};
use crate::render::FigureBoneData;

#[derive(Clone)]
pub struct CharacterSkeleton {
    head: Bone,
    chest: Bone,
    belt: Bone,
    shorts: Bone,
    l_hand: Bone,
    r_hand: Bone,
    l_foot: Bone,
    r_foot: Bone,
    weapon: Bone,
    l_shoulder: Bone,
    r_shoulder: Bone,
    draw: Bone,
    torso: Bone,
}

impl CharacterSkeleton {
    pub fn new() -> Self {
        Self {
            head: Bone::default(),
            chest: Bone::default(),
            belt: Bone::default(),
            shorts: Bone::default(),
            l_hand: Bone::default(),
            r_hand: Bone::default(),
            l_foot: Bone::default(),
            r_foot: Bone::default(),
            weapon: Bone::default(),
            l_shoulder: Bone::default(),
            r_shoulder: Bone::default(),
            draw: Bone::default(),
            torso: Bone::default(),
        }
    }
}

impl Skeleton for CharacterSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let chest_mat = self.chest.compute_base_matrix();
        let torso_mat = self.torso.compute_base_matrix();
        let l_hand_mat = self.l_hand.compute_base_matrix();
        let weapon_mat = self.weapon.compute_base_matrix();
        let head_mat = self.head.compute_base_matrix();
        [
            FigureBoneData::new(torso_mat * head_mat),
            FigureBoneData::new(torso_mat * chest_mat),
            FigureBoneData::new(torso_mat * self.belt.compute_base_matrix()),
            FigureBoneData::new(torso_mat * self.shorts.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat * l_hand_mat),
            FigureBoneData::new(torso_mat * chest_mat * self.r_hand.compute_base_matrix()),
            FigureBoneData::new(torso_mat * self.l_foot.compute_base_matrix()),
            FigureBoneData::new(torso_mat * self.r_foot.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat * weapon_mat),
            FigureBoneData::new(torso_mat * chest_mat * self.l_shoulder.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat * self.r_shoulder.compute_base_matrix()),
            FigureBoneData::new(torso_mat * self.draw.compute_base_matrix()),
            FigureBoneData::new(torso_mat),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.head.interpolate(&target.head, dt);
        self.chest.interpolate(&target.chest, dt);
        self.belt.interpolate(&target.belt, dt);
        self.shorts.interpolate(&target.shorts, dt);
        self.l_hand.interpolate(&target.l_hand, dt);
        self.r_hand.interpolate(&target.r_hand, dt);
        self.l_foot.interpolate(&target.l_foot, dt);
        self.r_foot.interpolate(&target.r_foot, dt);
        self.weapon.interpolate(&target.weapon, dt);
        self.l_shoulder.interpolate(&target.l_shoulder, dt);
        self.r_shoulder.interpolate(&target.r_shoulder, dt);
        self.draw.interpolate(&target.draw, dt);
        self.torso.interpolate(&target.torso, dt);
    }
}
