pub mod gliding;
pub mod idle;
pub mod jump;
pub mod run;
pub mod attack;

// Reexports
pub use self::gliding::GlidingAnimation;
pub use self::idle::IdleAnimation;
pub use self::jump::JumpAnimation;
pub use self::run::RunAnimation;
pub use self::attack::AttackAnimation;

// Crate
use crate::render::FigureBoneData;

// Local
use super::{Bone, Skeleton};

const SCALE: f32 = 11.0;

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
    l_hold: Bone,
    r_hold: Bone,
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
            l_hold: Bone::default(),
            r_hold: Bone::default(),
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
        [
            FigureBoneData::new(torso_mat * self.head.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat),
            FigureBoneData::new(torso_mat * self.belt.compute_base_matrix()),
            FigureBoneData::new(torso_mat * self.shorts.compute_base_matrix()),
            FigureBoneData::new(torso_mat * l_hand_mat),
            FigureBoneData::new(torso_mat * self.r_hand.compute_base_matrix()),
            FigureBoneData::new(torso_mat * self.l_foot.compute_base_matrix()),
            FigureBoneData::new(torso_mat * self.r_foot.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat * self.l_hold.compute_base_matrix() * weapon_mat),
            FigureBoneData::new(torso_mat * chest_mat * self.l_shoulder.compute_base_matrix()),
            FigureBoneData::new(torso_mat * chest_mat * self.r_shoulder.compute_base_matrix()),
            FigureBoneData::new(torso_mat * l_hand_mat * self.draw.compute_base_matrix()),
            FigureBoneData::new(torso_mat * self.l_hold.compute_base_matrix()),
            FigureBoneData::new(torso_mat * self.r_hold.compute_base_matrix()),
            FigureBoneData::new(torso_mat),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self) {
        self.head.interpolate(&target.head);
        self.chest.interpolate(&target.chest);
        self.belt.interpolate(&target.belt);
        self.shorts.interpolate(&target.shorts);
        self.l_hand.interpolate(&target.l_hand);
        self.r_hand.interpolate(&target.r_hand);
        self.l_foot.interpolate(&target.l_foot);
        self.r_foot.interpolate(&target.r_foot);
        self.weapon.interpolate(&target.weapon);
        self.l_shoulder.interpolate(&target.l_shoulder);
        self.r_shoulder.interpolate(&target.r_shoulder);
        self.draw.interpolate(&target.draw);
        self.l_hold.interpolate(&target.l_hold);
        self.r_hold.interpolate(&target.r_hold);
        self.torso.interpolate(&target.torso);
    }
}
