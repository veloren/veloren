use crate::lodstore::{
    LodData,
    LayerInfo,
    LodConfig,
    index::LodIndex,
};
use vek::*;

#[derive(Clone)]
pub struct Example9 {
    data: [u8; 700],
    child_id: Option<u32>, // Chunk5 2^(7*3), this is valid
}

#[derive(Clone)]
pub struct Example5 {
    data: [u8; 130],
    child_id: Option<u32>, // see Block0 2^(12*3)
}

#[derive(Debug, Clone)]
pub struct Example0 {
    data: u32,
    child_id: Option<u32>,// In reality 2^(16*3) SubBlock_4 should be possible, but 2^48 subblocks would kill anything anyway, so save 2 bytes here
}

#[derive(Debug, Clone)]
pub struct Example_4 {
    data: u16,
}

impl Example9 {
    pub fn new() -> Self {
        Example9{
            data: [0; 700],
            child_id: None,
        }
    }
}

impl LayerInfo for Example9 {
    fn get_child_index(self: &Self) -> Option<usize> {
        self.child_id.map(|n| n as usize)
    }
    const child_layer_id: Option<u8> = Some(9);
    const layer_volume: Vec3<u32> = Vec3{x: 16, y: 16, z: 16};
    const child_len: usize = 4096;//2_usize.pow(Self::child_dim*3);
}

impl LayerInfo for Example5 {
    fn get_child_index(self: &Self) -> Option<usize> {
        self.child_id.map(|n| n as usize)
    }
    const child_layer_id: Option<u8> = Some(4);
    const layer_volume: Vec3<u32> = Vec3{x: 32, y: 32, z: 32};
    const child_len: usize = 32768;//2_usize.pow(Self::child_dim*3);
}

impl LayerInfo for Example0 {
    fn get_child_index(self: &Self) -> Option<usize> {
        self.child_id.map(|n| n as usize)
    }
    const child_layer_id: Option<u8> = Some(0);
    const layer_volume: Vec3<u32> = Vec3{x: 16, y: 16, z: 16};
    const child_len: usize = 4096;//2_usize.pow(Self::child_dim*3);
}

impl LayerInfo for Example_4 {
    fn get_child_index(self: &Self) -> Option<usize> { None }
    const child_layer_id: Option<u8> = None;
    const layer_volume: Vec3<u32> = Vec3{x: 1, y: 1, z: 1};
    const child_len: usize = 0;
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

    const anchor_layer_id: u8 = 13;

    fn setup(&mut self) {

    }

    fn drill_down(data: &mut LodData::<Self>, level: u8, index: usize) {
        match level {
            0 => {
                panic!("cannot drill down further");
            },
            4 => {
                if data.layer4[index].child_id.is_some() {return;}
                let insert = data.layer0.len();
                data.layer4.reserve(Example_4::child_len);
                data.layer4[index].child_id = Some(insert as u32);
                for i in 0..Example0::child_len {
                    data.layer0.push(Example_4{
                        data: 0,
                    });
                }
            },
            9 => {
                if data.layer9[index].child_id.is_some() {return;}
                let insert = data.layer4.len();
                data.layer9.reserve(Example0::child_len);
                data.layer9[index].child_id = Some(insert as u32);
                for i in 0..Example5::child_len {
                    data.layer4.push(Example0{
                        data: 0,
                        child_id: None,
                    });
                }
            },
            13 => {
                if data.layer13[index].child_id.is_some() {return;}
                let insert = data.layer9.len();
                data.layer13.reserve(Example9::child_len);
                data.layer13[index].child_id = Some(insert as u32);
                for i in 0..Example9::child_len {
                    data.layer9.push(Example5{
                        data: [0; 130],
                        child_id: None,
                    });
                }
            },
            _ => unreachable!(),
        }

    }

    fn drill_up(data: &mut LodData::<Self>, level: u8, parent_index: usize) {
        match level {
            0 => {
                panic!("SubBlocks_4 does not have children");
            },
            4 => {
                let delete = data.layer4[parent_index].child_id.expect("has no childs to drill up") as usize;
                data.layer4[parent_index].child_id = None;
                data.layer0.drain(delete..delete+Example0::child_len);
            },
            9 => {
                let delete = data.layer9[parent_index].child_id.expect("has no childs to drill up") as usize;
                data.layer9[parent_index].child_id = None;
                data.layer4.drain(delete..delete+Example5::child_len);
            },
            13 => {
                let delete = data.layer13[parent_index].child_id.expect("has no childs to drill up") as usize;
                data.layer13[parent_index].child_id = None;
                data.layer9.drain(delete..delete+Example9::child_len);
            },
            _ => unreachable!(),
        }
    }
}

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

    fn randIndex(rng: &mut ThreadRng) -> LodIndex {
        let x: u16 = rng.gen();
        let y: u16 = rng.gen();
        let z: u16 = rng.gen();
        LodIndex::new(Vec3::new(x,y,z).map(|x| x as u32))
    }


    pub type Example = LodData<ExampleLodConfig>;

    fn createRegion(p_e5: f32, p_e0: f32, p_e_4: f32, p_foreign: f32) -> Example {
        let mut rng = rand::thread_rng();
        let mut result = Example::new();
        let abs9 = (index::two_pow_u(15-13) as u64).pow(3);
        let abs5 = (index::two_pow_u(15-9) as u64).pow(3);
        let abs0 = (index::two_pow_u(15-4) as u64).pow(3);
        let abs_4 = (index::two_pow_u(15)  as u64).pow(3);
        let act9 = (abs9 as f32 * (1.0+p_foreign) ) as u32;
        let act5 = (abs5 as f32 * (p_e5*(1.0+p_foreign))) as u32;
        let act0 = (abs0 as f32 * (p_e0*(1.0+p_foreign))) as u32;
        let act_4 = (abs_4 as f32 * (p_e_4*(1.0+p_foreign))) as u32;

        let w9 = index::two_pow_u(13) as u32;
        result.layer13 = vec![Example9::new(); 8*8*8];
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    println!("{:?}", Vec3::new(x*w9,y*w9,z*w9));
                    println!("{:?}", LodIndex::new(Vec3::new(x*w9,y*w9,z*w9)));
                    result.anchor.insert(LodIndex::new(Vec3::new(x*w9,y*w9,z*w9)), (x+y*8+z*8*8) as usize);
                }
            }
        }
        while result.layer9.len() < act5 as usize {
            let index = randIndex(&mut rng);
            let low = index.align_to_layer_id(9);
            result.make_at_least(low,low,9);
        }/*
        while result.layer5.len() < act0 as usize {
            let index = randIndex(&mut rng);
            let low = index.align_to_layer_id(5);
            result.make_at_least(low,low,5);
            println!("{}", result.layer5.len());
        }*//*
        while result.layer0.len() < act_4 as usize {
            let index = randIndex(&mut rng);
            let low = index.align_to_layer_id(0);
            result.make_at_least(low,low,0);
        }*/

        println!("creating Region with {} 5er, {} 0er, {} -4er", act5, act0 , act_4);
        println!("created Region l13: {} l9: {} l5: {} l0: {}", result.layer13.len(), result.layer9.len(), result.layer5.len(), result.layer0.len());
        println!("size {} {} {}", size_of::<Example>(), size_of::<Example9>(), size_of::<Example5>());
        result
    }

    #[test]
    fn reagiontest() {
        let reg = createRegion(0.15, 0.01, 0.001, 0.1);

        thread::sleep(time::Duration::from_secs(4));
        /*
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
        */
    }
}