// Standard
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

// Library
use coord::prelude::*;

// Local
use Volume;

fn floor_div(a: i32, b: i32) -> i32 {
    (if a < 0 { a - (b - 1) } else { a }) / b
}

pub struct VolumeMgr<V: Volume> {
    vols: RwLock<HashMap<Vec2i, Arc<V>>>,
}

impl<V: Volume> VolumeMgr<V> {
    pub fn new() -> VolumeMgr<V> {
        VolumeMgr {
            vols: RwLock::new(HashMap::new()),
        }
    }

    pub fn at(&self, pos: Vec2i) -> Option<Arc<V>> {
        self.vols.read().unwrap().get(&pos).map(|v| v.clone())
    }

    pub fn contains(&self, pos: Vec2i) -> bool {
        self.vols.read().unwrap().contains_key(&pos)
    }

    pub fn set(&self, pos: Vec2i, vol: V) {
        self.vols.write().unwrap().insert(pos, Arc::new(vol));
    }
}
