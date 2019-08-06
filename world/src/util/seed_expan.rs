/// Simple non-cryptographic diffusion function.
pub fn diffuse(mut x: u32) -> u32 {
    x ^= 2281701376;
    x = x.rotate_left(7);
    x.wrapping_mul(0x811c9dc5)
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
