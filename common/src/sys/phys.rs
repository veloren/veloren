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
const HUMANOID_JUMP_ACCEL: f32 = 17.0;
const ROLL_ACCEL: f32 = 120.0;
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
    lv.z = (lv.z - GRAVITY * dt).max(-50.0);

    let linear_damp = (1.0 - dt * damp).max(0.0);

    lv * linear_damp
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
            if vel.0.magnitude_squared() > 0.1 {
                ori.0 = vel.0.normalized() * Vec3::new(1.0, 1.0, 0.0);
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
            let player_rad = 0.3f32; // half-width of the player's AABB
            let player_height = 1.5f32;

            // Probe distances
            let hdist = player_rad.ceil() as i32;
            let vdist = player_height.ceil() as i32;
            // Neighbouring blocks iterator
            let near_iter = (-hdist..=hdist)
                .map(move |i| (-hdist..=hdist).map(move |j| (0..=vdist).map(move |k| (i, j, k))))
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

            let was_on_ground = on_grounds.get(entity).is_some();
            on_grounds.remove(entity); // Assume we're in the air - unless we can prove otherwise
            pos.0.z -= 0.0001; // To force collision with the floor

            let mut on_ground = false;
            let mut attempts = 0; // Don't loop infinitely here

            // Don't jump too far at once
            let increments = ((vel.0 * dt.0).map(|e| e.abs()).reduce_partial_max() / 0.3)
                .ceil()
                .max(1.0);
            for _ in 0..increments as usize {
                pos.0 += vel.0 * dt.0 / increments;

                // While the player is colliding with the terrain...
                while collision_with(pos.0, near_iter.clone()) && attempts < 32 {
                    // Calculate the player's AABB
                    let player_aabb = Aabb {
                        min: pos.0 + Vec3::new(-player_rad, -player_rad, 0.0),
                        max: pos.0 + Vec3::new(player_rad, player_rad, player_height),
                    };

                    // Determine the block that we are colliding with most (based on minimum collision axis)
                    let (block_pos, block_aabb) = near_iter
                        .clone()
                        // Calculate the block's position in world space
                        .map(|(i, j, k)| pos.0.map(|e| e.floor() as i32) + Vec3::new(i, j, k))
                        // Calculate the AABB of the block
                        .map(|block_pos| {
                            (
                                block_pos,
                                Aabb {
                                    min: block_pos.map(|e| e as f32),
                                    max: block_pos.map(|e| e as f32) + 1.0,
                                },
                            )
                        })
                        // Determine whether the block's AABB collides with the player's AABB
                        .filter(|(_, block_aabb)| block_aabb.collides_with_aabb(player_aabb))
                        // Make sure the block is actually solid
                        .filter(|(block_pos, _)| {
                            terrain
                                .get(*block_pos)
                                .map(|vox| !vox.is_empty())
                                .unwrap_or(false)
                        })
                        // Find the maximum of the minimum collision axes (this bit is weird, trust me that it works)
                        .max_by_key(|(_, block_aabb)| {
                            ((player_aabb.collision_vector_with_aabb(*block_aabb) / vel.0)
                                .map(|e| e.abs())
                                .reduce_partial_min()
                                * 1000.0) as i32
                        })
                        .expect("Collision detected, but no colliding blocks found!");

                    // Find the intrusion vector of the collision
                    let dir = player_aabb.collision_vector_with_aabb(block_aabb);

                    // Determine an appropriate resolution vector (i.e: the minimum distance needed to push out of the block)
                    let max_axis = dir.map(|e| e.abs()).reduce_partial_min();
                    let resolve_dir = -dir.map(|e| if e.abs() == max_axis { e } else { 0.0 });

                    // When the resolution direction is pointing upwards, we must be on the ground
                    if resolve_dir.z > 0.0 && vel.0.z <= 0.0 {
                        on_ground = true;
                    }

                    // When the resolution direction is non-vertical, we must be colliding with a wall
                    // If the space above is free...
                    if !collision_with(pos.0 + Vec3::unit_z() * 1.1, near_iter.clone())
                        && resolve_dir.z == 0.0
                        && terrain
                            .get((pos.0 - Vec3::unit_z()).map(|e| e.floor() as i32)) // Make sure we're close to the ground
                            .map(|vox| !vox.is_empty())
                            .unwrap_or(false)
                    {
                        // ...block-hop!
                        pos.0.z = (pos.0.z + 1.0).ceil();
                        on_ground = true;
                        break;
                    } else {
                        // Resolve the collision normally
                        pos.0 += resolve_dir;
                        vel.0 = vel
                            .0
                            .map2(resolve_dir, |e, d| if d == 0.0 { e } else { 0.0 });
                    }

                    attempts += 1;
                }
            }

            if on_ground {
                on_grounds.insert(entity, OnGround);
            // If we're not on the ground but the space below us is free, then "snap" to the ground
            } else if collision_with(pos.0 - Vec3::unit_z() * 1.0, near_iter.clone())
                && vel.0.z < 0.0
                && vel.0.z > -1.0
                && was_on_ground
            {
                pos.0.z = (pos.0.z - 0.05).floor();
                on_grounds.insert(entity, OnGround);
            }
        }
    }
}
