use vek::*;

/// Converts `(x, 0, 0)` to its index in the morton order. Arithmetically that
/// is, the bits of `x` are interleaved by two zero bits between every adjacent
/// pair of bits. Precondition is `0 <= x && x < 2048`.
///
/// Eidetic:
///
/// ```ignore
/// x00_to_morton(0bKJIHGFEDCBA) == 0bK00J00I00H00G00F00E00D00C00B00A
/// ```
#[inline(always)]
pub fn x00_to_morton(x: i32) -> u32 {
    // Because the result is u32, we support only inputs of up to 11 bits:
    assert!(0 <= x && x < 2048); // 2048 = 2^11
    let mut m = x as u32;
    m = (m | (m << 12)) & 0x7C003F;
    m = (m | (m << 4) | (m << 8)) & 0x430C30C3;
    m = (m | (m << 2)) & 0x49249249;
    m
}

/// Converts (pos.x, pos.y, pos.z) to its index in the morton order.
/// Arithmetically that is, the bits of `pos.x`, `pos.y` and `pos.z` are
/// interleaved such that the least (resp. 2nd to least, resp. 3rd to least)
/// significant bit and every third bit from thereon originates from `pos.x`
/// (resp. `pos.y`, resp. `pos.z`). Precondition is for all `c` in `pos`:
/// `0 <= c && c < 2048`.
///
/// Eidetic:
///
/// ```ignore
/// xyz_to_morton(Vec3<i32>::new(
///     0bKJIHGFEDCBA,
///     0b00000000000,
///     0b11111111111
/// )) == 0bK10J10I10H10G10F10E10D10C10B10A
/// ```
#[inline(always)]
pub fn xyz_to_morton(pos: Vec3<i32>) -> u32 {
    let x = x00_to_morton(pos.x);
    let y = x00_to_morton(pos.y) << 1;
    let z = x00_to_morton(pos.z) << 2;
    x | y | z
}

pub fn morton_to_x(morton: u32) -> i32 {
    let mut m = ((morton & 0x08208208) >> 2) | (morton & 0x41041041);
    m = ((m & 0x40003000) >> 8) | ((m & 0x30000C0) >> 4) | (m & 0xC0003);
    m = ((m & 0xFC0000) >> 12) | (m & 0x3F);
    m as i32
}

pub fn morton_to_y(morton: u32) -> i32 {
    morton_to_x(morton >> 1)
}

pub fn morton_to_z(morton: u32) -> i32 {
    morton_to_x(morton >> 2)
}

pub fn morton_to_xyz(morton: u32) -> Vec3<i32> {
    Vec3::<i32>::new(
        morton_to_x(morton),
        morton_to_y(morton),
        morton_to_z(morton),
    )
}

pub struct MortonIter {
    current: u32,
    begin: u32,
    end: u32,
}

impl MortonIter {
    pub fn new(lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self {
        assert!(lower_bound.map(|l| l >= 0).reduce_and());
        assert!(upper_bound.map(|u| u >= 0).reduce_and());
        if lower_bound.map2(upper_bound, |l, u| l < u).reduce_and() {
            let begin = xyz_to_morton(lower_bound);
            // The implementation treats `end` as inclusive.
            let end = xyz_to_morton(upper_bound - Vec3::one());
            Self {
                current: begin,
                begin,
                end,
            }
        } else {
            // The implementation doesn't work with empty ranges.
            // Therefore we have this special case.
            Self {
                current: 1,
                begin: 0,
                end: 0,
            }
        }
    }
}

impl Iterator for MortonIter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        fn smear3(mut a: u32) -> u32 {
            a |= (a >> 12) | (a >> 24); // Doing this in one assignment removes one data dependency.
            a |= a >> 6;
            a |= a >> 3;
            a
        }

        if self.current > self.end {
            return None;
        }

        loop {
            let lt_begin = !self.current & self.begin;
            let gt_begin = self.current & !self.begin;
            let lt_end = !self.current & self.end;
            let gt_end = self.current & !self.end;
            let defects = (!smear3(gt_begin) & lt_begin) | (!smear3(lt_end) & gt_end);
            if defects == 0 {
                break;
            }
            let mask = std::i32::MAX as u32 >> defects.leading_zeros();
            self.current = (self.current & !mask) + (defects & !mask);
        }

        let c = self.current;
        self.current += 1;
        Some(c)
    }
}
