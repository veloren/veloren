// Standard
use std::thread;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::marker::PhantomData;

// Library
use coord::prelude::*;

// Local
use {Volume, Voxel};

pub trait VolPayload: Send + Sync {}

pub enum VolState<V: Volume> {
    Loading,
    Exists(V, Box<VolPayload>),
}

pub struct VolGen<V: Volume> {
    gen_func: Arc<fn(Vec2<i64>) -> V>,
    payload_func: Arc<fn(&V) -> Box<VolPayload>>,
}

impl <V: Volume> VolGen<V> {
    pub fn new(gen_func: fn(Vec2<i64>) -> V, payload_func: fn(&V) -> Box<VolPayload>) -> VolGen<V> {
        VolGen {
            gen_func: Arc::new(gen_func),
            payload_func: Arc::new(payload_func),
        }
    }
}

pub struct VolMgr<V: 'static + Volume> {
    vol_size: i64,
    vols: RwLock<HashMap<Vec2<i64>, Arc<RwLock<VolState<V>>>>>,
    gen: VolGen<V>,
}

impl<V: 'static + Volume> VolMgr<V> {
    pub fn new(vol_size: i64, gen: VolGen<V>) -> VolMgr<V> {
        VolMgr {
            vol_size,
            vols: RwLock::new(HashMap::new()),
            gen,
        }
    }

    pub fn at(&self, pos: Vec2<i64>) -> Option<Arc<RwLock<VolState<V>>>> {
        self.vols.read().unwrap().get(&pos).map(|v| v.clone())
    }

    pub fn contains(&self, pos: Vec2<i64>) -> bool {
        self.vols.read().unwrap().contains_key(&pos)
    }

    pub fn remove(&self, pos: Vec2<i64>) -> bool {
        self.vols.write().unwrap().remove(&pos).is_some()
    }

    pub fn gen(&self, pos: Vec2<i64>) {
        if self.contains(pos) {
            return; // Don't try to generate the same chunk twice
        }

        let gen_func = self.gen.gen_func.clone();
        let payload_func = self.gen.payload_func.clone();
        let vol_state = Arc::new(RwLock::new(VolState::Loading));
        self.vols.write().unwrap().insert(pos, vol_state.clone());
        thread::spawn(move || {
            let vol = gen_func(pos);
            let payload = payload_func(&vol);
            *vol_state.write().unwrap() = VolState::Exists(vol, payload);
        });
    }

    pub fn set(&self, pos: Vec2<i64>, vol: V, payload: Box<VolPayload>) {
        self.vols.write().unwrap().insert(pos, Arc::new(RwLock::new(VolState::Exists(vol, payload))));
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
            .map(|v| match *v.read().unwrap() {
                VolState::Loading => V::VoxelType::empty(),
                VolState::Exists(ref v, _) => v
                    .at(vox_pos)
                    .unwrap_or(V::VoxelType::empty()),
                }
            )
            .unwrap_or(V::VoxelType::empty())
    }
}
