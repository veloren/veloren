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
pub struct DragonSkeleton {
    dragon_head: Bone,
    dragon_chest_front: Bone,
    dragon_chest_rear: Bone,
    dragon_tail_front: Bone,
    dragon_tail_rear: Bone,
    dragon_wing_in_l: Bone,
    dragon_wing_in_r: Bone,
    dragon_wing_out_l: Bone,
    dragon_wing_out_r: Bone,
    dragon_foot_fl: Bone,
    dragon_foot_fr: Bone,
    dragon_foot_bl: Bone,
    dragon_foot_br: Bone,

}

impl DragonSkeleton {
    pub fn new() -> Self {
        Self {
    dragon_head: Bone::default(),
    dragon_chest_front: Bone::default(),
    dragon_chest_rear: Bone::default(),
    dragon_tail_front: Bone::default(),
    dragon_tail_rear: Bone::default(),
    dragon_wing_in_l: Bone::default(),
    dragon_wing_in_r: Bone::default(),
    dragon_wing_out_l: Bone::default(),
    dragon_wing_out_r: Bone::default(),
    dragon_foot_fl: Bone::default(),
    dragon_foot_fr: Bone::default(),
    dragon_foot_bl: Bone::default(),
    dragon_foot_br: Bone::default(),


        }
    }
}

impl Skeleton for DragonSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let chest_front_mat = self.dragon_chest_front.compute_base_matrix();
        let wing_in_l_mat = self.dragon_wing_in_l.compute_base_matrix();
        let wing_in_r_mat = self.dragon_wing_in_r.compute_base_matrix();
        let tail_front_mat = self.dragon_tail_front.compute_base_matrix();


        [
            FigureBoneData::new(self.dragon_head.compute_base_matrix() * chest_front_mat),
            FigureBoneData::new(
                chest_front_mat,
            ),
            FigureBoneData::new(self.dragon_chest_rear.compute_base_matrix() * chest_front_mat),
            FigureBoneData::new(tail_front_mat),
            FigureBoneData::new(self.dragon_tail_rear.compute_base_matrix() * tail_front_mat),
            FigureBoneData::new(wing_in_l_mat),
            FigureBoneData::new(wing_in_r_mat),
            FigureBoneData::new(self.dragon_wing_out_l.compute_base_matrix() * wing_in_l_mat),
            FigureBoneData::new(self.dragon_wing_out_r.compute_base_matrix() * wing_in_r_mat),
            FigureBoneData::new(self.dragon_foot_fl.compute_base_matrix()),
            FigureBoneData::new(self.dragon_foot_fr.compute_base_matrix()),
            FigureBoneData::new(self.dragon_foot_bl.compute_base_matrix()),
            FigureBoneData::new(self.dragon_foot_br.compute_base_matrix()),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }

    fn interpolate(&mut self, target: &Self, dt: f32) {
        self.dragon_head.interpolate(&target.dragon_head, dt);
        self.dragon_chest_front.interpolate(&target.dragon_chest_front, dt);
        self.dragon_chest_rear.interpolate(&target.dragon_chest_rear, dt);
        self.dragon_tail_front.interpolate(&target.dragon_tail_front, dt);
        self.dragon_tail_rear.interpolate(&target.dragon_tail_rear, dt);
        self.dragon_wing_in_l.interpolate(&target.dragon_wing_in_l, dt);
        self.dragon_wing_in_r.interpolate(&target.dragon_wing_in_r, dt);
        self.dragon_wing_out_l.interpolate(&target.dragon_wing_out_l, dt);
        self.dragon_wing_out_r.interpolate(&target.dragon_wing_out_r, dt);
        self.dragon_foot_fl.interpolate(&target.dragon_foot_fl, dt);
        self.dragon_foot_fr.interpolate(&target.dragon_foot_fr, dt);
        self.dragon_foot_bl.interpolate(&target.dragon_foot_bl, dt);
        self.dragon_foot_br.interpolate(&target.dragon_foot_br, dt);


    }
}
