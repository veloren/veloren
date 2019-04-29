use vek::*;

/*
For our LodStructures we need a type that covers the values from 0 - 2047 in steps of 1/32.
which is 11 bits for the digits before the decimal point and 5 bits for the digits after the decimal point.
Because for accessing the decimal point makes no difference we use a u16 to represent this value.
The value needs to be shiftet to get it's "real inworld size",
e.g. 1 represents 1/32
     32 represents 1
     65535 represents 2047 + 31/32
*/

pub type LodIndex = Vec3<u16>;

pub fn to_lod_i(pos: Vec3<u16>) -> LodIndex {
    pos.map(|x| x * 32)
}

/*will round*/
pub fn to_lod_f(pos: Vec3<f32>) -> LodIndex {
    pos.map(|x| (x * 32.0).round() as u16)
}

pub fn to_pos_i(index: LodIndex) -> Vec3<u16> {
    index.map(|x| x / 32)
}

pub fn to_pos_f(index: LodIndex) -> Vec3<f32> {
    index.map(|x| x as f32 / 32.0)
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