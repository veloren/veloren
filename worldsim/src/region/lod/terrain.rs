use crate::lodstore::{
    LodData,
    LayerInfo,
    LodConfig,
    index::LodIndex,
    index::AbsIndex,
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
}

impl LayerInfo for Chunk5 {
    fn get_child_index(self: &Self) -> Option<usize> {
        self.child_id.map(|n| n as usize)
    }
}

impl LayerInfo for Block0 {
    fn get_child_index(self: &Self) -> Option<usize> {
        self.child_id.map(|n| n as usize)
    }
}

impl LayerInfo for SubBlock_4 {
    fn get_child_index(self: &Self) -> Option<usize> {
        None
    }
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

    const layer_volume: [Vec3<u32>; 16] = [
        Vec3{x: 1, y: 1, z: 1},
        Vec3{x: 1, y: 1, z: 1},
        Vec3{x: 1, y: 1, z: 1},
        Vec3{x: 1, y: 1, z: 1},
        Vec3{x: 16, y: 16, z: 16},
        Vec3{x: 1, y: 1, z: 1},
        Vec3{x: 1, y: 1, z: 1},
        Vec3{x: 1, y: 1, z: 1},
        Vec3{x: 1, y: 1, z: 1},
        Vec3{x: 32, y: 32, z: 32},
        Vec3{x: 1, y: 1, z: 1},
        Vec3{x: 1, y: 1, z: 1},
        Vec3{x: 1, y: 1, z: 1},
        Vec3{x: 16, y: 16, z: 16},
        Vec3{x: 1, y: 1, z: 1},
        Vec3{x: 1, y: 1, z: 1},
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
        Some(5),
        None,
        None,
        None,
        Some(9),
        None,
        None,
    ];
    const child_len: [usize; 16] = [
        (Self::layer_volume[0].x * Self::layer_volume[0].y * Self::layer_volume[0].z) as usize,
        (Self::layer_volume[1].x * Self::layer_volume[1].y * Self::layer_volume[1].z) as usize,
        (Self::layer_volume[2].x * Self::layer_volume[2].y * Self::layer_volume[2].z) as usize,
        (Self::layer_volume[3].x * Self::layer_volume[3].y * Self::layer_volume[3].z) as usize,
        (Self::layer_volume[4].x * Self::layer_volume[4].y * Self::layer_volume[4].z) as usize,
        (Self::layer_volume[5].x * Self::layer_volume[5].y * Self::layer_volume[5].z) as usize,
        (Self::layer_volume[6].x * Self::layer_volume[6].y * Self::layer_volume[6].z) as usize,
        (Self::layer_volume[7].x * Self::layer_volume[7].y * Self::layer_volume[7].z) as usize,
        (Self::layer_volume[8].x * Self::layer_volume[8].y * Self::layer_volume[8].z) as usize,
        (Self::layer_volume[9].x * Self::layer_volume[9].y * Self::layer_volume[9].z) as usize,
        (Self::layer_volume[10].x * Self::layer_volume[10].y * Self::layer_volume[10].z) as usize,
        (Self::layer_volume[11].x * Self::layer_volume[11].y * Self::layer_volume[11].z) as usize,
        (Self::layer_volume[12].x * Self::layer_volume[12].y * Self::layer_volume[12].z) as usize,
        (Self::layer_volume[13].x * Self::layer_volume[13].y * Self::layer_volume[13].z) as usize,
        (Self::layer_volume[14].x * Self::layer_volume[14].y * Self::layer_volume[14].z) as usize,
        (Self::layer_volume[15].x * Self::layer_volume[15].y * Self::layer_volume[15].z) as usize,
    ];

    fn setup(&mut self) {

    }

    fn drill_down(data: &mut LodData::<Self>, abs: AbsIndex) {
        match abs.layer {
            0 => {
                panic!("cannot drill down further");
            },
            4 => {
                let insert = data.layer0.len();
                data.layer4[abs.index].child_id = Some(insert as u32);
                for i in 0..Self::child_len[4] {
                    data.layer0[i+insert] = SubBlock_4{
                        material: 0,
                    };
                }
            },
            9 => {
                let insert = data.layer4.len();
                data.layer9[abs.index].child_id = Some(insert as u32);
                for i in 0..Self::child_len[9] {
                    data.layer4[i+insert] = Block0{
                        material: 0,
                        child_id: None,
                    };
                }
            },
            13 => {
                let insert = data.layer9.len();
                    data.layer13[abs.index].child_id = Some(insert as u32);
                    for i in 0..Self::child_len[13] {
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

    fn drill_up(data: &mut LodData::<Self>, parent_abs: AbsIndex) {
        match parent_abs.layer {
            0 => {
                panic!("SubBlocks_4 does not have children");
            },
            4 => {
                let delete = data.layer4[parent_abs.index].child_id.expect("has no childs to drill up") as usize;
                data.layer4[parent_abs.index].child_id = None;
                data.layer0.drain(delete..delete+Self::child_len[4]);
            },
            9 => {
                let delete = data.layer9[parent_abs.index].child_id.expect("has no childs to drill up") as usize;
                data.layer9[parent_abs.index].child_id = None;
                data.layer4.drain(delete..delete+Self::child_len[9]);
            },
            13 => {
                let delete = data.layer13[parent_abs.index].child_id.expect("has no childs to drill up") as usize;
                data.layer13[parent_abs.index].child_id = None;
                data.layer9.drain(delete..delete+Self::child_len[13]);
            },
            _ => unreachable!(),
        }
    }
}

pub type Terrain = LodData<TerrainLodConfig>;