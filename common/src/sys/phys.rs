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

            // Handle held-down control
            let on_ground = terrain
                .get((pos.0 - Vec3::unit_z() * 0.1).map(|e| e.floor() as i32))
                .map(|vox| !vox.is_empty())
                .unwrap_or(false)
                && vel.0.z <= 0.0;

            // Friction
            // Will never make the character go backwards since it is interpolating towards 0
            let friction = if on_ground { 0.15 } else { 0.015 };
            let mul = 50.0;
            let z = vel.0.z;
            vel.0 = Vec3::lerp(vel.0, Vec3::new(0.0, 0.0, 0.0), friction * dt.0 * mul);
            vel.0.z = z;

            // Gravity
            vel.0.z = (vel.0.z - GRAVITY * dt.0).max(-50.0);

            // Movement
            pos.0 += vel.0 * dt.0;

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
