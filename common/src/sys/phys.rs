use crate::{
    comp::{
        Collider, Gravity, Group, Mass, Mounting, Ori, PhysicsState, Pos, Projectile, Scale,
        Sticky, Vel,
    },
    event::{EventBus, ServerEvent},
    metrics::SysMetrics,
    span,
    state::DeltaTime,
    sync::{Uid, UidAllocator},
    terrain::{Block, BlockKind, TerrainGrid},
    vol::ReadVol,
};
use rayon::iter::ParallelIterator;
use specs::{
    saveload::MarkerAllocator, Entities, Join, ParJoin, Read, ReadExpect, ReadStorage, System,
    WriteStorage,
};
use std::ops::Range;
use vek::*;

pub const GRAVITY: f32 = 9.81 * 5.0;
const BOUYANCY: f32 = 1.0;
// Friction values used for linear damping. They are unitless quantities. The
// value of these quantities must be between zero and one. They represent the
// amount an object will slow down within 1/60th of a second. Eg. if the
// friction is 0.01, and the speed is 1.0, then after 1/60th of a second the
// speed will be 0.99. after 1 second the speed will be 0.54, which is 0.99 ^
// 60.
const FRIC_GROUND: f32 = 0.15;
const FRIC_AIR: f32 = 0.0125;
const FRIC_FLUID: f32 = 0.2;

// Integrates forces, calculates the new velocity based off of the old velocity
// dt = delta time
// lv = linear velocity
// damp = linear damping
// Friction is a type of damping.
fn integrate_forces(dt: f32, mut lv: Vec3<f32>, grav: f32, damp: f32) -> Vec3<f32> {
    // this is not linear damping, because it is proportional to the original
    // velocity this "linear" damping in in fact, quite exponential. and thus
    // must be interpolated accordingly
    let linear_damp = (1.0 - damp.min(1.0)).powf(dt * 60.0);

    lv.z = (lv.z - grav * dt).max(-80.0);
    lv * linear_damp
}

/// This system applies forces and calculates new positions and velocities.
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Uid>,
        ReadExpect<'a, TerrainGrid>,
        Read<'a, DeltaTime>,
        Read<'a, UidAllocator>,
        ReadExpect<'a, SysMetrics>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Scale>,
        ReadStorage<'a, Sticky>,
        ReadStorage<'a, Mass>,
        ReadStorage<'a, Collider>,
        ReadStorage<'a, Gravity>,
        WriteStorage<'a, PhysicsState>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        ReadStorage<'a, Mounting>,
        ReadStorage<'a, Group>,
        ReadStorage<'a, Projectile>,
    );

    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    #[allow(clippy::blocks_in_if_conditions)] // TODO: Pending review in #587
    fn run(
        &mut self,
        (
            entities,
            uids,
            terrain,
            dt,
            uid_allocator,
            sys_metrics,
            event_bus,
            scales,
            stickies,
            masses,
            colliders,
            gravities,
            mut physics_states,
            mut positions,
            mut velocities,
            mut orientations,
            mountings,
            groups,
            projectiles,
        ): Self::SystemData,
    ) {
        let start_time = std::time::Instant::now();
        span!(_guard, "run", "phys::Sys::run");
        let mut event_emitter = event_bus.emitter();

        // Add/reset physics state components
        span!(guard, "Add/reset physics state components");
        for (entity, _, _, _, _) in (
            &entities,
            &colliders,
            &positions,
            &velocities,
            &orientations,
        )
            .join()
        {
            let _ = physics_states
                .entry(entity)
                .map(|e| e.or_insert_with(Default::default));
        }
        drop(guard);

        // Apply pushback
        //
        // Note: We now do this first because we project velocity ahead. This is slighty
        // imperfect and implies that we might get edge-cases where entities
        // standing right next to the edge of a wall may get hit by projectiles
        // fired into the wall very close to them. However, this sort of thing is
        // already possible with poorly-defined hitboxes anyway so it's not too
        // much of a concern.
        //
        // If this situation becomes a problem, this code should be integrated with the
        // terrain collision code below, although that's not trivial to do since
        // it means the step needs to take into account the speeds of both
        // entities.
        span!(guard, "Apply pushback");
        for (entity, pos, scale, mass, collider, _, _, physics, projectile) in (
            &entities,
            &positions,
            scales.maybe(),
            masses.maybe(),
            colliders.maybe(),
            !&mountings,
            stickies.maybe(),
            &mut physics_states,
            // TODO: if we need to avoid collisions for other things consider moving whether it
            // should interact into the collider component or into a separate component
            projectiles.maybe(),
        )
            .join()
            .filter(|(_, _, _, _, _, _, sticky, physics, _)| {
                sticky.is_none() || (physics.on_wall.is_none() && !physics.on_ground)
            })
        {
            let scale = scale.map(|s| s.0).unwrap_or(1.0);
            let radius = collider.map(|c| c.get_radius()).unwrap_or(0.5);
            let z_limits = collider.map(|c| c.get_z_limits()).unwrap_or((-0.5, 0.5));
            let mass = mass.map(|m| m.0).unwrap_or(scale);

            // Group to ignore collisions with
            let ignore_group = projectile
                .filter(|p| p.ignore_group)
                .and_then(|p| p.owner)
                .and_then(|uid| uid_allocator.retrieve_entity_internal(uid.into()))
                .and_then(|e| groups.get(e));

            let mut vel_delta = Vec3::zero();

            for (
                entity_other,
                other,
                pos_other,
                scale_other,
                mass_other,
                collider_other,
                _,
                group,
            ) in (
                &entities,
                &uids,
                &positions,
                scales.maybe(),
                masses.maybe(),
                colliders.maybe(),
                !&mountings,
                groups.maybe(),
            )
                .join()
            {
                if entity == entity_other || (ignore_group.is_some() && ignore_group == group) {
                    continue;
                }

                let scale_other = scale_other.map(|s| s.0).unwrap_or(1.0);
                let radius_other = collider_other.map(|c| c.get_radius()).unwrap_or(0.5);
                let z_limits_other = collider_other
                    .map(|c| c.get_z_limits())
                    .unwrap_or((-0.5, 0.5));
                let mass_other = mass_other.map(|m| m.0).unwrap_or(scale_other);
                if mass_other == 0.0 {
                    continue;
                }

                let collision_dist = scale * radius + scale_other * radius_other;

                let vel = velocities.get(entity).copied().unwrap_or_default().0;
                let vel_other = velocities.get(entity_other).copied().unwrap_or_default().0;

                // Sanity check: don't try colliding entities that are too far from each other
                // Note: I think this catches all cases. If you get entity collision problems,
                // try removing this!
                if (pos.0 - pos_other.0).xy().magnitude()
                    > ((vel - vel_other) * dt.0).xy().magnitude() + collision_dist
                {
                    continue;
                }

                let min_collision_dist = 0.3;
                let increments = ((vel - vel_other).magnitude() * dt.0 / min_collision_dist)
                    .max(1.0)
                    .ceil() as usize;
                let step_delta = 1.0 / increments as f32;
                let mut collided = false;
                for i in 0..increments {
                    let factor = i as f32 * step_delta;
                    let pos = pos.0 + vel * dt.0 * factor;
                    let pos_other = pos_other.0 + vel_other * dt.0 * factor;

                    let diff = pos.xy() - pos_other.xy();

                    if diff.magnitude_squared() <= collision_dist.powf(2.0)
                        && pos.z + z_limits.1 * scale
                            >= pos_other.z + z_limits_other.0 * scale_other
                        && pos.z + z_limits.0 * scale
                            <= pos_other.z + z_limits_other.1 * scale_other
                    {
                        if !collided {
                            physics.touch_entities.push(*other);
                        }

                        if diff.magnitude_squared() > 0.0 {
                            let force = 400.0 * (collision_dist - diff.magnitude()) * mass_other
                                / (mass + mass_other);

                            vel_delta += Vec3::from(diff.normalized()) * force * step_delta;
                        }

                        collided = true;
                    }
                }
            }

            // Change velocity
            velocities
                .get_mut(entity)
                .map(|vel| vel.0 += vel_delta * dt.0);
        }
        drop(guard);

        // Apply movement inputs
        span!(guard, "Apply movement and terrain collision");
        let land_on_grounds = (
            &entities,
            scales.maybe(),
            stickies.maybe(),
            &colliders,
            &mut positions,
            &mut velocities,
            &mut orientations,
            &mut physics_states,
            !&mountings,
        )
        .par_join()
        .fold(Vec::new, |
            mut land_on_grounds,
            (entity, _scale, sticky, collider, mut pos, mut vel, _ori, mut physics_state, _),
        | {
            if sticky.is_some() && physics_state.on_surface().is_some() {
                vel.0 = Vec3::zero();
                return land_on_grounds;
            }

            // TODO: Use this
            //let scale = scale.map(|s| s.0).unwrap_or(1.0);

            let old_vel = *vel;
            // Integrate forces
            // Friction is assumed to be a constant dependent on location
            let friction = FRIC_AIR
                .max(if physics_state.on_ground {
                    FRIC_GROUND
                } else {
                    0.0
                })
                .max(if physics_state.in_fluid.is_some() {
                    FRIC_FLUID
                } else {
                    0.0
                });
            let in_loaded_chunk = terrain
                .get_key(terrain.pos_key(pos.0.map(|e| e.floor() as i32)))
                .is_some();
            let downward_force = if !in_loaded_chunk {
                0.0 // No gravity in unloaded chunks
            } else if physics_state
                .in_fluid
                .map(|depth| depth > 0.75)
                .unwrap_or(false)
            {
                (1.0 - BOUYANCY) * GRAVITY
            } else {
                GRAVITY
            } * gravities.get(entity).map(|g| g.0).unwrap_or_default();
            vel.0 = integrate_forces(dt.0, vel.0, downward_force, friction);

            // Don't move if we're not in a loaded chunk
            let mut pos_delta = if in_loaded_chunk {
                // this is an approximation that allows most framerates to
                // behave in a similar manner.
                let dt_lerp = 0.2;
                (vel.0 * dt_lerp + old_vel.0 * (1.0 - dt_lerp)) * dt.0
            } else {
                Vec3::zero()
            };

            match *collider {
                Collider::Box {
                    radius,
                    z_min,
                    z_max,
                } => {
                    // Scale collider
                    // TODO: Use scale & actual proportions when pathfinding is good enough to manage irregular entity
                    // sizes
                    let radius = radius.min(0.45); // * scale;
                    let z_min = z_min; // * scale;
                    let z_max = z_max.clamped(1.2, 1.95); // * scale;

                    // Probe distances
                    let hdist = radius.ceil() as i32;
                    // Neighbouring blocks iterator
                    let near_iter = (-hdist..hdist + 1)
                        .map(move |i| {
                            (-hdist..hdist + 1).map(move |j| {
                                (1 - BlockKind::MAX_HEIGHT.ceil() as i32 + z_min.floor() as i32
                                    ..z_max.ceil() as i32 + 1)
                                    .map(move |k| (i, j, k))
                            })
                        })
                        .flatten()
                        .flatten();

                    // Function for iterating over the blocks the player at a specific position
                    // collides with
                    fn collision_iter<'a>(
                        pos: Vec3<f32>,
                        terrain: &'a TerrainGrid,
                        hit: &'a impl Fn(&Block) -> bool,
                        near_iter: impl Iterator<Item = (i32, i32, i32)> + 'a,
                        radius: f32,
                        z_range: Range<f32>,
                    ) -> impl Iterator<Item = Aabb<f32>> + 'a {
                        near_iter.filter_map(move |(i, j, k)| {
                            let block_pos = pos.map(|e| e.floor() as i32) + Vec3::new(i, j, k);

                            if let Some(block) = terrain.get(block_pos).ok().copied().filter(hit) {
                                let player_aabb = Aabb {
                                    min: pos + Vec3::new(-radius, -radius, z_range.start),
                                    max: pos + Vec3::new(radius, radius, z_range.end),
                                };
                                let block_aabb = Aabb {
                                    min: block_pos.map(|e| e as f32),
                                    max: block_pos.map(|e| e as f32)
                                        + Vec3::new(1.0, 1.0, block.get_height()),
                                };

                                if player_aabb.collides_with_aabb(block_aabb) {
                                    return Some(block_aabb);
                                }
                            }

                            None
                        })
                    };

                    let z_range = z_min..z_max;
                    // Function for determining whether the player at a specific position collides
                    // with blocks with the given criteria
                    fn collision_with<'a>(
                        pos: Vec3<f32>,
                        terrain: &'a TerrainGrid,
                        hit: &impl Fn(&Block) -> bool,
                        near_iter: impl Iterator<Item = (i32, i32, i32)> + 'a,
                        radius: f32,
                        z_range: Range<f32>,
                    ) -> bool {
                        collision_iter(pos, terrain, hit, near_iter, radius, z_range).count()
                            > 0
                    };

                    let was_on_ground = physics_state.on_ground;
                    physics_state.on_ground = false;

                    let mut on_ground = false;
                    let mut on_ceiling = false;
                    let mut attempts = 0; // Don't loop infinitely here

                    // Don't jump too far at once
                    let increments = (pos_delta.map(|e| e.abs()).reduce_partial_max() / 0.3)
                        .ceil()
                        .max(1.0);
                    let old_pos = pos.0;
                    for _ in 0..increments as usize {
                        pos.0 += pos_delta / increments;

                        const MAX_ATTEMPTS: usize = 16;

                        // While the player is colliding with the terrain...
                        while collision_with(pos.0, &terrain, &|block| block.is_solid(), near_iter.clone(), radius, z_range.clone())
                            && attempts < MAX_ATTEMPTS
                        {
                            // Calculate the player's AABB
                            let player_aabb = Aabb {
                                min: pos.0 + Vec3::new(-radius, -radius, z_min),
                                max: pos.0 + Vec3::new(radius, radius, z_max),
                            };

                            // Determine the block that we are colliding with most (based on minimum
                            // collision axis)
                            let (_block_pos, block_aabb, block_height) = near_iter
                                .clone()
                                // Calculate the block's position in world space
                                .map(|(i, j, k)| pos.0.map(|e| e.floor() as i32) + Vec3::new(i, j, k))
                                // Make sure the block is actually solid
                                .filter_map(|block_pos| {
                                    if let Some(block) = terrain
                                        .get(block_pos)
                                        .ok()
                                        .filter(|block| block.is_solid())
                                    {
                                        // Calculate block AABB
                                        Some((
                                            block_pos,
                                            Aabb {
                                                min: block_pos.map(|e| e as f32),
                                                max: block_pos.map(|e| e as f32) + Vec3::new(1.0, 1.0, block.get_height()),
                                            },
                                            block.get_height(),
                                        ))
                                    } else {
                                        None
                                    }
                                })
                                // Determine whether the block's AABB collides with the player's AABB
                                .filter(|(_, block_aabb, _)| block_aabb.collides_with_aabb(player_aabb))
                                // Find the maximum of the minimum collision axes (this bit is weird, trust me that it works)
                                .min_by_key(|(_, block_aabb, _)| {
                                    ((block_aabb.center() - player_aabb.center() - Vec3::unit_z() * 0.5)
                                        .map(|e| e.abs())
                                        .sum()
                                        * 1_000_000.0) as i32
                                })
                                .expect("Collision detected, but no colliding blocks found!");

                            // Find the intrusion vector of the collision
                            let dir = player_aabb.collision_vector_with_aabb(block_aabb);

                            // Determine an appropriate resolution vector (i.e: the minimum distance
                            // needed to push out of the block)
                            let max_axis = dir.map(|e| e.abs()).reduce_partial_min();
                            let resolve_dir = -dir.map(|e| {
                                if e.abs().to_bits() == max_axis.to_bits() {
                                    e
                                } else {
                                    0.0
                                }
                            });

                            // When the resolution direction is pointing upwards, we must be on the
                            // ground
                            if resolve_dir.z > 0.0 && vel.0.z <= 0.0 {
                                on_ground = true;

                                if !was_on_ground {
                                    land_on_grounds.push((entity, *vel));
                                }
                            } else if resolve_dir.z < 0.0 && vel.0.z >= 0.0 {
                                on_ceiling = true;
                            }

                            // When the resolution direction is non-vertical, we must be colliding
                            // with a wall If the space above is free...
                            if !collision_with(Vec3::new(pos.0.x, pos.0.y, (pos.0.z + 0.1).ceil()), &terrain, &|block| block.is_solid(), near_iter.clone(), radius, z_range.clone())
                                // ...and we're being pushed out horizontally...
                                && resolve_dir.z == 0.0
                                // ...and the vertical resolution direction is sufficiently great...
                                && -dir.z > 0.1
                                // ...and we're falling/standing OR there is a block *directly* beneath our current origin (note: not hitbox)...
                                && (vel.0.z <= 0.0 || terrain
                                    .get((pos.0 - Vec3::unit_z() * 0.1).map(|e| e.floor() as i32))
                                    .map(|block| block.is_solid())
                                    .unwrap_or(false))
                                // ...and there is a collision with a block beneath our current hitbox...
                                && collision_with(
                                    pos.0 + resolve_dir - Vec3::unit_z() * 1.05,
                                    &terrain,
                                    &|block| block.is_solid(),
                                    near_iter.clone(),
                                    radius,
                                    z_range.clone(),
                                )
                            {
                                // ...block-hop!
                                pos.0.z = (pos.0.z + 0.1).floor() + block_height;
                                vel.0.z = 0.0;
                                on_ground = true;
                                break;
                            } else {
                                // Correct the velocity
                                vel.0 = vel.0.map2(resolve_dir, |e, d| {
                                    if d * e.signum() < 0.0 { 0.0 } else { e }
                                });
                                pos_delta *= resolve_dir.map(|e| if e != 0.0 { 0.0 } else { 1.0 });
                            }

                            // Resolve the collision normally
                            pos.0 += resolve_dir;

                            attempts += 1;
                        }

                        if attempts == MAX_ATTEMPTS {
                            vel.0 = Vec3::zero();
                            pos.0 = old_pos;
                            break;
                        }
                    }

                    if on_ceiling {
                        physics_state.on_ceiling = true;
                    }

                    if on_ground {
                        physics_state.on_ground = true;
                    // If the space below us is free, then "snap" to the ground
                    } else if collision_with(
                        pos.0 - Vec3::unit_z() * 1.05,
                        &terrain,
                        &|block| block.is_solid(),
                        near_iter.clone(),
                        radius,
                        z_range.clone(),
                    ) && vel.0.z < 0.0
                        && vel.0.z > -1.5
                        && was_on_ground
                        && !collision_with(
                            pos.0 - Vec3::unit_z() * 0.05,
                            &terrain,
                            &|block| {
                                block.is_solid()
                                    && block.get_height() >= (pos.0.z - 0.05).rem_euclid(1.0)
                            },
                            near_iter.clone(),
                            radius,
                            z_range.clone(),
                        )
                    {
                        let snap_height = terrain
                            .get(
                                Vec3::new(pos.0.x, pos.0.y, pos.0.z - 0.05)
                                    .map(|e| e.floor() as i32),
                            )
                            .ok()
                            .filter(|block| block.is_solid())
                            .map(|block| block.get_height())
                            .unwrap_or(0.0);
                        pos.0.z = (pos.0.z - 0.05).floor() + snap_height;
                        physics_state.on_ground = true;
                    }

                    let dirs = [
                        Vec3::unit_x(),
                        Vec3::unit_y(),
                        -Vec3::unit_x(),
                        -Vec3::unit_y(),
                    ];

                    if let (wall_dir, true) =
                        dirs.iter().fold((Vec3::zero(), false), |(a, hit), dir| {
                            if collision_with(
                                pos.0 + *dir * 0.01,
                                &terrain,
                                &|block| block.is_solid(),
                                near_iter.clone(),
                                radius,
                                z_range.clone(),
                            ) {
                                (a + dir, true)
                            } else {
                                (a, hit)
                            }
                        })
                    {
                        physics_state.on_wall = Some(wall_dir);
                    } else {
                        physics_state.on_wall = None;
                    }

                    // Figure out if we're in water
                    physics_state.in_fluid = collision_iter(
                        pos.0,
                        &terrain,
                        &|block| block.is_fluid(),
                        near_iter.clone(),
                        radius,
                        z_min..z_max,
                    )
                    .max_by_key(|block_aabb| (block_aabb.max.z * 100.0) as i32)
                    .map(|block_aabb| block_aabb.max.z - pos.0.z);
                },
                Collider::Point => {
                    let (dist, block) = terrain.ray(pos.0, pos.0 + pos_delta).ignore_error().cast();

                    pos.0 += pos_delta.try_normalized().unwrap_or(Vec3::zero()) * dist;

                    // Can't fair since we do ignore_error above
                    if block.unwrap().is_some() {
                        let block_center = pos.0.map(|e| e.floor()) + 0.5;
                        let block_rpos = (pos.0 - block_center)
                            .try_normalized()
                            .unwrap_or(Vec3::zero());

                        // See whether we're on the top/bottom of a block, or the side
                        if block_rpos.z.abs()
                            > block_rpos.xy().map(|e| e.abs()).reduce_partial_max()
                        {
                            if block_rpos.z > 0.0 {
                                physics_state.on_ground = true;
                            } else {
                                physics_state.on_ceiling = true;
                            }
                            vel.0.z = 0.0;
                        } else {
                            physics_state.on_wall =
                                Some(if block_rpos.x.abs() > block_rpos.y.abs() {
                                    vel.0.x = 0.0;
                                    Vec3::unit_x() * -block_rpos.x.signum()
                                } else {
                                    vel.0.y = 0.0;
                                    Vec3::unit_y() * -block_rpos.y.signum()
                                });
                        }
                    }
                },
            }

            land_on_grounds
        }).reduce(Vec::new, |mut land_on_grounds_a, mut land_on_grounds_b| {
            land_on_grounds_a.append(&mut land_on_grounds_b);
            land_on_grounds_a
        });
        drop(guard);

        land_on_grounds.into_iter().for_each(|(entity, vel)| {
            event_emitter.emit(ServerEvent::LandOnGround { entity, vel: vel.0 });
        });
        sys_metrics.phys_ns.store(
            start_time.elapsed().as_nanos() as i64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}
