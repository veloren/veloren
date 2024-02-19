use arr_macro::arr;

fn calc_idx(v: impl Iterator<Item = i32>) -> usize {
    let mut r = 0;
    for (e, h) in v.zip([0x6eed0e9d, 0x2f72b421, 0x18132f72, 0x891e2fba].into_iter()) {
        r ^= (e as u32).wrapping_mul(h);
    }
    r as usize
}

// NOTE: Use 128 if TerrainChunkSize::RECT_SIZE.x = 128.
const CACHE_LEN: usize = 32;

pub struct SmallCache<K, V: Default> {
    index: [Option<K>; CACHE_LEN + 9],
    data: [V; CACHE_LEN + 9],
    random: u32,
}
impl<K: Copy, V: Default> Default for SmallCache<K, V> {
    fn default() -> Self {
        Self {
            index: [None; CACHE_LEN + 9],
            data: arr![V::default(); 41], // TODO: Use CACHE_LEN
            random: 1,
        }
    }
}
impl<K: Copy + Eq + IntoIterator<Item = i32>, V: Default> SmallCache<K, V> {
    pub fn get<F: FnOnce(K) -> V>(&mut self, key: K, f: F) -> &V {
        let idx = calc_idx(key.into_iter()) % CACHE_LEN;

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
                self.index[idx] = Some(key);
                self.data[idx] = f(key);
                return &self.data[idx];
            }
        }
        // No space randomly remove someone
        let step = super::seed_expan::diffuse(self.random) as usize % 4;
        let idx = step * step + idx;
        self.random = self.random.wrapping_add(1);
        self.index[idx] = Some(key);
        self.data[idx] = f(key);
        &self.data[idx]
    }
}
