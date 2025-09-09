use common::{
    comp::{
        CharacterState, Collider, Mass, Ori, PhysicsState, Pos, PreviousPhysCache, Scale, Vel,
        body::ship::figuredata::VoxelCollider,
        fluid_dynamics::{Fluid, LiquidKind},
    },
    consts::FRIC_GROUND,
    outcome::Outcome,
    resources::DeltaTime,
    terrain::{Block, BlockKind},
    uid::Uid,
    vol::{BaseVol, ReadVol},
};
use specs::Entity;
use std::ops::Range;
use vek::*;

use super::PhysicsRead;

#[expect(clippy::too_many_lines)]
pub(super) fn box_voxel_collision<T: BaseVol<Vox = Block> + ReadVol>(
    cylinder: (f32, f32, f32), // effective collision cylinder
    terrain: &T,
    entity: Entity,
    pos: &mut Pos,
    tgt_pos: Vec3<f32>,
    vel: &mut Vel,
    physics_state: &mut PhysicsState,
    dt: &DeltaTime,
    was_on_ground: bool,
    block_snap: bool,
    climbing: bool,
    mut land_on_ground: impl FnMut(Entity, Vel, Vec3<f32>),
    read: &PhysicsRead,
    ori: &Ori,
    // Get the proportion of surface friction that should be applied based on the current velocity
    friction_factor: impl Fn(Vec3<f32>) -> f32,
) {
    // We cap out scale at 10.0 to prevent an enormous amount of lag
    let scale = read.scales.get(entity).map_or(1.0, |s| s.0.min(10.0));

    //prof_span!("box_voxel_collision");

    // Convience function to compute the player aabb
    fn player_aabb(pos: Vec3<f32>, radius: f32, z_range: Range<f32>) -> Aabb<f32> {
        Aabb {
            min: pos + Vec3::new(-radius, -radius, z_range.start),
            max: pos + Vec3::new(radius, radius, z_range.end),
        }
    }

    // Convience function to translate the near_aabb into the world space
    fn move_aabb(aabb: Aabb<i32>, pos: Vec3<f32>) -> Aabb<i32> {
        Aabb {
            min: aabb.min + pos.map(|e| e.floor() as i32),
            max: aabb.max + pos.map(|e| e.floor() as i32),
        }
    }

    // Function for determining whether the player at a specific position collides
    // with blocks with the given criteria
    fn collision_with<T: BaseVol<Vox = Block> + ReadVol>(
        pos: Vec3<f32>,
        terrain: &T,
        near_aabb: Aabb<i32>,
        radius: f32,
        z_range: Range<f32>,
        move_dir: Vec3<f32>,
    ) -> bool {
        let player_aabb = player_aabb(pos, radius, z_range);

        // Calculate the world space near aabb
        let near_aabb = move_aabb(near_aabb, pos);

        let mut collision = false;
        // TODO: could short-circuit here
        terrain.for_each_in(near_aabb, |block_pos, block| {
            if block.is_solid() {
                let block_aabb = Aabb {
                    min: block_pos.map(|e| e as f32),
                    max: block_pos.map(|e| e as f32) + Vec3::new(1.0, 1.0, block.solid_height()),
                };
                if player_aabb.collides_with_aabb(block_aabb)
                    && block.valid_collision_dir(player_aabb, block_aabb, move_dir)
                {
                    collision = true;
                }
            }
        });

        collision
    }

    let (radius, z_min, z_max) = (Vec3::from(cylinder) * scale).into_tuple();

    // Probe distances
    let hdist = radius.ceil() as i32;

    // Neighbouring blocks Aabb
    let near_aabb = Aabb {
        min: Vec3::new(
            -hdist,
            -hdist,
            1 - Block::MAX_HEIGHT.ceil() as i32 + z_min.floor() as i32,
        ),
        max: Vec3::new(hdist, hdist, z_max.ceil() as i32),
    };

    let z_range = z_min..z_max;

    // Setup values for the loop below
    physics_state.on_ground = None;
    physics_state.on_ceiling = false;

    let mut on_ground = None::<Block>;
    let mut on_ceiling = false;
    // Don't loop infinitely here
    let mut attempts = 0;

    let mut pos_delta = tgt_pos - pos.0;

    // Don't jump too far at once
    const MAX_INCREMENTS: usize = 100; // The maximum number of collision tests per tick
    let min_step = (radius / 2.0).min(z_max - z_min).clamped(0.01, 0.3);
    let increments = ((pos_delta.map(|e| e.abs()).reduce_partial_max() / min_step).ceil() as usize)
        .clamped(1, MAX_INCREMENTS);
    let old_pos = pos.0;
    for _ in 0..increments {
        //prof_span!("increment");
        const MAX_ATTEMPTS: usize = 16;
        pos.0 += pos_delta / increments as f32;

        let vel2 = *vel;
        let try_colliding_block = |pos: &Pos| {
            //prof_span!("most colliding check");
            // Calculate the player's AABB
            let player_aabb = player_aabb(pos.0, radius, z_range.clone());

            // Determine the block that we are colliding with most
            // (based on minimum collision axis)
            // (if we are colliding with one)
            let mut most_colliding = None;
            // Calculate the world space near aabb
            let near_aabb = move_aabb(near_aabb, pos.0);
            let player_overlap = |block_aabb: Aabb<f32>| {
                (block_aabb.center() - player_aabb.center() - Vec3::unit_z() * 0.5)
                    .map(f32::abs)
                    .sum()
            };

            terrain.for_each_in(near_aabb, |block_pos, block| {
                // Make sure the block is actually solid
                if block.is_solid() {
                    // Calculate block AABB
                    let block_aabb = Aabb {
                        min: block_pos.map(|e| e as f32),
                        max: block_pos.map(|e| e as f32)
                            + Vec3::new(1.0, 1.0, block.solid_height()),
                    };

                    // Determine whether the block's AABB collides with the player's AABB
                    if player_aabb.collides_with_aabb(block_aabb)
                        && block.valid_collision_dir(player_aabb, block_aabb, vel2.0)
                    {
                        match &most_colliding {
                            // Select the minimum of the value from `player_overlap`
                            Some((_, other_block_aabb, _))
                                if player_overlap(block_aabb)
                                    >= player_overlap(*other_block_aabb) => {},
                            _ => most_colliding = Some((block_pos, block_aabb, block)),
                        }
                    }
                }
            });

            most_colliding
        };

        // While the player is colliding with the terrain...
        while let Some((_block_pos, block_aabb, block)) = (attempts < MAX_ATTEMPTS)
            .then(|| try_colliding_block(pos))
            .flatten()
        {
            // Calculate the player's AABB
            let player_aabb = player_aabb(pos.0, radius, z_range.clone());

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
            /* if resolve_dir.z > 0.0 && vel.0.z <= 0.0 { */
            if resolve_dir.z > 0.0 {
                on_ground = Some(block);
            } else if resolve_dir.z < 0.0 && vel.0.z >= 0.0 {
                on_ceiling = true;
            }

            // When the resolution direction is non-vertical, we must be colliding
            // with a wall
            //
            // If we're being pushed out horizontally...
            if resolve_dir.z == 0.0
            // ...and the vertical resolution direction is sufficiently great...
            && dir.z < -0.1
            // ...and the space above is free...
            && {
                //prof_span!("space above free");
                !collision_with(
                    Vec3::new(pos.0.x, pos.0.y, (pos.0.z + 0.1).ceil()),
                    &terrain,
                    near_aabb,
                    radius,
                    z_range.clone(),
                    vel.0,
                )
            }
            // ...and there is a collision with a block beneath our current hitbox...
            && {
                //prof_span!("collision beneath");
                collision_with(
                    pos.0 + resolve_dir - Vec3::unit_z() * 1.25,
                    &terrain,
                    near_aabb,
                    radius,
                    z_range.clone(),
                    vel.0,
                )
            } {
                // ...block-hop!
                pos.0.z = pos.0.z.max(block_aabb.max.z);

                // Apply fall damage, in the vertical axis, and correct velocity
                land_on_ground(entity, *vel, Vec3::unit_z());
                vel.0.z = vel.0.z.max(0.0);

                // Push the character on to the block very slightly
                // to avoid jitter due to imprecision
                if (vel.0 * resolve_dir).xy().magnitude_squared() < 1.0_f32.powi(2) {
                    pos.0 -= resolve_dir.normalized() * 0.05;
                }
                on_ground = Some(block);
                break;
            }

            // If not, correct the velocity, applying collision damage as we do
            if resolve_dir.magnitude_squared() > 0.0 {
                land_on_ground(entity, *vel, resolve_dir.normalized());
            }
            vel.0 = vel.0.map2(
                resolve_dir,
                |e, d| {
                    if d * e.signum() < 0.0 { 0.0 } else { e }
                },
            );

            pos_delta *= resolve_dir.map(|e| if e == 0.0 { 1.0 } else { 0.0 });

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

    // Report on_ceiling state
    if on_ceiling {
        physics_state.on_ceiling = true;
    }

    if on_ground.is_some() {
        physics_state.on_ground = on_ground;
    // If the space below us is free, then "snap" to the ground
    } else if vel.0.z <= 0.0
        && was_on_ground
        && block_snap
        && physics_state.in_liquid().is_none()
        && {
            //prof_span!("snap check");
            collision_with(
                pos.0 - Vec3::unit_z() * 1.1,
                &terrain,
                near_aabb,
                radius,
                z_range.clone(),
                vel.0,
            )
        }
    {
        //prof_span!("snap!!");
        let snap_height = terrain
            .get(Vec3::new(pos.0.x, pos.0.y, pos.0.z - 0.1).map(|e| e.floor() as i32))
            .ok()
            .filter(|block| block.is_solid())
            .map_or(0.0, Block::solid_height);
        vel.0.z = 0.0;
        pos.0.z = (pos.0.z - 0.1).floor() + snap_height;
        physics_state.on_ground = terrain
            .get(Vec3::new(pos.0.x, pos.0.y, pos.0.z - 0.01).map(|e| e.floor() as i32))
            .ok()
            .copied();
    }

    // Find liquid immersion and wall collision all in one round of iteration
    let player_aabb = player_aabb(pos.0, radius, z_range.clone());
    // Calculate the world space near_aabb
    let near_aabb = move_aabb(near_aabb, pos.0);

    let dirs = [
        Vec3::unit_x(),
        Vec3::unit_y(),
        -Vec3::unit_x(),
        -Vec3::unit_y(),
    ];

    // Compute a list of aabbs to check for collision with nearby walls
    let player_wall_aabbs = dirs.map(|dir| {
        let pos = pos.0 + dir * 0.01;
        Aabb {
            min: pos + Vec3::new(-radius, -radius, z_range.start),
            max: pos + Vec3::new(radius, radius, z_range.end),
        }
    });

    let mut liquid = None::<(LiquidKind, f32)>;
    let mut wall_dir_collisions = [false; 4];
    //prof_span!(guard, "liquid/walls");
    terrain.for_each_in(near_aabb, |block_pos, block| {
        // Check for liquid blocks
        if let Some(block_liquid) = block.liquid_kind() {
            let liquid_aabb = Aabb {
                min: block_pos.map(|e| e as f32),
                // The liquid part of a liquid block always extends 1 block high.
                max: block_pos.map(|e| e as f32) + Vec3::one(),
            };
            if player_aabb.collides_with_aabb(liquid_aabb) {
                liquid = match liquid {
                    Some((kind, max_liquid_z)) => Some((
                        // TODO: merging of liquid kinds and max_liquid_z are done
                        // independently which allows mix and
                        // matching them
                        kind.merge(block_liquid),
                        max_liquid_z.max(liquid_aabb.max.z),
                    )),
                    None => Some((block_liquid, liquid_aabb.max.z)),
                };
            }
        }

        // Check for walls
        if block.is_solid() {
            let block_aabb = Aabb {
                min: block_pos.map(|e| e as f32),
                max: block_pos.map(|e| e as f32) + Vec3::new(1.0, 1.0, block.solid_height()),
            };

            for dir in 0..4 {
                if player_wall_aabbs[dir].collides_with_aabb(block_aabb)
                    && block.valid_collision_dir(player_wall_aabbs[dir], block_aabb, vel.0)
                {
                    wall_dir_collisions[dir] = true;
                }
            }
        }
    });
    //drop(guard);

    // Use wall collision results to determine if we are against a wall
    let mut on_wall = None;
    for dir in 0..4 {
        if wall_dir_collisions[dir] {
            on_wall = Some(match on_wall {
                Some(acc) => acc + dirs[dir],
                None => dirs[dir],
            });
        }
    }

    physics_state.on_wall = on_wall;
    let fric_mod = read.stats.get(entity).map_or(1.0, |s| s.friction_modifier);

    physics_state.in_fluid = liquid
        .map(|(kind, max_z)| {
            // NOTE: assumes min_z == 0.0
            let depth = max_z - pos.0.z;

            // This is suboptimal because it doesn't check for true depth,
            // so it can cause problems for situations like swimming down
            // a river and spawning or teleporting in(/to) water
            let new_depth = physics_state.in_liquid().map_or(depth, |old_depth| {
                (old_depth + old_pos.z - pos.0.z).max(depth)
            });

            // TODO: Change this at some point to allow entities to be moved by liquids?
            let vel = Vel::zero();

            if depth > 0.0 {
                physics_state.ground_vel = vel.0;
            }

            Fluid::Liquid {
                kind,
                depth: new_depth,
                vel,
            }
        })
        .or_else(|| match physics_state.in_fluid {
            Some(Fluid::Liquid { .. }) | None => Some(Fluid::Air {
                elevation: pos.0.z,
                vel: Vel::default(),
            }),
            fluid => fluid,
        });

    // skating (ski)
    if !vel.0.xy().is_approx_zero()
        && physics_state
            .on_ground
            .is_some_and(|g| physics_state.footwear.can_skate_on(g.kind()))
    {
        const DT_SCALE: f32 = 1.0; // other areas use 60.0???
        const POTENTIAL_TO_KINETIC: f32 = 8.0; // * 2.0 * GRAVITY;

        let kind = physics_state.on_ground.map_or(BlockKind::Air, |g| g.kind());
        let (longitudinal_friction, lateral_friction) = physics_state.footwear.get_friction(kind);
        // the amount of longitudinal speed preserved
        let longitudinal_friction_factor_squared =
            (1.0 - longitudinal_friction).powf(dt.0 * DT_SCALE * 2.0);
        let lateral_friction_factor = (1.0 - lateral_friction).powf(dt.0 * DT_SCALE);
        let groundplane_velocity = vel.0.xy();
        let mut longitudinal_dir = ori.look_vec().xy();
        if longitudinal_dir.is_approx_zero() {
            // fall back to travelling dir (in case we look up)
            longitudinal_dir = groundplane_velocity;
        }
        let longitudinal_dir = longitudinal_dir.normalized();
        let lateral_dir = Vec2::new(longitudinal_dir.y, -longitudinal_dir.x);
        let squared_velocity = groundplane_velocity.magnitude_squared();
        // if we crossed an edge up or down accelerate in travelling direction,
        // as potential energy is converted into kinetic energy we compare it with the
        // square of velocity
        let vertical_difference = physics_state.skating_last_height - pos.0.z;
        // might become negative when skating slowly uphill
        let height_factor_squared = if vertical_difference != 0.0 {
            // E=½mv², we scale both energies by ½m
            let kinetic = squared_velocity;
            // positive accelerate, negative decelerate, ΔE=mgΔh
            let delta_potential = vertical_difference.clamp(-1.0, 2.0) * POTENTIAL_TO_KINETIC;
            let new_energy = kinetic + delta_potential;
            physics_state.skating_last_height = pos.0.z;
            new_energy / kinetic
        } else {
            1.0
        };

        // we calculate these squared as we need to combined them Euclidianly anyway,
        // skiing: separate speed into longitudinal and lateral component
        let long_speed = groundplane_velocity.dot(longitudinal_dir);
        let lat_speed = groundplane_velocity.dot(lateral_dir);
        let long_speed_squared = long_speed.powi(2);

        // lateral speed is reduced by lateral_friction,
        let new_lateral = lat_speed * lateral_friction_factor;
        let lateral_speed_reduction = lat_speed - new_lateral;
        // we convert this reduction partically (by the cosine of the angle) into
        // longitudinal (elastic collision) and the remainder into heat
        let cosine_squared_aoa = long_speed_squared / squared_velocity;
        let converted_lateral_squared = cosine_squared_aoa * lateral_speed_reduction.powi(2);
        let new_longitudinal_squared = longitudinal_friction_factor_squared
            * (long_speed_squared + converted_lateral_squared)
            * height_factor_squared;
        let new_longitudinal =
            new_longitudinal_squared.signum() * new_longitudinal_squared.abs().sqrt();
        let new_ground_speed = new_longitudinal * longitudinal_dir + new_lateral * lateral_dir;
        physics_state.skating_active = true;
        vel.0 = Vec3::new(new_ground_speed.x, new_ground_speed.y, 0.0);
    } else {
        let ground_fric = if physics_state.in_liquid().is_some() {
            // HACK:
            // If we're in a liquid, radically reduce ground friction (i.e: assume that
            // contact force is negligible due to buoyancy) Note that this might
            // not be realistic for very dense entities (currently no entities in Veloren
            // are sufficiently negatively buoyant for this to matter). We
            // should really make friction be proportional to net downward force, but
            // that means taking into account buoyancy which is a bit difficult to do here
            // for now.
            0.1
        } else {
            1.0
        } * physics_state
            .on_ground
            .map(|b| b.get_friction())
            .unwrap_or(0.0)
            * friction_factor(vel.0);
        let wall_fric = if physics_state.on_wall.is_some() && climbing {
            FRIC_GROUND
        } else {
            0.0
        };
        let fric = ground_fric.max(wall_fric);
        if fric > 0.0 {
            vel.0 *= (1.0 - fric.min(1.0) * fric_mod).powf(dt.0 * 60.0);
            physics_state.ground_vel = Vec3::zero();
        }
        physics_state.skating_active = false;
    }
}

pub(super) fn point_voxel_collision(
    entity: Entity,
    pos: &mut Pos,
    pos_delta: Vec3<f32>,
    vel: &mut Vel,
    physics_state: &mut PhysicsState,
    sticky: bool,
    outcomes: &mut Vec<Outcome>,
    read: &PhysicsRead,
) {
    // TODO: If the velocity is exactly 0,
    // a raycast may not pick up the current block.
    //
    // Handle this.
    let (dist, block) = if let Some(block) = read
        .terrain
        .get(pos.0.map(|e| e.floor() as i32))
        .ok()
        .filter(|b| b.is_solid())
    {
        (0.0, Some(block))
    } else {
        let (dist, block) = read
            .terrain
            .ray(pos.0, pos.0 + pos_delta)
            .until(|block: &Block| block.is_solid())
            .ignore_error()
            .cast();
        // Can't fail since we do ignore_error above
        (dist, block.unwrap())
    };

    pos.0 += pos_delta.try_normalized().unwrap_or_else(Vec3::zero) * dist;

    // TODO: Not all projectiles should count as sticky!
    if sticky
        && let Some((projectile, body)) = read
            .projectiles
            .get(entity)
            .filter(|_| vel.0.magnitude_squared() > 1.0 && block.is_some())
            .zip(read.bodies.get(entity).copied())
    {
        outcomes.push(Outcome::ProjectileHit {
            pos: pos.0 + pos_delta * dist,
            body,
            vel: vel.0,
            source: projectile.owner,
            target: None,
        });
    }

    if block.is_some() {
        let block_center = pos.0.map(|e| e.floor()) + 0.5;
        let block_rpos = (pos.0 - block_center)
            .try_normalized()
            .unwrap_or_else(Vec3::zero);

        // See whether we're on the top/bottom of a block,
        // or the side
        if block_rpos.z.abs() > block_rpos.xy().map(|e| e.abs()).reduce_partial_max() {
            if block_rpos.z > 0.0 {
                physics_state.on_ground = block.copied();
            } else {
                physics_state.on_ceiling = true;
            }
            vel.0.z = 0.0;
        } else {
            physics_state.on_wall = Some(if block_rpos.x.abs() > block_rpos.y.abs() {
                vel.0.x = 0.0;
                Vec3::unit_x() * -block_rpos.x.signum()
            } else {
                vel.0.y = 0.0;
                Vec3::unit_y() * -block_rpos.y.signum()
            });
        }

        // Sticky things shouldn't move
        if sticky {
            vel.0 = physics_state.ground_vel;
        }
    }

    physics_state.in_fluid = read
        .terrain
        .get(pos.0.map(|e| e.floor() as i32))
        .ok()
        .and_then(|vox| {
            vox.liquid_kind().map(|kind| Fluid::Liquid {
                kind,
                depth: 1.0,
                vel: Vel::zero(),
            })
        })
        .or_else(|| match physics_state.in_fluid {
            Some(Fluid::Liquid { .. }) | None => Some(Fluid::Air {
                elevation: pos.0.z,
                vel: Vel::default(),
            }),
            fluid => fluid,
        });
}

pub(super) fn voxel_collider_bounding_sphere(
    voxel_collider: &VoxelCollider,
    pos: &Pos,
    ori: &Ori,
    scale: Option<&Scale>,
) -> Sphere<f32, f32> {
    let origin_offset = voxel_collider.translation;
    use common::vol::SizedVol;
    let lower_bound = voxel_collider.volume().lower_bound().map(|e| e as f32);
    let upper_bound = voxel_collider.volume().upper_bound().map(|e| e as f32);
    let center = (lower_bound + upper_bound) / 2.0;
    // Compute vector from the origin (where pos value corresponds to) and the model
    // center
    let center_offset = center + origin_offset;
    // Rotate
    let oriented_center_offset = ori.local_to_global(center_offset);
    // Add to pos to get world coordinates of the center
    let wpos_center = oriented_center_offset + pos.0;

    // Note: to not get too fine grained we use a 2D grid for now
    const SPRITE_AND_MAYBE_OTHER_THINGS: f32 = 4.0;
    let radius = ((upper_bound - lower_bound) / 2.0
        + Vec3::broadcast(SPRITE_AND_MAYBE_OTHER_THINGS))
    .magnitude();

    Sphere {
        center: wpos_center,
        radius: radius * scale.map_or(1.0, |s| s.0),
    }
}

pub(super) struct ColliderData<'a> {
    pub pos: &'a Pos,
    pub previous_cache: &'a PreviousPhysCache,
    pub z_limits: (f32, f32),
    pub collider: &'a Collider,
    pub mass: Mass,
}

/// Returns whether interesction between entities occured
#[expect(clippy::too_many_arguments)]
pub(super) fn resolve_e2e_collision(
    // utility variables for our entity
    collision_registered: &mut bool,
    entity_entity_collisions: &mut u64,
    factor: f32,
    physics: &mut PhysicsState,
    char_state_maybe: Option<&CharacterState>,
    vel_delta: &mut Vec3<f32>,
    step_delta: f32,
    // physics flags
    is_mid_air: bool,
    is_sticky: bool,
    is_immovable: bool,
    is_projectile: bool,
    // entity we colliding with
    other: Uid,
    // symetrical collider context
    our_data: ColliderData,
    other_data: ColliderData,
    vel: &Vel,
    is_riding: bool,
) -> bool {
    // Find the distance betwen our collider and
    // collider we collide with and get vector of pushback.
    //
    // If we aren't colliding, just skip step.

    // Get positions
    let pos = our_data.pos.0 + our_data.previous_cache.velocity_dt * factor;
    let pos_other = other_data.pos.0 + other_data.previous_cache.velocity_dt * factor;

    // Compare Z ranges
    let (z_min, z_max) = our_data.z_limits;
    let ceiling = pos.z + z_max * our_data.previous_cache.scale;
    let floor = pos.z + z_min * our_data.previous_cache.scale;

    let (z_min_other, z_max_other) = other_data.z_limits;
    let ceiling_other = pos_other.z + z_max_other * other_data.previous_cache.scale;
    let floor_other = pos_other.z + z_min_other * other_data.previous_cache.scale;

    let in_z_range = ceiling >= floor_other && floor <= ceiling_other;

    if !in_z_range {
        return false;
    }

    let ours = ColliderContext {
        pos,
        previous_cache: our_data.previous_cache,
    };
    let theirs = ColliderContext {
        pos: pos_other,
        previous_cache: other_data.previous_cache,
    };
    let (diff, collision_dist) = projection_between(ours, theirs);
    let in_collision_range = diff.magnitude_squared() <= collision_dist.powi(2);

    if !in_collision_range {
        return false;
    }

    // If entities have not yet collided this tick (but just did) and if entity
    // is either in mid air or is not sticky, then mark them as colliding with
    // the other entity.
    if !*collision_registered && (is_mid_air || !is_sticky) {
        physics.touch_entities.insert(other, pos);
        *entity_entity_collisions += 1;
    }

    // Don't apply e2e pushback to entities that are in a forced movement state
    // (e.g. roll, leapmelee).
    //
    // This allows leaps to work properly (since you won't get pushed away
    // before delivering the hit), and allows rolling through an enemy when
    // trapped (e.g. with minotaur).
    //
    // This allows using e2e pushback to gain speed by jumping out of a roll
    // while in the middle of a collider, this is an intentional combat mechanic.
    let forced_movement =
        matches!(char_state_maybe, Some(cs) if cs.is_forced_movement()) || is_riding;

    // Don't apply repulsive force to projectiles,
    // or if we're colliding with a terrain-like entity,
    // or if we are a terrain-like entity.
    //
    // Don't apply force when entity is immovable, or a sticky which is on the
    // ground (or on the wall).
    if !forced_movement
        && (!is_sticky || is_mid_air)
        && diff.magnitude_squared() > 0.0
        && !is_projectile
        && !is_immovable
        && !other_data.collider.is_voxel()
        && !our_data.collider.is_voxel()
    {
        const ELASTIC_FORCE_COEFFICIENT: f32 = 400.0;
        let mass_coefficient = other_data.mass.0 / (our_data.mass.0 + other_data.mass.0);
        let distance_coefficient = collision_dist - diff.magnitude();
        let force = ELASTIC_FORCE_COEFFICIENT * distance_coefficient * mass_coefficient;

        let diff = diff.normalized();

        *vel_delta += Vec3::from(diff)
            * force
            * step_delta
            * vel
                .0
                .xy()
                .try_normalized()
                .map_or(1.0, |dir| diff.dot(-dir).max(0.025));
    }

    *collision_registered = true;

    true
}

struct ColliderContext<'a> {
    pos: Vec3<f32>,
    previous_cache: &'a PreviousPhysCache,
}

/// Find pushback vector and collision_distance we assume between this
/// colliders.
fn projection_between(c0: ColliderContext, c1: ColliderContext) -> (Vec2<f32>, f32) {
    const DIFF_THRESHOLD: f32 = f32::EPSILON;
    let our_radius = c0.previous_cache.neighborhood_radius;
    let their_radius = c1.previous_cache.neighborhood_radius;
    let collision_dist = our_radius + their_radius;

    let we = c0.pos.xy();
    let other = c1.pos.xy();

    let (p0_offset, p1_offset) = match c0.previous_cache.origins {
        Some(origins) => origins,
        // fallback to simpler model
        None => return capsule2cylinder(c0, c1),
    };
    let segment = LineSegment2 {
        start: we + p0_offset,
        end: we + p1_offset,
    };

    let (p0_offset_other, p1_offset_other) = match c1.previous_cache.origins {
        Some(origins) => origins,
        // fallback to simpler model
        None => return capsule2cylinder(c0, c1),
    };
    let segment_other = LineSegment2 {
        start: other + p0_offset_other,
        end: other + p1_offset_other,
    };

    let (our, their) = closest_points(segment, segment_other);
    let diff = our - their;

    if diff.magnitude_squared() < DIFF_THRESHOLD {
        capsule2cylinder(c0, c1)
    } else {
        (diff, collision_dist)
    }
}

/// Returns the points on line segments n and m respectively that are the
/// closest to one-another. If the lines are parallel, an arbitrary,
/// unspecified pair of points that sit on the line segments will be chosen.
fn closest_points(n: LineSegment2<f32>, m: LineSegment2<f32>) -> (Vec2<f32>, Vec2<f32>) {
    // TODO: Rewrite this to something reasonable, if you have faith
    let a = n.start;
    let b = n.end - n.start;
    let c = m.start;
    let d = m.end - m.start;

    // Check to prevent div by 0.0 (produces NaNs) and minimize precision
    // loss from dividing by small values.
    // If both d.x and d.y are 0.0 then the segment is a point and we are fine
    // to fallback to the end point projection.
    let t = if d.x > d.y {
        (d.y / d.x * (c.x - a.x) + a.y - c.y) / (b.x * d.y / d.x - b.y)
    } else {
        (d.x / d.y * (c.y - a.y) + a.x - c.x) / (b.y * d.x / d.y - b.x)
    };
    let u = if d.y > d.x {
        (a.y + t * b.y - c.y) / d.y
    } else {
        (a.x + t * b.x - c.x) / d.x
    };

    // Check to see whether the lines are parallel
    if !t.is_finite() || !u.is_finite() {
        [
            (n.projected_point(m.start), m.start),
            (n.projected_point(m.end), m.end),
            (n.start, m.projected_point(n.start)),
            (n.end, m.projected_point(n.end)),
        ]
        .into_iter()
        .min_by_key(|(a, b)| ordered_float::OrderedFloat(a.distance_squared(*b)))
        .expect("Lines had non-finite elements")
    } else {
        let t = t.clamped(0.0, 1.0);
        let u = u.clamped(0.0, 1.0);

        let close_n = a + b * t;
        let close_m = c + d * u;

        let proj_n = n.projected_point(close_m);
        let proj_m = m.projected_point(close_n);

        if proj_n.distance_squared(close_m) < proj_m.distance_squared(close_n) {
            (proj_n, close_m)
        } else {
            (close_n, proj_m)
        }
    }
}

/// Find pushback vector and collision_distance we assume between this
/// colliders assuming that only one of them is capsule prism.
fn capsule2cylinder(c0: ColliderContext, c1: ColliderContext) -> (Vec2<f32>, f32) {
    // "Proper" way to do this would be handle the case when both our colliders
    // are capsule prisms by building origins from p0, p1 offsets and our
    // positions and find some sort of projection between line segments of
    // both colliders.
    // While it's possible, it's not a trivial operation especially
    // in the case when they are intersect. Because in such case,
    // even when you found intersection and you should push entities back
    // from each other, you get then difference between them is 0 vector.
    //
    // Considering that we won't fully simulate collision of capsule prism.
    // As intermediate solution, we would assume that bigger collider
    // (with bigger scaled_radius) is capsule prism (cylinder is special
    // case of capsule prism too) and smaller collider is cylinder (point is
    // special case of cylinder).
    // So in the end our model of collision and pushback vector is simplified
    // to checking distance of the point between segment of capsule.
    //
    // NOTE: no matter if we consider our collider capsule prism or cylinder
    // we should always build pushback vector to have direction
    // of motion from our target collider to our collider.
    //
    let we = c0.pos.xy();
    let other = c1.pos.xy();
    let calculate_projection_and_collision_dist = |our_radius: f32,
                                                   their_radius: f32,
                                                   origins: Option<(Vec2<f32>, Vec2<f32>)>,
                                                   start_point: Vec2<f32>,
                                                   end_point: Vec2<f32>,
                                                   coefficient: f32|
     -> (Vec2<f32>, f32) {
        let collision_dist = our_radius + their_radius;

        let (p0_offset, p1_offset) = match origins {
            Some(origins) => origins,
            None => return (we - other, collision_dist),
        };
        let segment = LineSegment2 {
            start: start_point + p0_offset,
            end: start_point + p1_offset,
        };

        let projection = coefficient * (segment.projected_point(end_point) - end_point);

        (projection, collision_dist)
    };

    if c0.previous_cache.scaled_radius > c1.previous_cache.scaled_radius {
        calculate_projection_and_collision_dist(
            c0.previous_cache.neighborhood_radius,
            c1.previous_cache.scaled_radius,
            c0.previous_cache.origins,
            we,
            other,
            1.0,
        )
    } else {
        calculate_projection_and_collision_dist(
            c0.previous_cache.scaled_radius,
            c1.previous_cache.neighborhood_radius,
            c1.previous_cache.origins,
            other,
            we,
            -1.0,
        )
    }
}
