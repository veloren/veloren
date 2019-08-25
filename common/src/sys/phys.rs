use {
    crate::{
        comp::{Body, MovementState::*, Ori, PhysicsState, Pos, Scale, Stats, Vel},
        event::{ServerEvent, EventBus},
        state::DeltaTime,
        terrain::TerrainMap,
        vol::{ReadVol, Vox},
    },
    specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage},
    vek::*,
};

const GRAVITY: f32 = 9.81 * 4.0;
const FRIC_GROUND: f32 = 0.15;
const FRIC_AIR: f32 = 0.015;

// Integrates forces, calculates the new velocity based off of the old velocity
// dt = delta time
// lv = linear velocity
// damp = linear damping
// Friction is a type of damping.
fn integrate_forces(dt: f32, mut lv: Vec3<f32>, grav: f32, damp: f32) -> Vec3<f32> {
    lv.z = (lv.z - grav * dt).max(-50.0);

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
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Scale>,
        ReadStorage<'a, Body>,
        WriteStorage<'a, PhysicsState>,
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
            event_bus,
            scales,
            bodies,
            mut physics_states,
            mut positions,
            mut velocities,
            mut orientations,
        ): Self::SystemData,
    ) {
        let mut event_emitter = event_bus.emitter();

        // Apply movement inputs
        for (entity, scale, b, mut pos, mut vel, mut ori) in (
            &entities,
            scales.maybe(),
            &bodies,
            &mut positions,
            &mut velocities,
            &mut orientations,
        )
            .join()
        {
            let mut physics_state = physics_states.get(entity).cloned().unwrap_or_default();
            let scale = scale.map(|s| s.0).unwrap_or(1.0);

            // Integrate forces
            // Friction is assumed to be a constant dependent on location
            let friction = 50.0
                * if physics_state.on_ground {
                    FRIC_GROUND
                } else {
                    FRIC_AIR
                };
            vel.0 = integrate_forces(dt.0, vel.0, GRAVITY, friction);

            // Basic collision with terrain
            let player_rad = 0.3 * scale; // half-width of the player's AABB
            let player_height = 1.5 * scale;

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
                        .map(|vox| vox.is_solid())
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

            let was_on_ground = physics_state.on_ground;
            physics_state.on_ground = false;

            let mut on_ground = false;
            let mut attempts = 0; // Don't loop infinitely here

            // Don't move if we're not in a loaded chunk
            let pos_delta = if terrain
                .get_key(terrain.pos_key(pos.0.map(|e| e.floor() as i32)))
                .is_some()
            {
                vel.0 * dt.0
            } else {
                Vec3::zero()
            };

            // Don't jump too far at once
            let increments = (pos_delta.map(|e| e.abs()).reduce_partial_max() / 0.3)
                .ceil()
                .max(1.0);
            let old_pos = pos.0;
            for _ in 0..increments as usize {
                pos.0 += pos_delta / increments;

                const MAX_ATTEMPTS: usize = 16;

                // While the player is colliding with the terrain...
                while collision_with(pos.0, near_iter.clone()) && attempts < MAX_ATTEMPTS {
                    // Calculate the player's AABB
                    let player_aabb = Aabb {
                        min: pos.0 + Vec3::new(-player_rad, -player_rad, 0.0),
                        max: pos.0 + Vec3::new(player_rad, player_rad, player_height),
                    };

                    // Determine the block that we are colliding with most (based on minimum collision axis)
                    let (_block_pos, block_aabb) = near_iter
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
                                .map(|vox| vox.is_solid())
                                .unwrap_or(false)
                        })
                        // Find the maximum of the minimum collision axes (this bit is weird, trust me that it works)
                        .max_by_key(|(_, block_aabb)| {
                            ((player_aabb
                                .collision_vector_with_aabb(*block_aabb)
                                .map(|e| e.abs())
                                .product()
                                + block_aabb.min.z)
                                * 1_000_000.0) as i32
                        })
                        .expect("Collision detected, but no colliding blocks found!");

                    // Find the intrusion vector of the collision
                    let dir = player_aabb.collision_vector_with_aabb(block_aabb);

                    // Determine an appropriate resolution vector (i.e: the minimum distance needed to push out of the block)
                    let max_axis = dir.map(|e| e.abs()).reduce_partial_min();
                    let resolve_dir = -dir.map(|e| {
                        if e.abs().to_bits() == max_axis.to_bits() {
                            e
                        } else {
                            0.0
                        }
                    });

                    // When the resolution direction is pointing upwards, we must be on the ground
                    if resolve_dir.z > 0.0 && vel.0.z <= 0.0 {
                        on_ground = true;

                        if !was_on_ground {
                            event_emitter.emit(ServerEvent::LandOnGround { entity, vel: vel.0 });
                        }
                    }

                    // When the resolution direction is non-vertical, we must be colliding with a wall
                    // If the space above is free...
                    if !collision_with(Vec3::new(pos.0.x, pos.0.y, (pos.0.z + 0.1).ceil()), near_iter.clone())
                        // ...and we're being pushed out horizontally...
                        && resolve_dir.z == 0.0
                        // ...and the vertical resolution direction is sufficiently great...
                        && -dir.z > 0.1
                        // ...and we're falling/standing OR there is a block *directly* beneath our current origin (note: not hitbox)...
                        && (vel.0.z <= 0.0 || terrain
                            .get((pos.0 - Vec3::unit_z() * 0.1).map(|e| e.floor() as i32))
                            .map(|vox| vox.is_solid())
                            .unwrap_or(false))
                        // ...and there is a collision with a block beneath our current hitbox...
                        && collision_with(
                            old_pos + resolve_dir - Vec3::unit_z() * 1.05,
                            near_iter.clone(),
                        )
                    {
                        // ...block-hop!
                        pos.0.z = (pos.0.z + 0.1).ceil();
                        on_ground = true;
                        break;
                    } else {
                        // Correct the velocity
                        vel.0 = vel.0.map2(
                            resolve_dir,
                            |e, d| if d * e.signum() < 0.0 { 0.0 } else { e },
                        );
                    }

                    // Resolve the collision normally
                    pos.0 += resolve_dir;

                    attempts += 1;
                }

                if attempts == MAX_ATTEMPTS {
                    pos.0 = old_pos;
                    break;
                }
            }

            if on_ground {
                physics_state.on_ground = true;
            // If the space below us is free, then "snap" to the ground
            } else if collision_with(pos.0 - Vec3::unit_z() * 1.05, near_iter.clone())
                && vel.0.z < 0.0
                && vel.0.z > -1.5
                && was_on_ground
            {
                pos.0.z = (pos.0.z - 0.05).floor();
                physics_state.on_ground = true;
            }

            let _ = physics_states.insert(entity, physics_state);
        }

        // Apply pushback
        for (pos, scale, mut vel, _) in
            (&positions, scales.maybe(), &mut velocities, &bodies).join()
        {
            let scale = scale.map(|s| s.0).unwrap_or(1.0);
            for (pos_other, scale_other, _) in (&positions, scales.maybe(), &bodies).join() {
                let scale_other = scale_other.map(|s| s.0).unwrap_or(1.0);
                let diff = Vec2::<f32>::from(pos.0 - pos_other.0);

                let collision_dist = 0.95 * (scale + scale_other);

                if diff.magnitude_squared() > 0.0
                    && diff.magnitude_squared() < collision_dist.powf(2.0)
                    && pos.0.z + 1.6 * scale > pos_other.0.z
                    && pos.0.z < pos_other.0.z + 1.6 * scale_other
                {
                    vel.0 +=
                        Vec3::from(diff.normalized()) * (collision_dist - diff.magnitude()) * 1.0;
                }
            }
        }
    }
}
