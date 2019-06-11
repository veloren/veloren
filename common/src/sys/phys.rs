use crate::{
    comp::{
        phys::{Ori, Pos, Vel},
        Gliding, Jumping, MoveDir, OnGround, Stats, Rolling, Cidling, Crunning,
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
const HUMANOID_ACCEL: f32 = 100.0;
const HUMANOID_SPEED: f32 = 500.0;
const HUMANOID_AIR_ACCEL: f32 = 10.0;
const HUMANOID_AIR_SPEED: f32 = 100.0;
const HUMANOID_JUMP_ACCEL: f32 = 16.0;
const GLIDE_ACCEL: f32 = 15.0;
const GLIDE_SPEED: f32 = 45.0;
// Gravity is 9.81 * 4, so this makes gravity equal to .15
const GLIDE_ANTIGRAV: f32 = 9.81 * 3.95;

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

/// This system applies forces and calculates new positions and velocities.
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, TerrainMap>,
        Read<'a, DeltaTime>,
        WriteStorage<'a, OnGround>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        ReadStorage<'a, MoveDir>,
        ReadStorage<'a, Jumping>,
        ReadStorage<'a, Gliding>,
        ReadStorage<'a, Rolling>,
        ReadStorage<'a, Crunning>,
        ReadStorage<'a, Cidling>,
        ReadStorage<'a, Stats>,
    );

    fn run(
        &mut self,
        (
            entities,
            terrain,
            dt,
            mut on_grounds,
            mut positions,
            mut velocities,
            mut orientations,
            move_dirs,
            jumpings,
            glidings,
            rollings,
            crunnings,
            cidlings,
            stats,
        ): Self::SystemData,
    ) {
        // Apply movement inputs
        for (entity, mut pos, mut vel, mut ori, mut on_ground, move_dir, jumping, gliding, rolling, crunning, cidling, stats) in
            (
                &entities,
                &mut positions,
                &mut velocities,
                &mut orientations,
                on_grounds.maybe(),
                move_dirs.maybe(),
                jumpings.maybe(),
                glidings.maybe(),
                rollings.maybe(),
                crunnings.maybe(),
                cidlings.maybe(),
                &stats,
            )
                .join()
        {
            // Disable while dead TODO: Replace with client states?
            if stats.is_dead {
                continue;
            }

            // Move player according to move_dir
            if let Some(move_dir) = move_dir {
                vel.0 += Vec2::broadcast(dt.0)
                    * move_dir.0
                    * match (on_ground.is_some(), gliding.is_some()) {
                        (true, false) if vel.0.magnitude() < HUMANOID_SPEED => HUMANOID_ACCEL,
                        (false, true) if vel.0.magnitude() < GLIDE_SPEED => GLIDE_ACCEL,
                        (false, false) if vel.0.magnitude() < HUMANOID_AIR_SPEED => {
                            HUMANOID_AIR_ACCEL
                        }
                        _ => 0.0,
                    };
            }

            // Jump
            if jumping.is_some() {
                vel.0.z = HUMANOID_JUMP_ACCEL;
            }

            // Glide
            if gliding.is_some() && vel.0.magnitude() < GLIDE_SPEED && vel.0.z < 0.0 {
                let lift = GLIDE_ANTIGRAV + vel.0.z.powf(2.0) * 0.2;
                vel.0.z += dt.0 * lift * Vec2::<f32>::from(vel.0 * 0.15).magnitude().min(1.0);
            }

            // TODO:
            if rolling.is_some() {}
            if crunning.is_some() {}
            if cidling.is_some() {}

            // Set direction based on velocity
            if vel.0.magnitude_squared() != 0.0 {
                ori.0 = vel.0.normalized() * Vec3::new(1.0, 1.0, 0.0);
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
                on_ground = Some(&OnGround);
            } else {
                on_ground = None;
            }

            // Integrate forces
            // Friction is assumed to be a constant dependent on location
            let friction = 50.0
                * if on_ground.is_some() {
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
