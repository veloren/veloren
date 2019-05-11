use crate::{
    comp::phys::{Pos, Vel},
    state::DeltaTime,
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};
use specs::{Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

// Basic ECS physics system
pub struct Sys;

const GRAVITY: f32 = 9.81 * 4.0;

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
            vel.0.z = (vel.0.z - GRAVITY * dt.0).max(-50.0);

            // Movement
            pos.0 += vel.0 * dt.0;

            // Don't fall into the void
            // TODO: This shouldn't be needed when we have proper physics and chunk loading
            if pos.0.z < 0.0 {
                pos.0.z = 0.0;
                vel.0.z = 0.0;
            }

            // Basic collision with terrain
            let mut i = 0;
            while terrain
                .get(pos.0.map(|e| e.floor() as i32))
                .map(|vox| !vox.is_empty())
                .unwrap_or(false)
                && i < 100
            {
                pos.0.z += 0.0025;
                vel.0.z = 0.0;
                i += 1;
            }
        }
    }
}
