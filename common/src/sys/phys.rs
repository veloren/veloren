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

// Integrates forces, calculates the new velocity based off of the old velocity
// dt = delta time
// lv = linear velocity
// damp = linear damping
// Friction is a type of damping.
fn integrate_forces(dt: f32, mut lv: Vec3<f32>, damp: f32) -> Vec3<f32> {
    lv.z -= (GRAVITY * dt).max(-50.0);

    let mut linear_damp = 1.0 - dt * damp;

    if linear_damp < 0.0
    // reached zero in the given time
    {
        linear_damp = 0.0;
    }

    lv *= linear_damp;

    lv
}

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

            // Integrate forces
            // Friction is assumed to be a constant dependent on location
            let friction = 50.0 * if on_ground { 0.15 } else { 0.015 };
            vel.0 = integrate_forces(dt.0, vel.0, friction);

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
