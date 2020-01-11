use arr_macro::arr;
use vek::*;

fn calc_idx(v: Vec2<i32>) -> usize {
    let mut x = v.x as u32;
    let mut y = v.y as u32;
    x = x.wrapping_mul(0x6eed0e9d);
    y = y.wrapping_mul(0x2f72b421);
    (x ^ y) as usize
}

const CACHE_LEN: usize = /*512*//*128*/32;

pub struct SmallCache<V: Default> {
    index: [Option<Vec2<i32>>; CACHE_LEN + 9],
    data: [V; CACHE_LEN + 9],
    random: u32,
}
impl<V: Default> Default for SmallCache<V> {
    fn default() -> Self {
        Self {
            index: [None; CACHE_LEN + 9],
            data: arr![V::default(); /*521*//*137*/41], // TODO: Use CACHE_LEN
            random: 1,
        }
    }
}
impl<V: Default> SmallCache<V> {
    pub fn get<F: FnOnce(Vec2<i32>) -> V>(&mut self, key: Vec2<i32>, f: F) -> &V {
        let idx = calc_idx(key) % CACHE_LEN;

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
