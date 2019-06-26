use crate::{
    comp::{Gliding, Jumping, MoveDir, OnGround, Ori, Pos, Rolling, Stats, Vel},
    state::DeltaTime,
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

const GRAVITY: f32 = 9.81 * 4.0;
const FRIC_GROUND: f32 = 0.15;
const FRIC_AIR: f32 = 0.015;
const HUMANOID_ACCEL: f32 = 70.0;
const HUMANOID_SPEED: f32 = 120.0;
const HUMANOID_AIR_ACCEL: f32 = 10.0;
const HUMANOID_AIR_SPEED: f32 = 100.0;
const HUMANOID_JUMP_ACCEL: f32 = 16.0;
const ROLL_ACCEL: f32 = 160.0;
const ROLL_SPEED: f32 = 550.0;
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
        ReadStorage<'a, MoveDir>,
        ReadStorage<'a, Gliding>,
        ReadStorage<'a, Stats>,
        WriteStorage<'a, Jumping>,
        WriteStorage<'a, Rolling>,
        WriteStorage<'a, OnGround>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
    );

    fn run(
        &mut self,
        (
            entities,
            terrain,
            dt,
            move_dirs,
            glidings,
            stats,
            mut jumpings,
            mut rollings,
            mut on_grounds,
            mut positions,
            mut velocities,
            mut orientations,
        ): Self::SystemData,
    ) {
        // Apply movement inputs
        for (entity, stats, move_dir, gliding, mut pos, mut vel, mut ori) in (
            &entities,
            &stats,
            move_dirs.maybe(),
            glidings.maybe(),
            &mut positions,
            &mut velocities,
            &mut orientations,
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
                    * match (
                        on_grounds.get(entity).is_some(),
                        glidings.get(entity).is_some(),
                        rollings.get(entity).is_some(),
                    ) {
                        (true, false, false) if vel.0.magnitude() < HUMANOID_SPEED => {
                            HUMANOID_ACCEL
                        }
                        (false, true, false) if vel.0.magnitude() < GLIDE_SPEED => GLIDE_ACCEL,
                        (false, false, false) if vel.0.magnitude() < HUMANOID_AIR_SPEED => {
                            HUMANOID_AIR_ACCEL
                        }
                        (true, false, true) if vel.0.magnitude() < ROLL_SPEED => ROLL_ACCEL,

                        _ => 0.0,
                    };
            }

            // Jump
            if jumpings.get(entity).is_some() {
                vel.0.z = HUMANOID_JUMP_ACCEL;
                jumpings.remove(entity);
            }

            // Glide
            if gliding.is_some() && vel.0.magnitude() < GLIDE_SPEED && vel.0.z < 0.0 {
                let lift = GLIDE_ANTIGRAV + vel.0.z.powf(2.0) * 0.2;
                vel.0.z += dt.0 * lift * Vec2::<f32>::from(vel.0 * 0.15).magnitude().min(1.0);
            }

            // Roll
            if let Some(time) = rollings.get_mut(entity).map(|r| &mut r.time) {
                *time += dt.0;
                if *time > 0.55 {
                    rollings.remove(entity);
                }
            }

            // Set direction based on velocity
            if vel.0.magnitude_squared() != 0.0 {
                ori.0 = vel.0.normalized() * Vec3::new(1.0, 1.0, 0.0);
            }

            // Movement
            pos.0 += vel.0 * dt.0;

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
            let player_rad = 0.3; // half-width of the player's AABB
            let player_height = 1.7;

            let dist = 2;
            let near_iter = (-dist..=dist)
                .map(move |i| (-dist..=dist)
                    .map(move |j| (-dist..=dist)
                        .map(move |k| (i, j, k))))
                .flatten()
                .flatten();

            // Function for determining whether the player at a specific position collides with the ground
            let collision_with = |pos: Vec3<f32>, near_iter| {
                for (i, j, k) in near_iter {
                    let block_pos = pos.map(|e| e.floor() as i32) + Vec3::new(i, j, k);

                    if terrain
                        .get(block_pos)
                        .map(|vox| !vox.is_empty())
                        .unwrap_or(false)
                    {
                        let player_aabb = Aabb {
                            min: pos + Vec3::new(-player_rad, -player_rad, 0.0),
                            max: pos + Vec3::new(player_rad, player_rad, player_height),
                        };
                        let block_aabb = Aabb {
                            min: block_pos.map(|e| e as f32),
                            max: block_pos.map(|e| e as f32) + 1.0,
                        };

                        if player_aabb.collides_with_aabb(block_aabb) {
                            return true;
                        }
                    }
                }
                false
            };

            on_grounds.remove(entity);
            pos.0.z -= 0.0001; // To force collision with the floor

            let mut on_ground = false;
            let mut attempts = 0;
            while collision_with(pos.0, near_iter.clone()) && attempts < 32 {
                let player_aabb = Aabb {
                    min: pos.0 + Vec3::new(-player_rad, -player_rad, 0.0),
                    max: pos.0 + Vec3::new(player_rad, player_rad, player_height),
                };

                let (block_pos, block_aabb) = near_iter
                    .clone()
                    .map(|(i, j, k)| pos.0.map(|e| e.floor() as i32) + Vec3::new(i, j, k))
                    .map(|block_pos| {
                        let block_aabb = Aabb {
                            min: block_pos.map(|e| e as f32),
                            max: block_pos.map(|e| e as f32) + 1.0,
                        };

                        (block_pos, block_aabb)
                    })
                    .filter(|(_, block_aabb)| block_aabb.collides_with_aabb(player_aabb))
                    .filter(|(block_pos, _)| terrain
                        .get(*block_pos)
                        .map(|vox| !vox.is_empty())
                        .unwrap_or(false))
                    .max_by_key(|(_, block_aabb)| ((player_aabb.collision_vector_with_aabb(*block_aabb) / vel.0).map(|e| e.abs()).reduce_partial_min() * 1000.0) as i32)
                    .expect("Collision detected, but no colliding blocks found!");

                let dir = player_aabb.collision_vector_with_aabb(block_aabb);

                let max_axis = dir.map(|e| e.abs()).reduce_partial_min();
                let resolve_dir = -dir.map(|e| if e.abs() == max_axis { e } else { 0.0 });

                // When the resolution direction is pointing upwards, we must be on the ground
                if resolve_dir.z > 0.0 {
                    on_ground = true;
                }

                if resolve_dir.z == 0.0
                    && !collision_with(pos.0 + Vec3::unit_z() * 1.1, near_iter.clone())
                {
                    pos.0.z += 1.0;
                    on_ground = true;
                    break;
                } else {
                    pos.0 += resolve_dir;
                    vel.0 = vel
                        .0
                        .map2(resolve_dir, |e, d| if d == 0.0 { e } else { 0.0 });
                }

                attempts += 1;
            }

            if on_ground {
                on_grounds.insert(entity, OnGround);
            } else if collision_with(pos.0 - Vec3::unit_z() * 1.0, near_iter.clone())
                && vel.0.z < 0.0
                && vel.0.z > -1.0
            {
                pos.0.z = (pos.0.z - 0.05).floor();
                on_grounds.insert(entity, OnGround);
            }
        }
    }
}
