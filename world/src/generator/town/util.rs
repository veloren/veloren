use rand::prelude::*;
use vek::*;

pub const UNITS: [Vec2<i32>; 4] = [
    Vec2 { x: 1, y: 0 },
    Vec2 { x: 0, y: 1 },
    Vec2 { x: -1, y: 0 },
    Vec2 { x: 0, y: -1 },
];

pub fn dir(i: usize) -> Vec2<i32> { UNITS[i % 4] }

pub fn unit(i: usize) -> (Vec2<i32>, Vec2<i32>) { (UNITS[i % 4], UNITS[(i + 1) % 4]) }

// unused
//pub fn gen_unit(rng: &mut impl Rng) -> (Vec2<i32>, Vec2<i32>) {
//    unit(rng.gen_range(0, 4))
//}

pub fn gen_dir(rng: &mut impl Rng) -> Vec2<i32> { UNITS[rng.gen_range(0, 4)] }

pub const UNITS_3D: [Vec3<i32>; 6] = [
    Vec3 { x: 1, y: 0, z: 0 },
    Vec3 { x: 0, y: 1, z: 0 },
    Vec3 { x: -1, y: 0, z: 0 },
    Vec3 { x: 0, y: -1, z: 0 },
    Vec3 { x: 0, y: 0, z: 1 },
    Vec3 { x: 0, y: 0, z: -1 },
];

pub fn dir_3d(i: usize) -> Vec3<i32> { UNITS_3D[i % 6] }

// unused
//pub fn gen_dir_3d(rng: &mut impl Rng) -> Vec3<i32> {
//    UNITS_3D[rng.gen_range(0, 6)]
//}
