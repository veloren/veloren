pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::idle::IdleAnimation;
pub use self::jump::JumpAnimation;
pub use self::run::RunAnimation;

use super::{Bone, Skeleton};
use crate::render::FigureBoneData;

#[derive(Clone)]
pub struct WildBoarSkeleton {
    wild_boar_head: Bone,
    wild_boar_torso: Bone,
    wild_boar_tail: Bone,
    wild_boar_leg_lf: Bone,
    wild_boar_leg_rf: Bone,
    wild_boar_leg_lb: Bone,
    wild_boar_leg_rb: Bone,
    wild_boar_foot_lf: Bone,
    wild_boar_foot_rf: Bone,
    wild_boar_foot_lb: Bone,
    wild_boar_foot_rb: Bone,
}

impl WildBoarSkeleton {
    pub fn new() -> Self {
        Self {
            wild_boar_head: Bone::default(),
            wild_boar_torso: Bone::default(),
            wild_boar_tail: Bone::default(),
            wild_boar_leg_lf: Bone::default(),
            wild_boar_leg_rf: Bone::default(),
            wild_boar_leg_lb: Bone::default(),
            wild_boar_leg_rb: Bone::default(),
            wild_boar_foot_lf: Bone::default(),
            wild_boar_foot_rf: Bone::default(),
            wild_boar_foot_lb: Bone::default(),
            wild_boar_foot_rb: Bone::default(),
        }
    }
}

impl Skeleton for WildBoarSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        [
            FigureBoneData::new(self.wild_boar_head.compute_base_matrix()),
            FigureBoneData::new(self.wild_boar_torso.compute_base_matrix()),
            FigureBoneData::new(self.wild_boar_tail.compute_base_matrix()),
            FigureBoneData::new(self.wild_boar_leg_lf.compute_base_matrix()),
            FigureBoneData::new(self.wild_boar_leg_rf.compute_base_matrix()),
            FigureBoneData::new(self.wild_boar_leg_lb.compute_base_matrix()),
            FigureBoneData::new(self.wild_boar_leg_rb.compute_base_matrix()),
            FigureBoneData::new(self.wild_boar_foot_lf.compute_base_matrix()),
            FigureBoneData::new(self.wild_boar_foot_rf.compute_base_matrix()),
            FigureBoneData::new(self.wild_boar_foot_lb.compute_base_matrix()),
            FigureBoneData::new(self.wild_boar_foot_rb.compute_base_matrix()),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.wild_boar_head.interpolate(&target.wild_boar_head, dt);
        self.wild_boar_torso
            .interpolate(&target.wild_boar_torso, dt);
        self.wild_boar_tail.interpolate(&target.wild_boar_tail, dt);
        self.wild_boar_leg_lf
            .interpolate(&target.wild_boar_leg_lf, dt);
        self.wild_boar_leg_rf
            .interpolate(&target.wild_boar_leg_rf, dt);
        self.wild_boar_leg_lb
            .interpolate(&target.wild_boar_leg_lb, dt);
        self.wild_boar_leg_rb
            .interpolate(&target.wild_boar_leg_rb, dt);
        self.wild_boar_foot_lf
            .interpolate(&target.wild_boar_foot_lf, dt);
        self.wild_boar_foot_rf
            .interpolate(&target.wild_boar_foot_rf, dt);
        self.wild_boar_foot_lb
            .interpolate(&target.wild_boar_foot_lb, dt);
        self.wild_boar_foot_rb
            .interpolate(&target.wild_boar_foot_rb, dt);
    }
}
