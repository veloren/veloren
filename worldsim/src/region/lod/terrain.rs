use crate::lodstore::{
    LodData,
    LayerInfo,
    LodConfig,
    index::LodIndex,
};
use vek::*;

#[derive(Debug, Clone)]
pub struct Region9 {
    precent_air: f32,
    percent_forrest: f32,
    percent_lava: f32,
    percent_water: f32,
    child_id: Option<u32>, // Chunk5 2^(7*3), this is valid
}

#[derive(Debug, Clone)]
pub struct Chunk5 {
    precent_air: f32,
    percent_forrest: f32,
    percent_lava: f32,
    percent_water: f32,
    child_id: Option<u32>, // see Block0 2^(12*3)
}

#[derive(Debug, Clone)]
pub struct Block0 {
    material: u32,
    child_id: Option<u32>,// In reality 2^(16*3) SubBlock_4 should be possible, but 2^48 subblocks would kill anything anyway, so save 2 bytes here
}

#[derive(Debug, Clone)]
pub struct SubBlock_4 {
    material: u32,
}

impl LayerInfo for Region9 {
    fn get_child_index(self: &Self) -> Option<usize> {
        self.child_id.map(|n| n as usize)
    }
    const child_layer_id: Option<u8> = Some(9);
    const layer_volume: Vec3<u32> = Vec3{x: 16, y: 16, z: 16};
    const child_len: usize = 4096;//2_usize.pow(Self::child_dim*3);
}

impl LayerInfo for Chunk5 {
    fn get_child_index(self: &Self) -> Option<usize> {
        self.child_id.map(|n| n as usize)
    }
    const child_layer_id: Option<u8> = Some(4);
    const layer_volume: Vec3<u32> = Vec3{x: 32, y: 32, z: 32};
    const child_len: usize = 32768;//2_usize.pow(Self::child_dim*3);
}

impl LayerInfo for Block0 {
    fn get_child_index(self: &Self) -> Option<usize> {
        self.child_id.map(|n| n as usize)
    }
    const child_layer_id: Option<u8> = Some(0);
    const layer_volume: Vec3<u32> = Vec3{x: 16, y: 16, z: 16};
    const child_len: usize = 4096;//2_usize.pow(Self::child_dim*3);
}

impl LayerInfo for SubBlock_4 {
    fn get_child_index(self: &Self) -> Option<usize> {
        None
    }
    const child_layer_id: Option<u8> = None;
    const layer_volume: Vec3<u32> = Vec3{x: 1, y: 1, z: 1};
    const child_len: usize = 0;
}

#[derive(Debug, Clone)]
pub struct TerrainLodConfig {}

impl LodConfig for TerrainLodConfig {
    type L0 = SubBlock_4;
    type L1 = ();
    type L2 = ();
    type L3 = ();
    type L4 = Block0;
    type L5 = ();
    type L6 = ();
    type L7 = ();
    type L8 = ();
    type L9 = Chunk5;
    type L10 = ();
    type L11 = ();
    type L12 = ();
    type L13 = Region9;
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
                let insert = data.layer0.len();
                data.layer4[index].child_id = Some(insert as u32);
                for i in 0..Block0::child_len {
                    data.layer0[i+insert] = SubBlock_4{
                        material: 0,
                    };
                }
            },
            9 => {
                let insert = data.layer4.len();
                data.layer9[index].child_id = Some(insert as u32);
                for i in 0..Chunk5::child_len {
                    data.layer4[i+insert] = Block0{
                        material: 0,
                        child_id: None,
                    };
                }
            },
            13 => {
                let insert = data.layer9.len();
                    data.layer13[index].child_id = Some(insert as u32);
                    for i in 0..Region9::child_len {
                        data.layer9[i+insert] = Chunk5{
                        precent_air: 0.2,
                        percent_forrest: 0.3,
                        percent_lava: 0.4,
                        percent_water: 0.1,
                        child_id: None,
                    };
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
                data.layer0.drain(delete..delete+Block0::child_len);
            },
            9 => {
                let delete = data.layer9[parent_index].child_id.expect("has no childs to drill up") as usize;
                data.layer9[parent_index].child_id = None;
                data.layer4.drain(delete..delete+Chunk5::child_len);
            },
            13 => {
                let delete = data.layer13[parent_index].child_id.expect("has no childs to drill up") as usize;
                data.layer13[parent_index].child_id = None;
                data.layer9.drain(delete..delete+Region9::child_len);
            },
            _ => unreachable!(),
        }
    }
}

pub type Terrain = LodData<TerrainLodConfig>;