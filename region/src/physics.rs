// Standard
use std::thread;
use std::time;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, Barrier};
use std::collections::HashMap;
use std::net::{ToSocketAddrs};

// Library
use coord::prelude::*;

// Project
use common::{Uid};

// Local
use super::{Entity, VolMgr, VolGen, VolState, Chunk, Voxel, Volume, collision::Collidable, collision::Cuboid};

pub fn tick<P: Send + Sync + 'static>(entities: &RwLock<HashMap<Uid, Entity>>,
            chunk_mgr: &VolMgr<Chunk, P>,
            chunk_size: i64,
            dt: f32) {
    let mut entities = entities.write().unwrap();
    for (.., entity) in entities.iter_mut() {
        let (chunk_x, chunk_y) = (
            (entity.pos().x as i64).div_euc(chunk_size),
            (entity.pos().y as i64).div_euc(chunk_size)
        );

        // Gravity
        match chunk_mgr.at(vec2!(chunk_x, chunk_y)) {
            Some(c) => match *c.read().unwrap() {
                VolState::Exists(_, _) => entity.move_dir_mut().z -= 0.2,
                _ => {},
            }
            None => {},
        }

        let move_dir = entity.move_dir();
        *entity.pos_mut() += move_dir * dt;

        /*
        let player_col = Collidable::Cuboid{cuboid: Cuboid::new(vec3!(
            (entity.pos().x as i64 + x),
            (entity.pos().y as i64 + y),
            (entity.pos().z as i64 + z)
        ), vec3!(
            0.5, 0.5, 0.5
        ))};
        */

        /*
        for x in -1..2 {
            for y in -1..2 {
                for z in -1..2 {
                    let vox = chunk_mgr.get_voxel(vec3!(
                        (entity.pos().x as i64 + x),
                        (entity.pos().y as i64 + y),
                        (entity.pos().z as i64 + z)
                    ));
                    if vox.is_solid() {
                        let a = Collidable{}

                        ERROROROROROOROR
                        let player_col = Collidable::Cuboid{cuboid: Cuboid::new{vec3!(
                            (entity.pos().x as i64 + x),
                            (entity.pos().y as i64 + y),
                            (entity.pos().z as i64 + z)
                        ), vec3!(
                            0.5, 0.5, 0.5,
                        )}}}

                        let col_res = resolve_collision()
                        entity.move_dir_mut().z = 0.0;
                        entity.pos_mut().z += 0.0025;
                    };
                }
            }
        }
        */


        while chunk_mgr.get_voxel_at(vec3!(
            (entity.pos().x as i64),
            (entity.pos().y as i64),
            (entity.pos().z as i64)
        )).is_solid() {
            entity.move_dir_mut().z = 0.0;
            entity.pos_mut().z += 0.0025;
        }


    }
}
