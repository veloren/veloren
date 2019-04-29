#[derive(Debug, Clone, Copy)]
pub enum ServerMsg {
    Attach(),
}

pub type RegionIdSize = i8;
pub type RegionId = (/*x*/RegionIdSize, /*y*/RegionIdSize /*z = 0*/);

pub const RegionMIN:i8 = -64;
pub const RegionMAX:i8 = 63;