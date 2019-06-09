use crate::{
    comp::{
        phys::{Pos, Vel},
        OnGround, Stats,
    },
    state::DeltaTime,
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

const GRAVITY: f32 = 9.81 * 4.0;
const FRIC_GROUND: f32 = 0.15;
const FRIC_AIR: f32 = 0.015;

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

/// This system applies forces and calculates new positions and velocities
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, TerrainMap>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Stats>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, OnGround>,
    );

    fn run(
        &mut self,
        (entities, terrain, dt, stats, mut positions, mut velocities, mut on_grounds): Self::SystemData,
    ) {
        for (entity, stats, pos, vel) in (&entities, &stats, &mut positions, &mut velocities).join()
        {
            // Disable while dead TODO: Replace with client states
            if stats.is_dead {
                continue;
            }

            // Movement
            pos.0 += vel.0 * dt.0;

            // Update OnGround component
            if terrain
                .get((pos.0 - Vec3::unit_z() * 0.1).map(|e| e.floor() as i32))
                .map(|vox| !vox.is_empty())
                .unwrap_or(false)
                && vel.0.z <= 0.0
            {
                on_grounds.insert(entity, OnGround);
            } else {
                on_grounds.remove(entity);
            }

            // Integrate forces
            // Friction is assumed to be a constant dependent on location
            let friction = 50.0
                * if on_grounds.get(entity).is_some() {
                    FRIC_GROUND
                } else {
                    FRIC_AIR
                };
            vel.0 = integrate_forces(dt.0, vel.0, friction);

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
