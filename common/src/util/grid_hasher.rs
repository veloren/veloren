use std::hash::{BuildHasher, Hasher};

#[derive(Copy, Clone, Default)]
pub struct GridHasher(u64);

// It's extremely unlikely that the spatial grid can be used to viably DOS the
// server given that clients only have control over their player and a handful
// of entities in their near vicinity. For this reason, we just use an xor hash,
// which should keep collisions relatively low since the spatial coherence of
// the grid is distributed fairly evenly with the output of the hash function.
impl Hasher for GridHasher {
    fn finish(&self) -> u64 { self.0 }

    fn write(&mut self, _: &[u8]) {
        panic!("Hashing arbitrary bytes is unimplemented");
    }

    fn write_i32(&mut self, x: i32) { self.0 = self.0.wrapping_mul(113989) ^ self.0 ^ x as u64; }
}

impl BuildHasher for GridHasher {
    type Hasher = Self;

    fn build_hasher(&self) -> Self::Hasher { *self }
}
