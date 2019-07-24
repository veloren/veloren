use vek::*;
use std::ops::Sub;
use std::ops::Add;
use std::cmp;
use std::fmt;

/*
A region owns the Values from in (0, 2048) in steps of 1/32.
But because regions can also subscribe we add support to the range (0, 2048*3).
which is 13 bits for the digits before the decimal point and 5 bits for the digits after the decimal point.
We use our own LodIndex type to store and compute based on these values, because u16 arithmetic (inside the owned area) is super easy to archive and allows us to optimize a lot.


-- lower neighbor
0 -> 0
65535 -> 2047 31/32
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

//TODO: Optimize!
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

    pub fn align_to_layer_id(&self, layer: u8) -> LodIndex {
        let xyz = self.get();
        let f = two_pow_u(layer) as u32;
        LodIndex::new(xyz.map(|i| {
            (i / f) * f
        }))
    }

    pub fn get_highest_layer_that_fits(&self) -> u8 {
        let pos = self.get();
        cmp::min( cmp::min(cmp::min(pos[0].trailing_zeros(),
                pos[1].trailing_zeros()), pos[2].trailing_zeros()), 15) as u8
    }
}

impl fmt::Display for LodIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let xyz = self.get();
        //write!(f, "({}|{}|{}) <{}>", xyz[0], xyz[1], xyz[2], self.data)
        write!(f, "({}|{}|{})", xyz[0], xyz[1], xyz[2])
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct AbsIndex {
    pub layer: u8,
    pub index: usize,
}

impl AbsIndex {
    pub fn new(layer: u8, index: usize) -> Self {
        AbsIndex {
            layer,
            index,
        }
    }
}

impl fmt::Display for AbsIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}]", self.layer, self.index)
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

pub const fn two_pow_u(n: u8) -> u16 {
    1 << n
}

pub fn relative_to_1d(child_lod: LodIndex, parent_lod: LodIndex, child_layer: u8, relative_size: Vec3<u32>) -> usize {
    let width = two_pow_u(child_layer) as u32;
    let index = (child_lod.get() - parent_lod.get()).map(|e| e / width);
    (index[0] * relative_size[2] * relative_size[1] + index[1] * relative_size[2] + index[2]) as usize
}

pub fn min(lhs: LodIndex, rhs: LodIndex) -> LodIndex {
    let lhs = lhs.get();
    let rhs = rhs.get();
    LodIndex::new(lhs.map2(rhs, |a,b| cmp::min(a,b)))
}

pub fn max(lhs: LodIndex, rhs: LodIndex) -> LodIndex {
    let lhs = lhs.get();
    let rhs = rhs.get();
    LodIndex::new(lhs.map2(rhs, |a,b| cmp::max(a,b)))
}

/*************
    TESTS
**************/

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

    #[test]
    fn align() {
        let i = LodIndex::new(Vec3::new(1337,0,0)).align_to_layer_id(4);
        assert_eq!(i.get(), Vec3::new(1328,0,0));

        let i = LodIndex::new(Vec3::new(1337,1800,0)).align_to_layer_id(5);
        assert_eq!(i.get(), Vec3::new(1312,1792,0));

        let i = LodIndex::new(Vec3::new(1337,0,50)).align_to_layer_id(3);
        assert_eq!(i.get(), Vec3::new(1336,0,48));

        let i = LodIndex::new(Vec3::new(1335,0,0)).align_to_layer_id(3);
        assert_eq!(i.get(), Vec3::new(1328,0,0));

        let i = LodIndex::new(Vec3::new(31337,22000,25000)).align_to_layer_id(7);
        assert_eq!(i.get(), Vec3::new(31232,21888,24960));

        let i = LodIndex::new(Vec3::new(31337,22000,25000)).align_to_layer_id(0);
        assert_eq!(i.get(), Vec3::new(31337,22000,25000));

        let i = LodIndex::new(Vec3::new(0,0,0)).align_to_layer_id(4);
        assert_eq!(i.get(), Vec3::new(0,0,0));
    }

    #[test]
    fn get_highest_layer_that_fits() {
        let i = LodIndex::new(Vec3::new(0,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 15);
        let i = LodIndex::new(Vec3::new(1,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 0);
        let i = LodIndex::new(Vec3::new(2,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 1);
        let i = LodIndex::new(Vec3::new(3,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 0);
        let i = LodIndex::new(Vec3::new(4,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 2);
        let i = LodIndex::new(Vec3::new(5,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 0);

        let i = LodIndex::new(Vec3::new(1337,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 0);

        let i = LodIndex::new(Vec3::new(1337,1800,0));
        assert_eq!(i.get_highest_layer_that_fits(), 0);

        let i = LodIndex::new(Vec3::new(1338,0,50));
        assert_eq!(i.get_highest_layer_that_fits(), 1);

        let i = LodIndex::new(Vec3::new(1336,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 3);

        let i = LodIndex::new(Vec3::new(31348,22000,25000));
        assert_eq!(i.get_highest_layer_that_fits(), 2);

        let i = LodIndex::new(Vec3::new(0,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 15);

        let i = LodIndex::new(Vec3::new(65536,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 15);

        let i = LodIndex::new(Vec3::new(32768,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 15);

        let i = LodIndex::new(Vec3::new(16384,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 14);

        let i = LodIndex::new(Vec3::new(8192,0,0));
        assert_eq!(i.get_highest_layer_that_fits(), 13);

        let i = LodIndex::new(Vec3::new(65536,0,8192));
        assert_eq!(i.get_highest_layer_that_fits(), 13);
    }
}

