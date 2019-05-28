use crate::{
    comp::{
        phys::{Pos, Vel},
        Stats,
    },
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
        ReadStorage<'a, Stats>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
    );

    fn run(&mut self, (terrain, dt, stats, mut positions, mut velocities): Self::SystemData) {
        for (stats, pos, vel) in (&stats, &mut positions, &mut velocities).join() {
            // Disable while dead TODO: Replace with client states
            if stats.is_dead {
                continue;
            }

            // Gravity
            vel.0.z = (vel.0.z - GRAVITY * dt.0).max(-50.0);

            // Movement
            pos.0 += vel.0 * dt.0;

            // Don't fall into the void.
            // TODO: This shouldn't be needed when we have proper physics and chunk loading.
            if pos.0.z < 0.0 {
                pos.0.z = 0.0;
                vel.0.z = 0.0;
            }

            // Basic collision with terrain
            let mut i = 0.0;
            while terrain
                .get(pos.0.map(|e| e.floor() as i32))
                .map(|vox| !vox.is_empty())
                .unwrap_or(false)
                && i < 6000.0 * dt.0
            {
                pos.0.z += 0.0025;
                vel.0.z = 0.0;
                i += 1.0;
            }
        }
    }
}
