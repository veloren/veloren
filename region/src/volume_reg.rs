// Standard
use std::collections::HashMap;

// Library
use coord::prelude::*;

// Local
use Volume;

pub struct VolumeReg<T> {
    chunks: HashMap<Vec2i, T>,
}

impl<T> VolumeReg<T> {
    pub fn new() -> VolumeReg<T> {
        VolumeReg {
            chunks: HashMap::new(),
        }
    }

    pub fn contains(&self, pos: Vec2i) -> bool {
        self.chunks.contains_key(&pos)
    }
}
