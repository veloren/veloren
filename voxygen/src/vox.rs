// Standard
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex, RwLock, RwLockWriteGuard};
use std::sync::atomic::{AtomicBool, Ordering};
//use std::f32::{sin, cos};

// Library
use coord::prelude::*;

// Project
use region::{Chunk, BlockMaterial, Block, Voxel as OurVoxel};
use client::Volume;

// Local
use model_object::{ModelObject, Constants};
use mesh::{Mesh, Vertex};
use dot_vox::{DotVoxData, Model, Voxel};

pub fn vox_to_model(vox: DotVoxData) -> Chunk {
    let model = vox.models.first().unwrap();
    let block = <Block as OurVoxel>::new( BlockMaterial::Air );
    let mut chunk = Chunk::filled_with_size_offset(vec3!((model.size.x+1) as i64, (model.size.y+1) as i64, (model.size.z+1) as i64), vec3!(0,0,0), block);
    for ref v in model.voxels.iter() {
        let pos = vec3!(v.x as i64, v.y as i64, v.z as i64);
        //let ref mut block = chunk.at(vec3!(v.x as i64, v.y as i64, v.z as i64)).unwrap();
        println!("{:?}", pos);
        let nblock = <Block as OurVoxel>::new( BlockMaterial::Stone );
        chunk.set(pos, nblock);
    }

    //let chunk = Chunk::test(vec3!(0, 0, 0), vec3!(3,3,3));
    return chunk;
}
