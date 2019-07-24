use crate::lodstore::{
    LodData,
    LodConfig,
    index::LodIndex,
    index::AbsIndex,
    delta::LodDelta,
    delta::DefaultLodDelta,
};
use vek::*;
use std::u32;

#[derive(Debug, Clone)]
pub struct Region9 {
    precent_air: f32,
    percent_forrest: f32,
    percent_lava: f32,
    percent_water: f32,
}

#[derive(Debug, Clone)]
pub struct Chunk6 {
    precent_air: f32,
    percent_forrest: f32,
    percent_lava: f32,
    percent_water: f32,
}

#[derive(Debug, Clone)]
pub struct TerrainLodConfig {}

impl LodConfig for TerrainLodConfig {
    type L0 = ();
    type L1 = ();
    type L2 = ();
    type L3 = ();
    type L4 = ();
    type L5 = ();
    type L6 = ();
    type L7 = ();
    type L8 = ();
    type L9 = ();
    type L10 = Chunk6;
    type L11 = ();
    type L12 = ();
    type L13 = Region9;
    type L14 = ();
    type L15 = ();

    type I0 = ();
    type I1 = ();
    type I2 = ();
    type I3 = ();
    type I4 = ();
    type I5 = ();
    type I6 = ();
    type I7 = ();
    type I8 = ();
    type I9 = ();
    type I10 = ();
    type I11 = ();
    type I12 = ();
    type I13 = u16; // Chunk5 2^(6*3), this is valid
    type I14 = ();
    type I15 = ();

    type Delta = DefaultLodDelta<Self>;
    type Additional = ();

    const anchor_layer_id: u8 = 13;

    const layer_volume: [Vec3<u32>; 16] = [
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 0, y: 0, z: 0},
        Vec3{x: 8, y: 8, z: 8},
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
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(9),
        None,
        None,
    ];

    fn setup(&mut self) {

    }

    fn drill_down(data: &mut LodData::<Self>, abs: AbsIndex, delta: &mut Option<DefaultLodDelta<Self>>) {
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
                for i in 0..Self::layer_len[0] {
                    data.layer0.push(SubBlock_4{
                        material: 0,
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
                    data.layer4.push(Block0{
                        material: 0,
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
                    data.layer9.push(Chunk5{
                        precent_air: 0.2,
                        percent_forrest: 0.3,
                        percent_lava: 0.4,
                        percent_water: 0.1,
                    });
                    data.child9.push(u32::MAX);
                }
            },
            _ => unreachable!(),
        }
    }

    fn drill_up(data: &mut LodData::<Self>, parent_abs: AbsIndex, delta: &mut Option<DefaultLodDelta<Self>>) {
    unreachable!("finish this like in example");
        match parent_abs.layer {
            0 => {
                panic!("SubBlocks_4 does not have children");
            },
            4 => {
                //let delete = data.layer4[parent_abs.index].child_id.expect("has no childs to drill up") as usize;
                //data.layer4[parent_abs.index].child_id = None;
                //data.layer0.drain(delete..delete+Self::layer_len[0]);
            },
            9 => {
                //let delete = data.layer9[parent_abs.index].child_id.expect("has no childs to drill up") as usize;
                //data.layer9[parent_abs.index].child_id = None;
                //data.layer4.drain(delete..delete+Self::layer_len[4]);
            },
            13 => {
                //let delete = data.layer13[parent_abs.index].child_id.expect("has no childs to drill up") as usize;
                //data.layer13[parent_abs.index].child_id = None;
                //data.layer9.drain(delete..delete+Self::layer_len[9]);
            },
            _ => unreachable!(),
        }
    }
}

pub type Terrain = LodData<TerrainLodConfig>;