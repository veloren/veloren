/// The zerocopy crate exists and can replace this function.
/// We should evaluate using it when we have multiple usage spots for it.
/// For now we have this safe alternative.
fn cast_u32x8_u8x32(a: [u32; 8]) -> [u8; 32] {
    let mut r = [0; 32];
    for i in 0..8 {
        let a = a[i].to_ne_bytes();
        for j in 0..4 {
            r[i * 4 + j] = a[j];
        }
    }
    r
}

/// Simple non-cryptographic diffusion function.
pub fn diffuse(mut x: u32) -> u32 {
    x = x.wrapping_add(0x7ed55d16).wrapping_add(x << 12);
    x = (x ^ 0xc761c23c) ^ (x >> 19);
    x = x.wrapping_add(0x165667b1).wrapping_add(x << 5);
    x = x.wrapping_add(0xd3a2646c) ^ (x << 9);
    x = x.wrapping_add(0xfd7046c5).wrapping_add(x << 3);
    x = (x ^ 0xb55a4f09) ^ (x >> 16);
    x = x.wrapping_add((1 << 31) - 1) ^ (x << 13);
    x
}

/// Expand a 32 bit seed into a 32 byte RNG state.
pub fn rng_state(mut x: u32) -> [u8; 32] {
    let mut r: [u32; 8] = [0; 8];
    for s in &mut r {
        x = diffuse(x);
        *s = x;
    }
    cast_u32x8_u8x32(r)
}
