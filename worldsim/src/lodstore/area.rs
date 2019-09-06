use super::index::{
    LodIndex,
};

/*
    A LodArea is the area between 2 LodIndex
*/

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct LodArea {
    pub lower: LodIndex,
    pub upper: LodIndex,
}

impl LodArea {
    pub fn new(lower: LodIndex, upper: LodIndex) -> Self {
        LodArea {
            lower,
            upper,
        }
    }

    pub fn is_inside(&self, lod: LodIndex) -> bool {
        let lower = self.lower.get();
        let upper = self.upper.get();
        let lod = lod.get();
        lod[0] >= lower[0] && lod[0] <= upper[0] &&
        lod[1] >= lower[1] && lod[1] <= upper[1] &&
        lod[2] >= lower[2] && lod[2] <= upper[2]
    }
}