// Standard
use std::thread;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::marker::PhantomData;

// Library
use coord::prelude::*;

// Local
use {Volume, Voxel};

pub enum VolState<V: Volume> {
    Loading,
    Exists(V),
}

pub struct VolGen<V: Volume> {
    func: Arc<fn(Vec2<i64>) -> V>,
}

impl <V: Volume> VolGen<V> {
    pub fn new(f: fn(Vec2<i64>) -> V) -> VolGen<V> {
        VolGen {
            func: Arc::new(f),
        }
    }
}

// pub struct VolGen<V: 'static + Volume, F: Fn(Vec2<i64>) -> V>
//     where F: 'static + Send + Sync + Sized + Fn(Vec2<i64>, Arc<RwLock<VolState<V>>>) -> Arc<RwLock<VolState<V>>>
// {
//     gen_func: Arc<F>,
//     _marker: PhantomData<V>,
// }

// impl<V: 'static + Volume, F> VolGen<V, F>
//     where F: 'static + Send + Sync + Sized + Fn(Vec2<i64>, Arc<RwLock<VolState<V>>>) -> Arc<RwLock<VolState<V>>>
// {
//     pub fn new(f: F) -> VolGen<V, F> {
//         VolGen {
//             gen_func: Arc::new(f),
//             _marker: PhantomData,
//         }
//     }

//     fn generate(&self, pos: Vec2<i64>, vol: Arc<RwLock<VolState<V>>>) {
//         let vol_clone = vol.clone();
//         let gen_func_clone = self.gen_func.clone();
//         *vol.write().unwrap() = VolState::Loading(
//             thread::spawn(move || { let vol = vol_clone; (*gen_func_clone)(pos, vol) })
//         );
//     }
// }

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

    pub fn gen(&self, pos: Vec2<i64>) {
        let gen = self.gen.func.clone();
        let vol = Arc::new(RwLock::new(VolState::Loading));
        self.vols.write().unwrap().insert(pos, vol.clone());
        thread::spawn(move || {
            *vol.write().unwrap() = VolState::Exists(gen(pos));
        });
    }

    pub fn set(&self, pos: Vec2<i64>, vol: V) {
        self.vols.write().unwrap().insert(pos, Arc::new(RwLock::new(VolState::Exists(vol))));
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
                VolState::Exists(ref v) => v
                    .at(vox_pos)
                    .unwrap_or(V::VoxelType::empty()),
                }
            )
            .unwrap_or(V::VoxelType::empty())
    }
}
