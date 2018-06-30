// Standard
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

// Library
use coord::prelude::*;

// Local
use {Volume, Voxel};

pub struct VolumeMgr<V: Volume> {
    vol_size: i64,
    vols: RwLock<HashMap<Vec2<i64>, Arc<V>>>,
}

impl<V: Volume> VolumeMgr<V> {
    pub fn new(vol_size: i64) -> VolumeMgr<V> {
        VolumeMgr {
            vol_size,
            vols: RwLock::new(HashMap::new()),
        }
    }

    pub fn at(&self, pos: Vec2<i64>) -> Option<Arc<V>> {
        self.vols.read().unwrap().get(&pos).map(|v| v.clone())
    }

    pub fn contains(&self, pos: Vec2<i64>) -> bool {
        self.vols.read().unwrap().contains_key(&pos)
    }

    pub fn set(&self, pos: Vec2<i64>, vol: V) {
        self.vols.write().unwrap().insert(pos, Arc::new(vol));
    }

    pub fn get_voxel(&self, pos: Vec3<i64>) -> V::VoxelType {
        let vol_pos = vec2!(
            pos.x.div_euc(self.vol_size),
            pos.y.div_euc(self.vol_size)
        );

        let vox_pos = vec3!(
            pos.x.mod_euc(self.vol_size),
            pos.y.mod_euc(self.vol_size),
            pos.z
        );

        self.vols.read().unwrap()
            .get(&vol_pos)
            .map(|v| v
                .at(vox_pos)
                .unwrap_or(V::VoxelType::empty())
            )
            .unwrap_or(V::VoxelType::empty())
    }
}
