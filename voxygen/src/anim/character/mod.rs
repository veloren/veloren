pub mod run;

// Reexports
pub use self::run::RunAnimation;

// Crate
use crate::render::FigureBoneData;

// Local
use super::{
    Skeleton,
    Bone,
};

pub struct CharacterSkeleton {
    head: Bone,
    chest: Bone,
    bl_foot: Bone,
    br_foot: Bone,
    r_hand: Bone,
    l_hand: Bone,
    l_foot: Bone,
    r_foot: Bone,
    back: Bone,
}

impl CharacterSkeleton {
    pub fn new() -> Self {
        Self {
            head: Bone::default(),
            chest: Bone::default(),
            br_foot: Bone::default(),
            bl_foot: Bone::default(),
            r_hand: Bone::default(),
            l_hand: Bone::default(),
            l_foot: Bone::default(),
            r_foot: Bone::default(),
            back: Bone::default(),
        }
    }
}

impl Skeleton for CharacterSkeleton {
    fn compute_matrices(&self) -> [FigureBoneData; 16] {
        let chest_mat = self.chest.compute_base_matrix();

        [
            FigureBoneData::new(self.head.compute_base_matrix()),
            FigureBoneData::new(chest_mat),
            FigureBoneData::new(self.bl_foot.compute_base_matrix()),
            FigureBoneData::new(self.br_foot.compute_base_matrix()),
            FigureBoneData::new(self.r_hand.compute_base_matrix()),
            FigureBoneData::new(self.l_hand.compute_base_matrix()),
            FigureBoneData::new(self.l_foot.compute_base_matrix()),
            FigureBoneData::new(self.r_foot.compute_base_matrix()),
            FigureBoneData::new(chest_mat * self.back.compute_base_matrix()),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
            FigureBoneData::default(),
        ]
    }
}
