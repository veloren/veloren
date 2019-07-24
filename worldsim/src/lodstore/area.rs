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
}