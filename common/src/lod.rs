use crate::{terrain::TerrainChunkSize, vol::RectVolSize};
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use vek::*;

// In chunks
pub const ZONE_SIZE: u32 = 32;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct Flags: u8 {
        const SNOW_COVERED  = 0b00000001;
        const IS_BUILDING   = 0b00000010;
        const IS_GIANT_TREE = 0b00000100;
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug, Serialize, Deserialize, EnumIter)]
#[repr(u16)]
pub enum ObjectKind {
    Oak,
    Pine,
    Dead,
    House,
    GiantTree,
    MapleTree,
    Cherry,
    AutumnTree,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Object {
    pub kind: ObjectKind,
    pub pos: Vec3<i16>,
    pub flags: Flags,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    pub objects: Vec<Object>,
}

pub fn to_wpos(wpos: i32) -> i32 { wpos * (TerrainChunkSize::RECT_SIZE.x * ZONE_SIZE) as i32 }

pub fn from_wpos(zone_pos: i32) -> i32 {
    zone_pos.div_euclid((TerrainChunkSize::RECT_SIZE.x * ZONE_SIZE) as i32)
}
