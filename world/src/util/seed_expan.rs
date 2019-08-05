// This module contains a few functions and utilities for expanding a seed into more data for use in worldgen.

// Very standard substitution box. Takes one number and gives back another. This one works per byte.
// Standard component in diffusion functions. The numbers here are totally random and could be whatever.
// Onlu rule is each index has to match an unique number.
static SBOX: [u8; 256] = [
    206, 21, 212, 69, 54, 234, 13, 42, 184, 48, 92, 64, 196, 55, 225, 235, 229, 120, 135, 72, 32,
    147, 74, 142, 197, 79, 139, 164, 110, 57, 176, 47, 192, 174, 178, 49, 193, 71, 78, 18, 237, 81,
    255, 187, 5, 246, 247, 109, 26, 44, 93, 230, 96, 102, 204, 31, 100, 175, 182, 245, 9, 0, 127,
    161, 125, 52, 129, 179, 209, 130, 219, 77, 218, 252, 61, 75, 62, 248, 124, 220, 98, 87, 63,
    163, 101, 40, 29, 4, 36, 123, 23, 238, 134, 35, 17, 169, 226, 19, 2, 253, 158, 172, 37, 104,
    183, 194, 43, 167, 59, 215, 162, 88, 140, 25, 133, 221, 132, 159, 232, 250, 154, 10, 211, 112,
    146, 189, 141, 95, 1, 111, 28, 160, 73, 181, 67, 30, 190, 157, 148, 149, 11, 8, 41, 217, 106,
    39, 214, 152, 180, 168, 14, 56, 70, 34, 137, 243, 45, 195, 94, 38, 7, 116, 16, 136, 82, 114,
    186, 105, 15, 223, 200, 131, 85, 20, 128, 210, 97, 233, 151, 241, 138, 6, 60, 24, 249, 12, 207,
    239, 171, 65, 113, 115, 22, 107, 68, 143, 90, 119, 185, 153, 166, 46, 155, 191, 254, 58, 150,
    251, 99, 213, 118, 240, 122, 108, 231, 126, 177, 80, 227, 91, 145, 203, 228, 198, 236, 53, 50,
    51, 76, 242, 103, 117, 170, 173, 121, 188, 27, 244, 205, 224, 144, 3, 89, 84, 66, 202, 83, 156,
    216, 33, 165, 86, 222, 199, 208, 201,
];

// Helper function to work with the box above. Takes a u64 and runs each byte through the box.
fn sbox(x: u64) -> u64 {
    let mut bytes = x.to_ne_bytes();
    for byte in &mut bytes {
        *byte = SBOX[*byte as usize];
    }
    u64::from_ne_bytes(bytes)
}

// A bijective diffusion function with chaotic behaviour. It essentially mixes numbers.
// A 1 bit change somewhere will affect a large portion of the other bits after running through this function.
fn diffuse_rnd(mut x: u64) -> u64 {
    x = x.wrapping_mul(0x6eed0e9da4d94a4f);
    let a = x >> 32;
    let b = x >> 60;
    x ^= a >> b;
    x = x.wrapping_mul(0x6eed0e9da4d94a4f);
    sbox(x)
}

// Helper for running diffuse_rnd 4 times, enough to mix the bits around properly.
fn diffuse(mut x: u64) -> u64 {
    for _ in 0..4 {
        x = diffuse_rnd(x);
    }
    x
}

// Expands a 32 bit state into a 64 bit state.
fn initial_expand(x: u32) -> u64 {
    let f = (x as u64).wrapping_mul(0x2f72b4215a3d8caf);
    f.wrapping_mul(f)
}

// Truncate a 64 bit state to a 32 bit seed.
fn truncate(x: u64) -> u32 {
    let xb = x.to_ne_bytes();
    let nb = [
        xb[0].wrapping_mul(xb[1]),
        xb[2].wrapping_mul(xb[3]),
        xb[4].wrapping_mul(xb[5]),
        xb[6].wrapping_mul(xb[7]),
    ];
    u32::from_ne_bytes(nb)
}

// Generates a sequence of diffused (mixed) numbers from one seed.
// Used for generating a lot of initial seeds for the noise algorithms in worldgen.
pub fn diffused_field(seed: u32, amount: u32) -> Vec<u32> {
    let mut field = Vec::new();
    for i in 0..=amount {
        let n = truncate(diffuse(initial_expand(seed + i)));
        field.push(n);
    }
    field
}

// Expand a 32 bit seed into a 32 byte state for the RNG used in worldgen.
pub fn expand_seed_to_rng(seed: u32) -> [u8; 32] {
    // Create a new empty ChaChaRng state.
    let mut r: [u64; 4] = [0; 4];

    // Create a new empty internal state.
    let mut state: u64 = initial_expand(seed);

    // Fill the ChaChaRng state with random bits from repeatedly mixing the state.
    for i in 0..4 {
        state = diffuse(state);
        r[i] = state;
    }

    // Convert the ChaChaRng state to bytes. Uses unsafe here because the safe code for it would be much longer.
    unsafe { std::mem::transmute(r) }
}
