use vek::*;
use specs::{Join, Read, ReadStorage, System, WriteStorage, ReadExpect};
use crate::{
    comp::phys::{Pos, Vel},
    state::DeltaTime,
    terrain::TerrainMap,
    vol::{Vox, ReadVol},
};

// Basic ECS physics system
pub struct Sys;

const GRAVITY: f32 = 9.81;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadExpect<'a, TerrainMap>,
        Read<'a, DeltaTime>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
    );

    fn run(&mut self, (terrain, dt, mut positions, mut velocities): Self::SystemData) {
        for (pos, vel) in (&mut positions, &mut velocities).join() {
            // Gravity
            vel.0.z -= GRAVITY * dt.0 as f32;

            // Movement
            pos.0 += vel.0 * dt.0 as f32;

            // Basic collision with terrain
            while terrain
                .get(pos.0.map(|e| e as i32))
                .map(|vox| !vox.is_empty())
                .unwrap_or(false)
            {
                pos.0.z += 0.05;
                vel.0.z = 0.0;
            }
        }
    }
}
