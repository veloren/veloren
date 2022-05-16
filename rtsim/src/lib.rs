pub mod data;
pub mod gen;

use self::data::Data;
use std::sync::Arc;
use world::World;

pub struct RtState {
    pub data: Data,
}
