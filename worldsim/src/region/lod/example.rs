use crate::lodstore::{
    LodData,
    LodConfig,
    data::CacheLine,
    index::LodIndex,
    index::AbsIndex,
    area::LodArea,
    delta::LodDelta,
};
use vek::*;
use std::u32;

#[derive(Clone)]
pub struct Example9 {
    data: [u8; 700],
}

#[derive(Clone)]
pub struct Example5 {
    data: [u8; 130],
}

#[derive(Debug, Clone)]
pub struct Example0 {
    data: u32,
}

#[derive(Debug, Clone)]
pub struct Example_4 {
    data: u16,
}

impl Example9 {
    pub fn new() -> Self {
        Example9{
            data: [0; 700],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExampleLodConfig {}

impl LodConfig for ExampleLodConfig {
    type L0 = Example_4;
    type L1 = ();
    type L2 = ();
    type L3 = ();
    type L4 = Example0;
    type L5 = ();
    type L6 = ();
    type L7 = ();
    type L8 = ();
    type L9 = Example5;
    type L10 = ();
    type L11 = ();
    type L12 = ();
    type L13 = Example9;
    type L14 = ();
    type L15 = ();

    type I0 = ();
    type I1 = ();
    type I2 = ();
    type I3 = ();
    type I4 = u32; // In reality 2^(16*3) SubBlock_4 should be possible, but 2^48 subblocks would kill anything anyway, so save 2 bytes here
    type I5 = ();
    type I6 = ();
    type I7 = ();
    type I8 = ();
    type I9 = u32; // see Block0 2^(12*3)
    type I10 = ();
    type I11 = ();
    type I12 = ();
    type I13 = u32; // Chunk5 2^(7*3), this is valid
    type I14 = ();
    type I15 = ();

    type Additional = ();

    const anchor_layer_id: u8 = 13;

    // this is not for the children, a layer9 has 32x32x32 childs, not 16x16x16
    const layer_volume: [Vec3<u32>; 16] = [
        Vec3{x: 16, y: 16, z: 16},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 32, y: 32, z: 32},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 16, y: 16, z: 16},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 8, y: 8, z: 8},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
    ];
    const child_layer_id: [Option<u8>; 16] = [
        None,
        None,
        None,
        None,
        Some(0),
        None,
        None,
        None,
        None,
        Some(4),
        None,
        None,
        None,
        Some(9),
        None,
        None,
    ];

    fn setup(&mut self) {

    }

    fn drill_down(data: &mut LodData::<Self>, abs: AbsIndex) {
        match abs.layer {
            0 => {
                panic!("cannot drill down further");
            },
            4 => {
                if data.child4[abs.index] != u32::MAX {return;}
                let insert = data.layer0.len();
                data.layer0.reserve(Self::layer_len[0]);
                data.child4[abs.index] = insert as u32;
                //debug!("set0 {:?} = {}", abs, insert);
                // in the future use something like a child_index as parameter and a RawVec for allocations
                //data.layer0[child_index..child_index+Self::layer_len[0]].iter_mut().map(
                //                    |e| *e = Example_4{
                //                        data: 0,
                //                    }
                //                );
                for i in 0..Self::layer_len[0] {
                    data.layer0.push(Example_4{
                        data: 0,
                    });
                }
            },
            9 => {
                if data.child9[abs.index] != u32::MAX {return;}
                let insert = data.layer4.len();
                data.layer4.reserve(Self::layer_len[4]);
                data.child4.reserve(Self::layer_len[4]);
                data.child9[abs.index] = insert as u32;
                //debug!("set4 {:?} = {}", abs, insert);
                for i in 0..Self::layer_len[4] {
                    data.layer4.push(Example0{
                        data: 0,
                    });
                    data.child4.push(u32::MAX);
                }
            },
            13 => {
                if data.child13[abs.index] != u32::MAX {return;}
                let insert = data.layer9.len();
                data.layer9.reserve(Self::layer_len[9]);
                data.child9.reserve(Self::layer_len[9]);
                data.child13[abs.index] = insert as u32;
                //debug!("set13 {:?} = {}", abs, insert);
                for i in 0..Self::layer_len[9] {
                    data.layer9.push(Example5{
                        data: [0; 130],
                    });
                    data.child9.push(u32::MAX);
                }
            },
            _ => unreachable!(),
        }

    }

    fn drill_up(data: &mut LodData::<Self>, parent_abs: AbsIndex) {
        match parent_abs.layer {
            0 => {
                panic!("SubBlocks_4 does not have children");
            },
            4 => {
                let delete = data.child4[parent_abs.index] as usize;
                data.child4[parent_abs.index] = u32::MAX;
                data.layer0.drain(delete..delete+Self::layer_len[0]);
                data.child0.drain(delete..delete+Self::layer_len[0]);
            },
            9 => {
                let delete = data.child9[parent_abs.index] as usize;
                data.child9[parent_abs.index] = u32::MAX;
                data.layer4.drain(delete..delete+Self::layer_len[4]);
                data.child4.drain(delete..delete+Self::layer_len[4]);
            },
            13 => {
                let delete = data.child13[parent_abs.index] as usize;
                data.child13[parent_abs.index] = u32::MAX;
                data.layer9.drain(delete..delete+Self::layer_len[9]);
                data.child9.drain(delete..delete+Self::layer_len[9]);
            },
            _ => unreachable!(),
        }
    }
}

//DELTA


#[cfg(test)]
mod tests {
    use crate::{
        region::lod::example::ExampleLodConfig,
        region::lod::example::*,
        lodstore::LodData,
        lodstore::index::LodIndex,
        lodstore::index,
    };
    use std::{thread, time, mem::size_of};
    use vek::*;
    use rand::Rng;
    use rand::ThreadRng;
    use test::Bencher;

    fn randIndex(rng: &mut ThreadRng) -> LodIndex {
        let x: u16 = rng.gen();
        let y: u16 = rng.gen();
        let z: u16 = rng.gen();
        LodIndex::new(Vec3::new(x,y,z).map(|x| x as u32))
    }


    pub type Example = LodData<ExampleLodConfig>;
    pub type ExampleDelta = LodDelta::<ExampleLodConfig>;

    fn createRegion(p_e5: f32, p_e0: f32, p_e_4: f32, p_foreign: f32) -> ( Example, ExampleDelta ) {
        let mut rng = rand::thread_rng();
        let mut delta = ExampleDelta::new();
        let mut result = Example::new();
        let abs9 = (index::two_pow_u(15-13) as u64).pow(3);
        let abs5 = (index::two_pow_u(15-9) as u64).pow(3);
        let abs0 = (index::two_pow_u(15-4) as u64).pow(3);
        let abs_4 = (index::two_pow_u(15)  as u64).pow(3);
        let p_e9 = 1.0+p_foreign;
        let p_e5 = p_e9*p_e5;
        let p_e0 = p_e5*p_e0;
        let p_e_4 = p_e0*p_e_4;
        let act9 = (abs9 as f32 * p_e9 ) as u32;
        let act5 = (abs5 as f32 * p_e5) as u32;
        let act0 = (abs0 as f32 * p_e0 ) as u32;
        let act_4 = (abs_4 as f32 * p_e_4 ) as u32;

        let w9 = index::two_pow_u(13) as u32;
        result.layer13 = vec![Example9::new(); 8*8*8];
        result.child13 = vec![u32::MAX; 8*8*8];
        println!("size test {} -- {}", size_of::<usize>(), size_of::<Option<usize>>());
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    result.anchor.insert(LodIndex::new(Vec3::new(x*w9,y*w9,z*w9)), (x*8*8+y*8+z) as usize);
                }
            }
        }

        println!("creating Region with {} 5er, {} 0er, {} -4er", act5, act0 , act_4);
        while result.layer9.len() < act5 as usize {
            let index = randIndex(&mut rng);
            let low = index.align_to_layer_id(9);
            let area = LodArea::new(low, low);
            result.make_at_least(area,9, Some(&mut delta));
        }
        while result.layer4.len() < act0 as usize {
            let index = randIndex(&mut rng);
            let low = index.align_to_layer_id(4);
            let area = LodArea::new(low, low);
            result.make_at_least(area, 4, Some(&mut delta));
        }
        while result.layer0.len() < act_4 as usize {
            let index = randIndex(&mut rng);
            let low = index.align_to_layer_id(0);
            let area = LodArea::new(low, low);
            result.make_at_least(area, 0, Some(&mut delta));
        }

        println!("creating Region with {} 5er, {} 0er, {} -4er", act5, act0 , act_4);
        println!("created Region l13: {} l9: {} l5: {} l0: {}", result.layer13.len(), result.layer9.len(), result.layer4.len(), result.layer0.len());
        println!("size {} {} {}", size_of::<Example>(), size_of::<Example9>(), size_of::<Example5>());
        (result, delta)
    }

    #[test]
    fn regiontest() {
        let reg = createRegion(0.0015, 0.001, 0.001, 0.1);
    }

    #[test]
    fn reagionmake_at_least() {
        let (mut reg, _) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(8192, 8192, 8192));
        let high = LodIndex::new(Vec3::new(16384, 16384, 16384));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 4, None);
    }

    #[test]
    fn access_0a() {
        let (mut reg, _) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(0, 0, 0));
        let high = LodIndex::new(Vec3::new(4, 4, 4));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 0, None);
        reg.get0(LodIndex::new(Vec3::new(0, 0, 0)));
        reg.get0(LodIndex::new(Vec3::new(1, 0, 0)));
        reg.get0(LodIndex::new(Vec3::new(0, 1, 0)));
        reg.get0(LodIndex::new(Vec3::new(0, 0, 1)));
        reg.get0(LodIndex::new(Vec3::new(1, 1, 1)));
        reg.get0(LodIndex::new(Vec3::new(2, 2, 2)));
        reg.get0(LodIndex::new(Vec3::new(3, 3, 3)));
        reg.get0(LodIndex::new(Vec3::new(4, 4, 4)));
    }

    #[test]
    fn access_0b() {
        let (mut reg, _) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(8704, 8704, 8704));
        let high = LodIndex::new(Vec3::new(9216, 9216, 9216));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 0, None);
        reg.get0(LodIndex::new(Vec3::new(8704, 8704, 8704)));
        reg.get0(LodIndex::new(Vec3::new(9000, 9000, 9000)));
        reg.get0(LodIndex::new(Vec3::new(9000, 9000, 9001)));
        reg.get0(LodIndex::new(Vec3::new(9001, 9000, 9000)));
        reg.get0(LodIndex::new(Vec3::new(9001, 9001, 9001)));
        reg.get0(LodIndex::new(Vec3::new(9216, 9216, 9216)));
        reg.get4(LodIndex::new(Vec3::new(9000, 9000, 9000)));
        reg.get9(LodIndex::new(Vec3::new(9000, 9000, 9000)));
    }

    #[test]
    #[should_panic]
    fn access_0c_fail() {
        let (mut reg, _) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(8704, 8704, 8704));
        let high = LodIndex::new(Vec3::new(9216, 9216, 9216));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 0, None);
        reg.get0(LodIndex::new(Vec3::new(8704, 8704, 8703)));
    }

    #[test]
    #[should_panic]
    fn access_0d_fail() {
        let (mut reg, _) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(8704, 8704, 8704));
        let high = LodIndex::new(Vec3::new(9216, 9216, 9216));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 0, None);
        reg.get0(LodIndex::new(Vec3::new(10240, 10240, 10240)));
    }

    #[test]
    fn access_4() {
        let (mut reg, _) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(8192, 8192, 8192));
        let high = LodIndex::new(Vec3::new(10240, 10240, 10240));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 4, None);
    }

    #[test]
    #[should_panic]
    fn access_0_fail() {
        let (mut reg, _) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(0, 0, 0));
        let high = LodIndex::new(Vec3::new(4, 4, 4));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 0, None);
        reg.get0(LodIndex::new(Vec3::new(5, 5, 5))); //this access is not guaranteed but will work
        reg.get0(LodIndex::new(Vec3::new(16, 16, 16))); // out of range
    }

    #[bench]
    fn bench_region(b: &mut Bencher) {
        b.iter(|| createRegion(0.0015, 0.001, 0.001, 0.1));
    }

    #[bench]
    fn bench_clone_region(b: &mut Bencher) {
        let (mut reg, _) = createRegion(0.00015, 0.0001, 0.00000001, 0.1);
        b.iter(|| reg.clone());
    }

    #[bench]
    fn bench_make_at_least1(b: &mut Bencher) {
        let (reg, _) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(0, 0, 0));
        let high = LodIndex::new(Vec3::new(255, 255, 255));
        b.iter(|| {
            let mut reg2 = reg.clone();
            let area = LodArea::new(low, high);
            reg2.make_at_least(area, 0, None);
        });
    }

    #[bench]
    fn bench_make_at_least2(b: &mut Bencher) {
        let (reg, _) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(0, 0, 0));
        let high = LodIndex::new(Vec3::new(4, 4, 4));
        b.iter(|| {
            let mut reg2 = reg.clone();
            let area = LodArea::new(low, high);
            reg2.make_at_least(area, 0, None);
        });
    }

    #[bench]
    fn bench_make_at_least3(b: &mut Bencher) {
        let (reg, _) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(8192, 8192, 8192));
        let high = LodIndex::new(Vec3::new(10240, 10240, 10240));
        b.iter(|| {
            let mut reg2 = reg.clone();
            let area = LodArea::new(low, high);
            reg2.make_at_least(area, 4, None);
        });
    }

    #[bench]
    fn bench_access_0_cached(b: &mut Bencher) {
        let (mut reg, _) = createRegion(0.0015, 0.001, 0.001, 0.1);
        let access = LodIndex::new(Vec3::new(8561, 8312, 8412));
        let low = LodIndex::new(Vec3::new(8192, 8192, 8192));
        let high = LodIndex::new(Vec3::new(8800, 8800, 8800));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 0, None);
        let mut cache = CacheLine::new();
        b.iter(|| reg.get0_cached(&mut cache, access));
    }

    #[bench]
    fn bench_access_0(b: &mut Bencher) {
        let (mut reg, _) = createRegion(0.0015, 0.001, 0.001, 0.1);
        let access = LodIndex::new(Vec3::new(8561, 8312, 8412));
        let low = LodIndex::new(Vec3::new(8192, 8192, 8192));
        let high = LodIndex::new(Vec3::new(8800, 8800, 8800));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 0, None);
        b.iter(|| reg.get0(access));
    }

    #[bench]
    fn bench_access_0_4_multiple(b: &mut Bencher) {
        let (mut reg, _) = createRegion(0.0015, 0.001, 0.001, 0.1);
        let low = LodIndex::new(Vec3::new(8192, 8192, 8192));
        let high = LodIndex::new(Vec3::new(8800, 8800, 8800));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 0, None);
        b.iter(|| {
            reg.get0(LodIndex::new(Vec3::new(8561, 8312, 8412)));
            reg.get0(LodIndex::new(Vec3::new(8200, 8599, 8413)));
            reg.get0(LodIndex::new(Vec3::new(8300, 8782, 8414)));
            reg.get0(LodIndex::new(Vec3::new(8761, 8352, 8212)));
            reg.get0(LodIndex::new(Vec3::new(8261, 8282, 8712)));
            reg.get0(LodIndex::new(Vec3::new(8461, 8752, 8652)));
            reg.get0(LodIndex::new(Vec3::new(8661, 8512, 8582)));
            reg.get0(LodIndex::new(Vec3::new(8461, 8612, 8419)));
            reg.get0(LodIndex::new(Vec3::new(8261, 8192, 8414)));
            reg.get0(LodIndex::new(Vec3::new(8761, 8192, 8192)));
            reg.get4(LodIndex::new(Vec3::new(8448, 8704, 8704)));
            reg.get4(LodIndex::new(Vec3::new(8461, 8448, 8704)));
            reg.get4(LodIndex::new(Vec3::new(8704, 8192, 8704)));
            reg.get4(LodIndex::new(Vec3::new(8192, 8704, 8192)));
        });
    }

    #[bench]
    fn bench_access_0_random1(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let (mut reg, _) = createRegion(0.0015, 0.001, 0.001, 0.1);
        let low = LodIndex::new(Vec3::new(8192, 8192, 8192));
        let high = LodIndex::new(Vec3::new(9192, 9192, 9192));
        let mut accesslist = Vec::new();
        for i in 0..1000000 {
            let x: u16 = rng.gen();
            let y: u16 = rng.gen();
            let z: u16 = rng.gen();
            accesslist.push(LodIndex::new(Vec3::new(x,y,z).map(|x| (8192 + x / 66) as u32)));
        }
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 0, None);
        b.iter(|| {
            for i in 0..1000000 {
                reg.get0(accesslist[i]);
            }
        });
    }

    #[bench]
    fn bench_access_0_random2(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let (mut reg, _) = createRegion(0.0015, 0.001, 0.001, 0.1);
        let low = LodIndex::new(Vec3::new(8192, 8192, 8192));
        let high = LodIndex::new(Vec3::new(9192, 9192, 9192));
        let mut accesslist = Vec::new();
        for i in 0..9990000 {
            let x: u16 = rng.gen();
            let y: u16 = rng.gen();
            let z: u16 = rng.gen();
            accesslist.push(LodIndex::new(Vec3::new(x,y,z).map(|x| (8192 + x / 66) as u32)));
        }
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 0, None);
        b.iter(|| {
            for i in 0..9990000 {
                reg.get0(accesslist[i]);
            }
        });
    }

    #[bench]
    fn bench_access_4(b: &mut Bencher) {
        let (mut reg, _) = createRegion(0.0, 0.0, 0.0, 0.1);
        let access = LodIndex::new(Vec3::new(9561, 9312, 8412));
        let low = LodIndex::new(Vec3::new(8192, 8192, 8192));
        let high = LodIndex::new(Vec3::new(10240, 10240, 10240));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 4, None);

        b.iter(|| reg.get4(access));
    }

    // DELTA TESTS
    #[test]
    fn delta_make_at_least() {
        let (mut reg, mut delta) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(8704, 8704, 8704));
        let high = LodIndex::new(Vec3::new(9216, 9216, 9216));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 4, Some(&mut delta));

        //assert_eq!(delta.layer4.len(), 20);
    }
    #[test]
    fn delta_set() {
        let (mut reg, mut delta) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(8704, 8704, 8704));
        let high = LodIndex::new(Vec3::new(9216, 9216, 9216));
        let access = LodIndex::new(Vec3::new(9000, 9000, 9000));
        let area = LodArea::new(low, high);
        reg.make_at_least(area, 4, Some(&mut delta));
        let x = reg.get4(access);
        assert_eq!(delta.layer4.len(), 0);
        reg.set4(access, x.clone(), Some(&mut delta));
        assert_eq!(delta.layer4.len(), 1);
    }

    #[test]
    fn delta_filter() {
        let (mut reg, mut delta) = createRegion(0.0, 0.0, 0.0, 0.1);
        let low = LodIndex::new(Vec3::new(8704, 8704, 8704));
        let high = LodIndex::new(Vec3::new(9216, 9216, 9216));
        let access1 = LodIndex::new(Vec3::new(9000, 9000, 9000));
        let access2 = LodIndex::new(Vec3::new(9001, 9000, 9000));
        let access3 = LodIndex::new(Vec3::new(9002, 9001, 9000));
        let access4 = LodIndex::new(Vec3::new(9003, 9000, 9000));
        let area = LodArea::new(low, high);
        let area2 = LodArea::new(low, access2);
        let area3 = LodArea::new(low, access1);
        reg.make_at_least(area, 4, Some(&mut delta));
        let x = reg.get4(access1).clone();
        reg.set4(access1, x.clone(), Some(&mut delta));
        reg.set4(access2, x.clone(), Some(&mut delta));
        reg.set4(access3, x.clone(), Some(&mut delta));
        reg.set4(access4, x.clone(), Some(&mut delta));
        assert_eq!(delta.layer4.len(), 4);
        let delta = delta.filter(area);
        assert_eq!(delta.layer4.len(), 4);
        let delta = delta.filter(area2);
        assert_eq!(delta.layer4.len(), 2);
        let delta = delta.filter(area);
        assert_eq!(delta.layer4.len(), 2);
        let delta = delta.filter(area3);
        assert_eq!(delta.layer4.len(), 1);
    }

}