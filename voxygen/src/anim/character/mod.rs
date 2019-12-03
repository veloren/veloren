pub mod attack;
pub mod block;
pub mod blockidle;
pub mod charge;
pub mod cidle;
pub mod climb;
pub mod gliding;
pub mod idle;
pub mod jump;
pub mod roll;
pub mod run;
pub mod sit;
pub mod stand;
pub mod swim;
pub mod wield;

// Reexports
pub use self::attack::AttackAnimation;
pub use self::block::BlockAnimation;
pub use self::blockidle::BlockIdleAnimation;
pub use self::charge::ChargeAnimation;
pub use self::cidle::CidleAnimation;
pub use self::climb::ClimbAnimation;
pub use self::gliding::GlidingAnimation;
pub use self::idle::IdleAnimation;
pub use self::jump::JumpAnimation;
pub use self::roll::RollAnimation;
pub use self::run::RunAnimation;
pub use self::sit::SitAnimation;
pub use self::stand::StandAnimation;
pub use self::swim::SwimAnimation;
pub use self::wield::WieldAnimation;

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
    main: Bone,
    l_shoulder: Bone,
    r_shoulder: Bone,
    glider: Bone,
    lantern: Bone,
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
            main: Bone::default(),
            l_shoulder: Bone::default(),
            r_shoulder: Bone::default(),
            glider: Bone::default(),
            lantern: Bone::default(),
            torso: Bone::default(),
        }
    }
}

impl Skeleton for CharacterSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let chest_mat = self.chest.compute_base_matrix();
        let torso_mat = self.torso.compute_base_matrix();
        let l_hand_mat = self.l_hand.compute_base_matrix();
        let main_mat = self.main.compute_base_matrix();

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
            FigureBoneData::new(torso_mat * chest_mat * main_mat),
            FigureBoneData::new(torso_mat * chest_mat * self.l_shoulder.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat * self.r_shoulder.compute_base_matrix()),
            FigureBoneData::new(torso_mat * self.glider.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat * self.lantern.compute_base_matrix()),
            FigureBoneData::new(torso_mat),
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
        self.main.interpolate(&target.main, dt);
        self.l_shoulder.interpolate(&target.l_shoulder, dt);
        self.r_shoulder.interpolate(&target.r_shoulder, dt);
        self.glider.interpolate(&target.glider, dt);
        self.lantern.interpolate(&target.lantern, dt);
        self.torso.interpolate(&target.torso, dt);
    }
}
