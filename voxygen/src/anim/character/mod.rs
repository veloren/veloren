pub mod run;
pub mod idle;

// Reexports
pub use self::run::RunAnimation;
pub use self::idle::IdleAnimation;

// Crate
use crate::render::FigureBoneData;

// Local
use super::{
    Skeleton,
    Bone,
};

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
    back: Bone,
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
            FigureBoneData::new(self.belt.compute_base_matrix()),
            FigureBoneData::new(self.shorts.compute_base_matrix()),
            FigureBoneData::new(self.l_hand.compute_base_matrix()),
            FigureBoneData::new(self.r_hand.compute_base_matrix()),
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

    fn interpolate(&mut self, target: &Self) {
        self.head.interpolate(&target.head);
        self.chest.interpolate(&target.chest);
        self.belt.interpolate(&target.belt);
        self.shorts.interpolate(&target.shorts);
        self.l_hand.interpolate(&target.l_hand);
        self.r_hand.interpolate(&target.r_hand);
        self.l_foot.interpolate(&target.l_foot);
        self.r_foot.interpolate(&target.r_foot);
        self.back.interpolate(&target.back);
    }
}
