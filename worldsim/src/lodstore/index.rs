use vek::*;
use std::ops::Sub;
use std::ops::Add;

/*
For our LodStructures we need a type that covers the values from 0 - 2047 in steps of 1/32.
which is 11 bits for the digits before the decimal point and 5 bits for the digits after the decimal point.
Because for accessing the decimal point makes no difference we use a u16 to represent this value.
The value needs to be shiftet to get it's "real inworld size",

Edit: now it actually implements a value from 0 - 3*2048 - 1/32, covering over 3 regions for accessing neighbor region values

-- lower neighbor
0 -> 0
1 -> 2047 31/32
-- owned
65536 -> 2048
131071 -> 4095 31/32
-- upper neighbor
196607 -> 6143 31/32

*/

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct LodIndex {
    /*
        bit 0..17 -> x
        bit 18..35 -> y
        bit 36..53 -> z
        bit 54..63 -> unused
    */
    data: u64,
}

/*does not work on big endian!*/
const BIT_X_MASK: u64 = 0b0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0011_1111_1111_1111_1111;
const BIT_Y_MASK: u64 = 0b0000_0000_0000_0000_0000_0000_0000_1111_1111_1111_1111_1100_0000_0000_0000_0000;
const BIT_Z_MASK: u64 = 0b0000_0000_0011_1111_1111_1111_1111_0000_0000_0000_0000_0000_0000_0000_0000_0000;
const BIT_X_MASK32: u32 = 0b0000_0000_0000_0011_1111_1111_1111_1111;

impl LodIndex {
    pub fn new(data: Vec3<u32>) -> Self {
        let mut index = LodIndex {data: 0};
        index.set(data);
        index
    }

    pub fn get(&self) -> Vec3<u32>  {
        let x = (self.data & BIT_X_MASK) as u32;
        let y = ((self.data & BIT_Y_MASK) >> 18 ) as u32;
        let z = ((self.data & BIT_Z_MASK) >> 36 ) as u32;
        Vec3{x,y,z}
    }

    pub fn set(&mut self, data: Vec3<u32>) {
        let x = (data.x & BIT_X_MASK32) as u64;
        let y = ((data.y & BIT_X_MASK32) as u64 ) << 18;
        let z = ((data.z & BIT_X_MASK32) as u64 ) << 36;
        self.data = x + y + z;
    }

    pub fn align_to_layer_id(&self, level: u8) -> LodIndex {
        let xyz = self.get();
        let f = two_pow_u(level) as u32;
        LodIndex::new(xyz.map(|i| {
            (i / f) * f
        }))
    }
}


#[cfg(test)]
mod tests {
    use crate::{
        lodstore::index::LodIndex,
    };
    use vek::*;

    #[test]
    fn setter_getter() {
        let i = LodIndex::new(Vec3::new(0,0,0));
        assert_eq!(i.get(), Vec3::new(0,0,0));

        let i = LodIndex::new(Vec3::new(1337,0,0));
        assert_eq!(i.get(), Vec3::new(1337,0,0));

        let i = LodIndex::new(Vec3::new(0,1337,0));
        assert_eq!(i.get(), Vec3::new(0,1337,0));

        let i = LodIndex::new(Vec3::new(0,0,1337));
        assert_eq!(i.get(), Vec3::new(0,0,1337));

        let i = LodIndex::new(Vec3::new(1,1,1));
        assert_eq!(i.get(), Vec3::new(1,1,1));

        let i = LodIndex::new(Vec3::new(262143,262143,262143));
        assert_eq!(i.get(), Vec3::new(262143,262143,262143));

        let i = LodIndex::new(Vec3::new(262144,262144,262144)); //overflow
        assert_eq!(i.get(), Vec3::new(0,0,0));

        let i = LodIndex::new(Vec3::new(42,1337,69));
        assert_eq!(i.get(), Vec3::new(42,1337,69));
    }
}

impl Sub for LodIndex {
    type Output = LodIndex;
    fn sub(self, rhs: LodIndex) -> Self::Output {
        LodIndex {
            data: self.data - rhs.data /*fast but has overflow issues*/
        }
    }
}

impl Add for LodIndex {
    type Output = LodIndex;
    fn add(self, rhs: LodIndex) -> Self::Output {
        LodIndex {
            data: self.data + rhs.data /*fast but has overflow issues*/
        }
    }
}


/*
impl LodIndex {
    pub fn new(pos: Vec3<i32>) -> Self {
        Self {
            data: pos.map(|x| (x * 32 + 65535) as u32),
        }
    }

    pub fn newf(pos: Vec3<f32>) -> Self {
        Self {
            data: pos.map(|x| (x * 32.0).round() as u32 + 65535),
        }
    }

    pub fn to_pos_i(&self) -> Vec3<i32> { self.data.map(|x| (x / 32 - 2048) as i32) }

    pub fn to_pos_f(&self) -> Vec3<f32> {
        self.data.map(|x| x as f32 / 32.0 - 2048.0)
    }
}

pub const LEVEL_LENGTH_POW_MAX: i8 = 11;
pub const LEVEL_LENGTH_POW_MIN: i8 = -4;

pub const LEVEL_INDEX_POW_MAX: u8 = 15;
pub const LEVEL_INDEX_POW_MIN: u8 = 0;

pub const fn length_to_index(n: i8) -> u8 { (n+4) as u8 }

pub const fn two_pow_u(n: u8) -> u16 {
    1 << n
}

pub fn two_pow_i(n: i8) -> f32 {
    2.0_f32.powi(n as i32)
}

*/

pub const fn two_pow_u(n: u8) -> u16 {
    1 << n
}

pub fn relative_to_1d(index: LodIndex, relative_size: Vec3<u32>) -> usize {
    let index = index.get();
    (index[0] + index[1] * relative_size[0] + index[2] * relative_size[0] * relative_size[1]) as usize
}
