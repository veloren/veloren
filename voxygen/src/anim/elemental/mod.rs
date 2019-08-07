<<<<<<< HEAD
=======
pub mod attack;
>>>>>>> 7a12421481d84594a1aced62362df30b218b085d
pub mod idle;
pub mod jump;
pub mod run;

// Reexports
<<<<<<< HEAD
=======
pub use self::attack::AttackAnimation;
>>>>>>> 7a12421481d84594a1aced62362df30b218b085d
pub use self::idle::IdleAnimation;
pub use self::jump::JumpAnimation;
pub use self::run::RunAnimation;

use super::{Bone, Skeleton};
use crate::render::FigureBoneData;

#[derive(Clone)]
pub struct ElementalSkeleton {
    head: Bone,
    upper_torso: Bone,
    lower_torso: Bone,
    shoulder_l: Bone,
    shoulder_r: Bone,
    hand_l: Bone,
    hand_r: Bone,
    feet: Bone,
}

impl ElementalSkeleton {
    pub fn new() -> Self {
        Self {
            head: Bone::default(),
            upper_torso: Bone::default(),
            lower_torso: Bone::default(),
            shoulder_l: Bone::default(),
            shoulder_r: Bone::default(),
            hand_l: Bone::default(),
            hand_r: Bone::default(),
            feet: Bone::default(),
        }
    }
}

impl Skeleton for ElementalSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        [
            FigureBoneData::new(self.head.compute_base_matrix()),
            FigureBoneData::new(self.upper_torso.compute_base_matrix()),
            FigureBoneData::new(self.lower_torso.compute_base_matrix()),
            FigureBoneData::new(self.shoulder_l.compute_base_matrix()),
            FigureBoneData::new(self.shoulder_r.compute_base_matrix()),
            FigureBoneData::new(self.hand_l.compute_base_matrix()),
            FigureBoneData::new(self.hand_r.compute_base_matrix()),
            FigureBoneData::new(self.feet.compute_base_matrix()),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.head.interpolate(&target.head, dt);
        self.upper_torso.interpolate(&target.upper_torso, dt);
        self.lower_torso.interpolate(&target.lower_torso, dt);
        self.shoulder_l.interpolate(&target.shoulder_l, dt);
        self.shoulder_r.interpolate(&target.shoulder_r, dt);
        self.hand_l.interpolate(&target.hand_l, dt);
        self.hand_r.interpolate(&target.hand_r, dt);
        self.feet.interpolate(&target.feet, dt);
    }
}
