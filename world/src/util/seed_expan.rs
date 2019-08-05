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

fn sbox(x: u64) -> u64 {
    let mut bytes = x.to_ne_bytes();
    for byte in &mut bytes {
        *byte = SBOX[*byte as usize];
    }
    u64::from_ne_bytes(bytes)
}

// chaotic bijective function
fn diffuse_rnd(mut x: u64) -> u64 {
    x = x.wrapping_mul(0x6eed0e9da4d94a4f);
    let a = x >> 32;
    let b = x >> 60;
    x ^= a >> b;
    x = x.wrapping_mul(0x6eed0e9da4d94a4f);
    sbox(x)
}

fn diffuse(mut x: u64) -> u64 {
    for _ in 0..4 {
        x = diffuse_rnd(x);
    }
    x
}

fn initial_expand(x: u32) -> u64 {
    (x as u64).wrapping_mul(0x2f72b4215a3d8caf).pow(2)
}

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

pub fn diffused_field(seed: u32, amount: u32) -> Vec<u32> {
    let mut field = Vec::new();
    for i in 0..=amount {
        let n = truncate(diffuse(initial_expand(seed + i)));
        field.push(n);
    }
    field
}

pub fn expand_seed_to_rng(seed: u32) -> [u8; 32] {
    let mut r: [u64; 4] = [0; 4];
    let mut state: u64 = initial_expand(seed);

    for i in 0..4 {
        state = diffuse(state);
        r[i] = state;
    }

    unsafe { std::mem::transmute(r) }
}
