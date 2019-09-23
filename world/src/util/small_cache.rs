use arr_macro::arr;
use vek::*;

fn calc_idx(v: Vec2<i32>) -> usize {
    let mut x = v.x as u32;
    let mut y = v.y as u32;
    x = x.wrapping_mul(0x6eed0e9d);
    y = y.wrapping_mul(0x2f72b421);
    (x ^ y) as usize
}

pub struct SmallCache<V: Default> {
    index: [Option<Vec2<i32>>; 137],
    data: [V; 137],
    random: u32,
}
impl<V: Default> Default for SmallCache<V> {
    fn default() -> Self {
        Self {
            index: arr![None; 137],
            data: arr![V::default(); 137],
            random: 1,
        }
    }
}
impl<V: Default> SmallCache<V> {
    pub fn get<F: FnOnce(Vec2<i32>) -> V>(&mut self, key: Vec2<i32>, f: F) -> &V {
        let idx = calc_idx(key) % 128;

        // Search
        if self.index[idx].as_ref().map(|k| k == &key).unwrap_or(false) {
            return &self.data[idx];
        } else if self.index[idx + 1]
            .as_ref()
            .map(|k| k == &key)
            .unwrap_or(false)
        {
            return &self.data[idx + 1];
        } else if self.index[idx + 4]
            .as_ref()
            .map(|k| k == &key)
            .unwrap_or(false)
        {
            return &self.data[idx + 4];
        } else if self.index[idx + 9]
            .as_ref()
            .map(|k| k == &key)
            .unwrap_or(false)
        {
            return &self.data[idx + 9];
        }
        // Not present so insert
        for i in 0..4 {
            let idx = idx + i * i;
            if self.index[idx].is_none() {
                self.index[idx] = Some(key.clone());
                self.data[idx] = f(key);
                return &self.data[idx];
            }
        }
        // No space randomly remove someone
        let step = super::seed_expan::diffuse(self.random) as usize % 4;
        let idx = step * step + idx;
        self.random = self.random.wrapping_add(1);
        self.index[idx] = Some(key.clone());
        self.data[idx] = f(key);
        &self.data[idx]
    }
}

/*pub struct SmallCache<V: Default> {
    index: [Option<Vec2<i32>>; 128],
    data: [V; 128],
    random: u32,
}
impl<V: Default> Default for SmallCache<V> {
    fn default() -> Self {
        Self {
            index: arr![None; 128],
            data: arr![V::default(); 128],
            random: 1,
        }
    }
}
impl<V: Default> SmallCache<V> {
    pub fn get<F: FnOnce(Vec2<i32>) -> V>(&mut self, key: Vec2<i32>, f: F) -> &V {
        let idx = calc_idx(key) % 32 * 4;

        // Search
        if self.index[idx].as_ref().map(|k| k == &key).unwrap_or(false) {
            return &self.data[idx];
        } else if self.index[idx + 1]
            .as_ref()
            .map(|k| k == &key)
            .unwrap_or(false)
        {
            return &self.data[idx + 1];
        } else if self.index[idx + 2]
            .as_ref()
            .map(|k| k == &key)
            .unwrap_or(false)
        {
            return &self.data[idx + 2];
        } else if self.index[idx + 3]
            .as_ref()
            .map(|k| k == &key)
            .unwrap_or(false)
        {
            return &self.data[idx + 3];
        }
        // Not present so insert
        for i in 0..4 {
            let idx = idx + i;
            if self.index[idx].is_none() {
                self.index[idx] = Some(key.clone());
                self.data[idx] = f(key);
                return &self.data[idx];
            }
        }
        let idx = idx + super::seed_expan::diffuse(self.random) as usize % 4;
        self.random = self.random.wrapping_add(1);
        self.index[idx] = Some(key.clone());
        self.data[idx] = f(key);
        &self.data[idx]
    }
}*/

/*const ZCACHE_SIZE: usize = 32;

#[derive(Default)]
pub struct ZestSmallCache<K: Eq + Clone, V: Default> {
    index: [Option<K>; ZCACHE_SIZE],
    cache: [V; ZCACHE_SIZE],
    not_cached: V,
}

impl<K: Eq + Clone, V: Default> ZestSmallCache<K, V> {
    pub fn get<F: FnOnce(K) -> V>(&mut self, key: K, cache: bool, f: F) -> &V {
        for i in 0..ZCACHE_SIZE {
            if self.index[i].as_ref().map(|k| k == &key).unwrap_or(false) {
                return &self.cache[i];
            }
        }
        if !cache {
            self.not_cached = f(key);
            return &self.not_cached;
        }
        for i in 0..ZCACHE_SIZE {
            if self.index[i].is_none() {
                self.index[i] = Some(key.clone());
                self.cache[i] = f(key.clone());
                return &self.cache[i];
            }
        }
        self.index[0] = Some(key.clone());
        self.cache[0] = f(key);
        &self.cache[0]
    }
}*/
