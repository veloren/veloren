use hashbrown::HashMap;
use vek::Vec2;

use super::{Sampler, StructureGen2d};

pub struct StructureGenCache<T> {
    gen: StructureGen2d,
    // TODO: Compare performance of using binary search instead of hashmap
    cache: HashMap<Vec2<i32>, Option<T>>,
}

impl<T> StructureGenCache<T> {
    pub fn new(gen: StructureGen2d) -> Self {
        Self {
            gen,
            cache: HashMap::new(),
        }
    }

    pub fn get(
        &mut self,
        index: Vec2<i32>,
        mut generate: impl FnMut(Vec2<i32>, u32) -> Option<T>,
    ) -> Vec<&T> {
        let close = self.gen.get(index);
        for (wpos, seed) in close {
            self.cache
                .entry(wpos)
                .or_insert_with(|| generate(wpos, seed));
        }

        close
            .iter()
            .filter_map(|(wpos, _)| self.cache.get(wpos).unwrap().as_ref())
            .collect()
    }

    pub fn generated(&self) -> impl Iterator<Item = &T> {
        self.cache.values().filter_map(|v| v.as_ref())
    }
}
