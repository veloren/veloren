/// Simple non-cryptographic diffusion function.
pub fn diffuse(mut x: u32) -> u32 {
    x = x.wrapping_add(0x7ed55d16).wrapping_add(x << 12);
    x = (x ^ 0xc761c23c) ^ (x >> 19);
    x = x.wrapping_add(0x165667b1).wrapping_add(x << 5);
    x = x.wrapping_add(0xd3a2646c) ^ (x << 9);
    x = x.wrapping_add(0xfd7046c5).wrapping_add(x << 3);
    x = (x ^ 0xb55a4f09) ^ (x >> 16);
    x
}

/// Expand a 32 bit seed into a 32 byte RNG state.
pub fn rng_state(mut x: u32) -> [u8; 32] {
    let mut r: [u32; 8] = [0; 8];
    for s in &mut r {
        x = diffuse(x);
        *s = x;
    }
    unsafe { std::mem::transmute(r) }
}
