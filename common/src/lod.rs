use vek::*;
use serde::{Serialize, Deserialize};
use strum::EnumIter;
use crate::{
    terrain::TerrainChunkSize,
    vol::RectVolSize,
};

// In chunks
pub const ZONE_SIZE: u32 = 64;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug, Serialize, Deserialize, EnumIter)]
#[repr(u16)]
pub enum ObjectKind {
    Tree,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Object {
    pub kind: ObjectKind,
    pub pos: Vec3<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    pub objects: Vec<Object>,
}

pub fn to_wpos(wpos: i32) -> i32 {
    wpos * (TerrainChunkSize::RECT_SIZE.x * ZONE_SIZE) as i32
}

pub fn from_wpos(zone_pos: i32) -> i32 {
    zone_pos / (TerrainChunkSize::RECT_SIZE.x * ZONE_SIZE) as i32
}
