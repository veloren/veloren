// Standard

// Library
use coord::prelude::*;

// Project
use region::{Model as OurModel, Cell, Voxel as OurVoxel};
use client::Volume;

// Local
use model_object::{ModelObject, Constants};
use dot_vox::{DotVoxData, Model, Voxel};

pub fn vox_to_model(vox: DotVoxData) -> OurModel {
    let model = vox.models.first().unwrap();
    let block = <Cell as OurVoxel>::new( 255 );
    let mut chunk = OurModel::new();
    chunk.set_size(vec3!((model.size.x) as i64, (model.size.y) as i64, (model.size.z) as i64));
    chunk.set_offset(vec3!(0,0,0));
    chunk.set_scale(vec3!(0.1,0.1,0.1));
    chunk.fill(block);
    for ref v in model.voxels.iter() {
        let pos = vec3!(v.x as i64, v.y as i64, v.z as i64);
        //let ref mut block = chunk.at(vec3!(v.x as i64, v.y as i64, v.z as i64)).unwrap();
        let nblock = <Cell as OurVoxel>::new( v.i );
        chunk.set(pos, nblock);
    }

    //let chunk = Chunk::test(vec3!(0, 0, 0), vec3!(3,3,3));
    return chunk;
}
