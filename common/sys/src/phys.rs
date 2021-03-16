mod spatial_grid;

use spatial_grid::SpatialGrid;

use common::{
    comp::{
        body::ship::figuredata::VOXEL_COLLIDER_MANIFEST, BeamSegment, Body, CharacterState,
        Collider, Gravity, Mass, Mounting, Ori, PhysicsState, Pos, PreviousPhysCache, Projectile,
        Scale, Shockwave, Sticky, Vel,
    },
    consts::{FRIC_GROUND, GRAVITY},
    event::{EventBus, ServerEvent},
    resources::DeltaTime,
    terrain::{Block, TerrainGrid},
    uid::Uid,
    vol::{BaseVol, ReadVol},
};
use common_base::{prof_span, span};
use common_ecs::{Job, Origin, ParMode, Phase, PhysicsMetrics, System};
use hashbrown::HashMap;
use rayon::iter::ParallelIterator;
use specs::{
    shred::{ResourceId, World},
    Entities, Entity, Join, ParJoin, Read, ReadExpect, ReadStorage, SystemData, WriteExpect,
    WriteStorage,
};
use std::ops::Range;
use vek::*;

pub const BOUYANCY: f32 = 1.0;
// Friction values used for linear damping. They are unitless quantities. The
// value of these quantities must be between zero and one. They represent the
// amount an object will slow down within 1/60th of a second. Eg. if the
// friction is 0.01, and the speed is 1.0, then after 1/60th of a second the
// speed will be 0.99. after 1 second the speed will be 0.54, which is 0.99 ^
// 60.
pub const FRIC_AIR: f32 = 0.0025;
pub const FRIC_FLUID: f32 = 0.4;

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

    // TODO: investigate if we can have air friction provide the neccessary limits
    // here
    lv.z = (lv.z - grav * dt).max(-80.0).min(lv.z);
    lv * linear_damp
}

fn calc_z_limit(
    char_state_maybe: Option<&CharacterState>,
    collider: Option<&Collider>,
) -> (f32, f32) {
    let modifier = if char_state_maybe.map_or(false, |c_s| c_s.is_dodge()) {
        0.5
    } else {
        1.0
    };
    collider
        .map(|c| c.get_z_limits(modifier))
        .unwrap_or((-0.5 * modifier, 0.5 * modifier))
}

/// This system applies forces and calculates new positions and velocities.
#[derive(Default)]
pub struct Sys;

#[derive(SystemData)]
pub struct PhysicsRead<'a> {
    entities: Entities<'a>,
    uids: ReadStorage<'a, Uid>,
    terrain: ReadExpect<'a, TerrainGrid>,
    dt: Read<'a, DeltaTime>,
    event_bus: Read<'a, EventBus<ServerEvent>>,
    scales: ReadStorage<'a, Scale>,
    stickies: ReadStorage<'a, Sticky>,
    masses: ReadStorage<'a, Mass>,
    colliders: ReadStorage<'a, Collider>,
    gravities: ReadStorage<'a, Gravity>,
    mountings: ReadStorage<'a, Mounting>,
    projectiles: ReadStorage<'a, Projectile>,
    beams: ReadStorage<'a, BeamSegment>,
    shockwaves: ReadStorage<'a, Shockwave>,
    char_states: ReadStorage<'a, CharacterState>,
    bodies: ReadStorage<'a, Body>,
    character_states: ReadStorage<'a, CharacterState>,
}

#[derive(SystemData)]
pub struct PhysicsWrite<'a> {
    physics_metrics: WriteExpect<'a, PhysicsMetrics>,
    physics_states: WriteStorage<'a, PhysicsState>,
    positions: WriteStorage<'a, Pos>,
    velocities: WriteStorage<'a, Vel>,
    orientations: WriteStorage<'a, Ori>,
    previous_phys_cache: WriteStorage<'a, PreviousPhysCache>,
}

#[derive(SystemData)]
pub struct PhysicsData<'a> {
    read: PhysicsRead<'a>,
    write: PhysicsWrite<'a>,
}

impl<'a> PhysicsData<'a> {
    /// Add/reset physics state components
    fn reset(&mut self) {
        span!(_guard, "Add/reset physics state components");
        for (entity, _, _, _, _) in (
            &self.read.entities,
            &self.read.colliders,
            &self.write.positions,
            &self.write.velocities,
            &self.write.orientations,
        )
            .join()
        {
            let _ = self
                .write
                .physics_states
                .entry(entity)
                .map(|e| e.or_insert_with(Default::default));
        }
    }

    fn maintain_pushback_cache(&mut self) {
        span!(_guard, "Maintain pushback cache");
        // Add PreviousPhysCache for all relevant entities
        for entity in (
            &self.read.entities,
            &self.write.velocities,
            &self.write.positions,
            !&self.write.previous_phys_cache,
            !&self.read.mountings,
            !&self.read.beams,
            !&self.read.shockwaves,
        )
            .join()
            .map(|(e, _, _, _, _, _, _)| e)
            .collect::<Vec<_>>()
        {
            let _ = self
                .write
                .previous_phys_cache
                .insert(entity, PreviousPhysCache {
                    velocity_dt: Vec3::zero(),
                    center: Vec3::zero(),
                    collision_boundary: 0.0,
                    scale: 0.0,
                    scaled_radius: 0.0,
                    ori: Quaternion::identity(),
                });
        }

        // Update PreviousPhysCache
        for (_, vel, position, mut phys_cache, collider, scale, cs, _, _, _) in (
            &self.read.entities,
            &self.write.velocities,
            &self.write.positions,
            &mut self.write.previous_phys_cache,
            self.read.colliders.maybe(),
            self.read.scales.maybe(),
            self.read.char_states.maybe(),
            !&self.read.mountings,
            !&self.read.beams,
            !&self.read.shockwaves,
        )
            .join()
        {
            let scale = scale.map(|s| s.0).unwrap_or(1.0);
            let z_limits = calc_z_limit(cs, collider);
            let z_limits = (z_limits.0 * scale, z_limits.1 * scale);
            let half_height = (z_limits.1 - z_limits.0) / 2.0;

            phys_cache.velocity_dt = vel.0 * self.read.dt.0;
            let entity_center = position.0 + Vec3::new(0.0, z_limits.0 + half_height, 0.0);
            let flat_radius = collider.map(|c| c.get_radius()).unwrap_or(0.5) * scale;
            let radius = (flat_radius.powi(2) + half_height.powi(2)).sqrt();

            // Move center to the middle between OLD and OLD+VEL_DT so that we can reduce
            // the collision_boundary
            phys_cache.center = entity_center + phys_cache.velocity_dt / 2.0;
            phys_cache.collision_boundary = radius + (phys_cache.velocity_dt / 2.0).magnitude();
            phys_cache.scale = scale;
            phys_cache.scaled_radius = flat_radius;
        }
    }

    fn construct_spatial_grid(&mut self) -> SpatialGrid {
        span!(_guard, "Construct spatial grid");
        let PhysicsData {
            ref read,
            ref write,
        } = self;
        // NOTE: assumes that entity max radius * 2 + max velocity per tick is less than
        // half a chunk (16 blocks)
        // NOTE: i32 places certain constraints on how far out collision works
        // NOTE: uses the radius of the entity and their current position rather than
        // the radius of their bounding sphere for the current frame of movement
        // because the nonmoving entity is what is collided against in the inner
        // loop of the pushback collision code
        // TODO: maintain frame to frame? (requires handling deletion)
        // TODO: if not maintaining frame to frame consider counting entities to
        // preallocate?
        // TODO: assess parallelizing (overhead might dominate here? would need to merge
        // the vecs in each hashmap)
        let lg2_cell_size = inline_tweak::release_tweak!(5);
        let lg2_large_cell_size = 6;
        let radius_cutoff = 8;
        common_base::plot!("spatial grid cell size", (1 << lg2_cell_size) as f64);
        let mut spatial_grid = SpatialGrid::new(lg2_cell_size, lg2_large_cell_size, radius_cutoff);
        for (entity, pos, phys_cache, _, _, _, _, _) in (
            &read.entities,
            &write.positions,
            &write.previous_phys_cache,
            write.velocities.mask(),
            !&read.projectiles, // Not needed because they are skipped in the inner loop below
            !&read.mountings,
            !&read.beams,
            !&read.shockwaves,
        )
            .join()
        {
            // Note: to not get too fine grained we use a 2D grid for now
            let radius_2d = phys_cache.scaled_radius.ceil() as u32;
            let pos_2d = pos.0.xy().map(|e| e as i32);
            const POS_TRUNCATION_ERROR: u32 = 1;
            spatial_grid.insert(pos_2d, radius_2d + POS_TRUNCATION_ERROR, entity);
        }

        spatial_grid
    }

    fn apply_pushback(&mut self, job: &mut Job<Sys>, spatial_grid: &SpatialGrid) {
        // TODO: make sure to check git stash show -p  to make sure nothing was missed
        span!(_guard, "Apply pushback");
        let use_grid = inline_tweak::release_tweak!(true);
        common_base::plot!("use grid", if use_grid { 1.0 } else { 0.0 });
        job.cpu_stats.measure(ParMode::Rayon);
        let PhysicsData {
            ref read,
            ref mut write,
        } = self;
        let (positions, previous_phys_cache) = (&write.positions, &write.previous_phys_cache);
        let metrics = (
            &read.entities,
            positions,
            &mut write.velocities,
            previous_phys_cache,
            read.masses.maybe(),
            read.colliders.maybe(),
            !&read.mountings,
            read.stickies.maybe(),
            &mut write.physics_states,
            // TODO: if we need to avoid collisions for other things consider moving whether it
            // should interact into the collider component or into a separate component
            read.projectiles.maybe(),
            read.char_states.maybe(),
        )
            .par_join()
            .filter(|(_, _, _, _, _, _, _, sticky, physics, _, _)| {
                sticky.is_none() || (physics.on_wall.is_none() && !physics.on_ground)
            })
            .map(|(e, p, v, vd, m, c, _, _, ph, pr, c_s)| (e, p, v, vd, m, c, ph, pr, c_s))
            .map_init(
                || {
                    prof_span!(guard, "physics e<>e rayon job");
                    guard
                },
                |_guard,
                 (
                    entity,
                    pos,
                    vel,
                    previous_cache,
                    mass,
                    collider,
                    physics,
                    projectile,
                    char_state_maybe,
                )| {
                    let z_limits = calc_z_limit(char_state_maybe, collider);
                    let mass = mass.map(|m| m.0).unwrap_or(previous_cache.scale);

                    // Resets touch_entities in physics
                    physics.touch_entities.clear();

                    let is_projectile = projectile.is_some();

                    let mut vel_delta = Vec3::zero();

                    let mut entity_entity_collision_checks = 0;
                    let mut entity_entity_collisions = 0;

                    if use_grid {
                        let aabr = {
                            let center = previous_cache.center.xy().map(|e| e as i32);
                            let radius = previous_cache.collision_boundary.ceil() as i32;
                            // From conversion of center above
                            const CENTER_TRUNCATION_ERROR: i32 = 1;
                            let max_dist = radius + CENTER_TRUNCATION_ERROR;

                            Aabr {
                                min: center - max_dist,
                                max: center + max_dist,
                            }
                        };

                        spatial_grid
                            .in_aabr(aabr)
                            .filter_map(|entity| {
                                read.uids
                                    .get(entity)
                                    .zip(positions.get(entity))
                                    .zip(previous_phys_cache.get(entity))
                                    .map(|((uid, pos), previous_cache)| {
                                        (
                                            entity,
                                            uid,
                                            pos,
                                            previous_cache,
                                            read.masses.get(entity),
                                            read.colliders.get(entity),
                                            read.char_states.get(entity),
                                        )
                                    })
                            })
                            .for_each(
                                |(
                                    entity_other,
                                    other,
                                    pos_other,
                                    previous_cache_other,
                                    mass_other,
                                    collider_other,
                                    char_state_other_maybe,
                                )| {
                                    let collision_boundary = previous_cache.collision_boundary
                                        + previous_cache_other.collision_boundary;
                                    if previous_cache
                                        .center
                                        .distance_squared(previous_cache_other.center)
                                        > collision_boundary.powi(2)
                                        || entity == entity_other
                                    {
                                        return;
                                    }

                                    let collision_dist = previous_cache.scaled_radius
                                        + previous_cache_other.scaled_radius;
                                    let z_limits_other =
                                        calc_z_limit(char_state_other_maybe, collider_other);

                                    let mass_other = mass_other
                                        .map(|m| m.0)
                                        .unwrap_or(previous_cache_other.scale);
                                    //This check after the pos check, as we currently don't have
                                    // that many
                                    // massless entites [citation needed]
                                    if mass_other == 0.0 {
                                        return;
                                    }

                                    entity_entity_collision_checks += 1;

                                    const MIN_COLLISION_DIST: f32 = 0.3;
                                    let increments = ((previous_cache.velocity_dt
                                        - previous_cache_other.velocity_dt)
                                        .magnitude()
                                        / MIN_COLLISION_DIST)
                                        .max(1.0)
                                        .ceil()
                                        as usize;
                                    let step_delta = 1.0 / increments as f32;
                                    let mut collided = false;

                                    for i in 0..increments {
                                        let factor = i as f32 * step_delta;
                                        let pos = pos.0 + previous_cache.velocity_dt * factor;
                                        let pos_other =
                                            pos_other.0 + previous_cache_other.velocity_dt * factor;

                                        let diff = pos.xy() - pos_other.xy();

                                        if diff.magnitude_squared() <= collision_dist.powi(2)
                                            && pos.z + z_limits.1 * previous_cache.scale
                                                >= pos_other.z
                                                    + z_limits_other.0 * previous_cache_other.scale
                                            && pos.z + z_limits.0 * previous_cache.scale
                                                <= pos_other.z
                                                    + z_limits_other.1 * previous_cache_other.scale
                                        {
                                            if !collided {
                                                physics.touch_entities.push(*other);
                                                entity_entity_collisions += 1;
                                            }

                                            // Don't apply repulsive force to projectiles or if
                                            // we're
                                            // colliding
                                            // with a terrain-like entity, or if we are a
                                            // terrain-like
                                            // entity
                                            if diff.magnitude_squared() > 0.0
                                                && !is_projectile
                                                && !matches!(
                                                    collider_other,
                                                    Some(Collider::Voxel { .. })
                                                )
                                                && !matches!(collider, Some(Collider::Voxel { .. }))
                                            {
                                                let force = 400.0
                                                    * (collision_dist - diff.magnitude())
                                                    * mass_other
                                                    / (mass + mass_other);

                                                vel_delta += Vec3::from(diff.normalized())
                                                    * force
                                                    * step_delta;
                                            }

                                            collided = true;
                                        }
                                    }
                                },
                            );
                    } else {
                        for (
                            entity_other,
                            other,
                            pos_other,
                            previous_cache_other,
                            mass_other,
                            collider_other,
                            _,
                            _,
                            _,
                            _,
                            char_state_other_maybe,
                        ) in (
                            &read.entities,
                            &read.uids,
                            positions,
                            previous_phys_cache,
                            read.masses.maybe(),
                            read.colliders.maybe(),
                            !&read.projectiles,
                            !&read.mountings,
                            !&read.beams,
                            !&read.shockwaves,
                            read.char_states.maybe(),
                        )
                            .join()
                        {
                            let collision_boundary = previous_cache.collision_boundary
                                + previous_cache_other.collision_boundary;
                            if previous_cache
                                .center
                                .distance_squared(previous_cache_other.center)
                                > collision_boundary.powi(2)
                                || entity == entity_other
                            {
                                continue;
                            }

                            let collision_dist =
                                previous_cache.scaled_radius + previous_cache_other.scaled_radius;
                            let z_limits_other =
                                calc_z_limit(char_state_other_maybe, collider_other);

                            let mass_other = mass_other
                                .map(|m| m.0)
                                .unwrap_or(previous_cache_other.scale);
                            //This check after the pos check, as we currently don't have that many
                            // massless entites [citation needed]
                            if mass_other == 0.0 {
                                continue;
                            }

                            entity_entity_collision_checks += 1;

                            const MIN_COLLISION_DIST: f32 = 0.3;
                            let increments = ((previous_cache.velocity_dt
                                - previous_cache_other.velocity_dt)
                                .magnitude()
                                / MIN_COLLISION_DIST)
                                .max(1.0)
                                .ceil() as usize;
                            let step_delta = 1.0 / increments as f32;
                            let mut collided = false;

                            for i in 0..increments {
                                let factor = i as f32 * step_delta;
                                let pos = pos.0 + previous_cache.velocity_dt * factor;
                                let pos_other =
                                    pos_other.0 + previous_cache_other.velocity_dt * factor;

                                let diff = pos.xy() - pos_other.xy();

                                if diff.magnitude_squared() <= collision_dist.powi(2)
                                    && pos.z + z_limits.1 * previous_cache.scale
                                        >= pos_other.z
                                            + z_limits_other.0 * previous_cache_other.scale
                                    && pos.z + z_limits.0 * previous_cache.scale
                                        <= pos_other.z
                                            + z_limits_other.1 * previous_cache_other.scale
                                {
                                    if !collided {
                                        physics.touch_entities.push(*other);
                                        entity_entity_collisions += 1;
                                    }

                                    // Don't apply repulsive force to projectiles or if we're
                                    // colliding
                                    // with a terrain-like entity, or if we are a terrain-like
                                    // entity
                                    if diff.magnitude_squared() > 0.0
                                        && !is_projectile
                                        && !matches!(collider_other, Some(Collider::Voxel { .. }))
                                        && !matches!(collider, Some(Collider::Voxel { .. }))
                                    {
                                        let force = 400.0
                                            * (collision_dist - diff.magnitude())
                                            * mass_other
                                            / (mass + mass_other);

                                        vel_delta +=
                                            Vec3::from(diff.normalized()) * force * step_delta;
                                    }

                                    collided = true;
                                }
                            }
                        }
                    }

                    // Change velocity
                    vel.0 += vel_delta * read.dt.0;

                    // Metrics
                    PhysicsMetrics {
                        entity_entity_collision_checks,
                        entity_entity_collisions,
                    }
                },
            )
            .reduce(PhysicsMetrics::default, |old, new| PhysicsMetrics {
                entity_entity_collision_checks: old.entity_entity_collision_checks
                    + new.entity_entity_collision_checks,
                entity_entity_collisions: old.entity_entity_collisions
                    + new.entity_entity_collisions,
            });
        write.physics_metrics.entity_entity_collision_checks =
            metrics.entity_entity_collision_checks;
        write.physics_metrics.entity_entity_collisions = metrics.entity_entity_collisions;
    }

    fn handle_movement_and_terrain(&mut self, job: &mut Job<Sys>) {
        let PhysicsData {
            ref read,
            ref mut write,
        } = self;
        // Apply movement inputs
        span!(guard, "Apply movement and terrain collision");
        let (positions, velocities, previous_phys_cache, orientations) = (
            &write.positions,
            &mut write.velocities,
            &write.previous_phys_cache,
            &write.orientations,
        );

        // First pass: update velocity using air resistance and gravity for each entity.
        // We do this in a first pass because it helps keep things more stable for
        // entities that are anchored to other entities (such as airships).
        (
            &read.entities,
            positions,
            velocities,
            &write.physics_states,
            !&read.mountings,
        )
            .par_join()
            .for_each(|(entity, pos, vel, physics_state, _)| {
                let in_loaded_chunk = read
                    .terrain
                    .get_key(read.terrain.pos_key(pos.0.map(|e| e.floor() as i32)))
                    .is_some();
                // Integrate forces
                // Friction is assumed to be a constant dependent on location
                let friction = if physics_state.on_ground { 0.0 } else { FRIC_AIR }
                        // .max(if physics_state.on_ground {
                        //     FRIC_GROUND
                        // } else {
                        //     0.0
                        // })
                        .max(if physics_state.in_liquid.is_some() {
                            FRIC_FLUID
                        } else {
                            0.0
                        });
                let downward_force =
                    if !in_loaded_chunk {
                        0.0 // No gravity in unloaded chunks
                    } else if physics_state
                        .in_liquid
                        .map(|depth| depth > 0.75)
                        .unwrap_or(false)
                    {
                        (1.0 - BOUYANCY) * GRAVITY
                    } else {
                        GRAVITY
                    } * read.gravities.get(entity).map(|g| g.0).unwrap_or_default();

                vel.0 = integrate_forces(read.dt.0, vel.0, downward_force, friction);
            });

        let velocities = &write.velocities;

        // Second pass: resolve collisions
        let (pos_writes, vel_writes, land_on_grounds) = (
            &read.entities,
            read.scales.maybe(),
            read.stickies.maybe(),
            &read.colliders,
            positions,
            velocities,
            orientations,
            read.bodies.maybe(),
            read.character_states.maybe(),
            &mut write.physics_states,
            previous_phys_cache,
            !&read.mountings,
        )
            .par_join()
            .fold(
                || (Vec::new(), Vec::new(), Vec::new()),
                |(mut pos_writes, mut vel_writes, mut land_on_grounds),
                 (
                    entity,
                    scale,
                    sticky,
                    collider,
                    pos,
                    vel,
                    _ori,
                    body,
                    character_state,
                    mut physics_state,
                    _previous_cache,
                    _,
                )| {
                    // Defer the writes of positions and velocities to allow an inner loop over
                    // terrain-like entities
                    let mut vel = *vel;
                    let old_vel = vel;
                    if sticky.is_some() && physics_state.on_surface().is_some() {
                        vel.0 = physics_state.ground_vel;
                        return (pos_writes, vel_writes, land_on_grounds);
                    }

                    let scale = if let Collider::Voxel { .. } = collider {
                        scale.map(|s| s.0).unwrap_or(1.0)
                    } else {
                        // TODO: Use scale & actual proportions when pathfinding is good
                        // enough to manage irregular entity sizes
                        1.0
                    };

                    let in_loaded_chunk = read
                        .terrain
                        .get_key(read.terrain.pos_key(pos.0.map(|e| e.floor() as i32)))
                        .is_some();

                    // Don't move if we're not in a loaded chunk
                    let pos_delta = if in_loaded_chunk {
                        vel.0 * read.dt.0
                    } else {
                        Vec3::zero()
                    };

                    // What's going on here? Because collisions need to be resolved against multiple
                    // colliders, this code takes the current position and
                    // propagates it forward according to velocity to find a
                    // 'target' position. This is where we'd ideally end up at the end of the tick,
                    // assuming no collisions. Then, we refine this target by
                    // stepping from the original position to the target for
                    // every obstacle, refining the target position as we go. It's not perfect, but
                    // it works pretty well in practice. Oddities can occur on
                    // the intersection between multiple colliders, but it's not
                    // like any game physics system resolves these sort of things well anyway. At
                    // the very least, we don't do things that result in glitchy
                    // velocities or entirely broken position snapping.
                    let mut tgt_pos = pos.0 + pos_delta;

                    let was_on_ground = physics_state.on_ground;
                    let block_snap = body.map_or(false, |body| body.jump_impulse().is_some());
                    let climbing =
                        character_state.map_or(false, |cs| matches!(cs, CharacterState::Climb));

                    match &collider {
                        Collider::Voxel { .. } => {
                            // for now, treat entities with voxel colliders as their bounding
                            // cylinders for the purposes of colliding them with terrain

                            // Additionally, multiply radius by 0.1 to make the cylinder smaller to
                            // avoid lag
                            let radius = collider.get_radius() * scale * 0.1;
                            let (z_min, z_max) = collider.get_z_limits(scale);

                            let mut cpos = *pos;
                            let cylinder = (radius, z_min, z_max);
                            box_voxel_collision(
                                cylinder,
                                &*read.terrain,
                                entity,
                                &mut cpos,
                                tgt_pos,
                                &mut vel,
                                &mut physics_state,
                                Vec3::zero(),
                                &read.dt,
                                was_on_ground,
                                block_snap,
                                climbing,
                                |entity, vel| land_on_grounds.push((entity, vel)),
                            );
                            tgt_pos = cpos.0;
                        },
                        Collider::Box {
                            radius,
                            z_min,
                            z_max,
                        } => {
                            // Scale collider
                            let radius = radius.min(0.45) * scale;
                            let z_min = *z_min * scale;
                            let z_max = z_max.clamped(1.2, 1.95) * scale;

                            let cylinder = (radius, z_min, z_max);
                            let mut cpos = *pos;
                            box_voxel_collision(
                                cylinder,
                                &*read.terrain,
                                entity,
                                &mut cpos,
                                tgt_pos,
                                &mut vel,
                                &mut physics_state,
                                Vec3::zero(),
                                &read.dt,
                                was_on_ground,
                                block_snap,
                                climbing,
                                |entity, vel| land_on_grounds.push((entity, vel)),
                            );
                            tgt_pos = cpos.0;
                        },
                        Collider::Point => {
                            let mut pos = *pos;

                            let (dist, block) = read
                                .terrain
                                .ray(pos.0, pos.0 + pos_delta)
                                .until(|block: &Block| block.is_filled())
                                .ignore_error()
                                .cast();

                            pos.0 += pos_delta.try_normalized().unwrap_or_else(Vec3::zero) * dist;

                            // Can't fail since we do ignore_error above
                            if block.unwrap().is_some() {
                                let block_center = pos.0.map(|e| e.floor()) + 0.5;
                                let block_rpos = (pos.0 - block_center)
                                    .try_normalized()
                                    .unwrap_or_else(Vec3::zero);

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

                            physics_state.in_liquid = read
                                .terrain
                                .get(pos.0.map(|e| e.floor() as i32))
                                .ok()
                                .and_then(|vox| vox.is_liquid().then_some(1.0));

                            tgt_pos = pos.0;
                        },
                    }

                    // Collide with terrain-like entities
                    for (
                        entity_other,
                        _other,
                        pos_other,
                        vel_other,
                        previous_cache_other,
                        _mass_other,
                        collider_other,
                        ori_other,
                        _,
                        _,
                        _,
                        _,
                        _char_state_other_maybe,
                    ) in (
                        &read.entities,
                        &read.uids,
                        positions,
                        velocities,
                        previous_phys_cache,
                        read.masses.maybe(),
                        &read.colliders,
                        orientations,
                        !&read.projectiles,
                        !&read.mountings,
                        !&read.beams,
                        !&read.shockwaves,
                        read.char_states.maybe(),
                    )
                        .join()
                    {
                        // TODO: terrain-collider-size aware broadphase
                        /*let collision_boundary = previous_cache.collision_boundary
                            + previous_cache_other.collision_boundary;
                        if previous_cache
                            .center
                            .distance_squared(previous_cache_other.center)
                            > collision_boundary.powi(2)
                        {
                            continue;
                        }*/
                        if entity == entity_other {
                            continue;
                        }

                        let voxel_id = if let Collider::Voxel { id } = collider_other {
                            id
                        } else {
                            continue;
                        };
                        // use bounding cylinder regardless of our collider
                        // TODO: extract point-terrain collision above to its own function
                        let radius = collider.get_radius();
                        let (z_min, z_max) = collider.get_z_limits(1.0);

                        let radius = radius.min(0.45) * scale;
                        let z_min = z_min * scale;
                        let z_max = z_max.clamped(1.2, 1.95) * scale;

                        if let Some(voxel_collider) =
                            VOXEL_COLLIDER_MANIFEST.read().colliders.get(&*voxel_id)
                        {
                            let mut physics_state_delta = physics_state.clone();
                            // deliberately don't use scale yet here, because the 11.0/0.8
                            // thing is in the comp::Scale for visual reasons
                            let mut cpos = *pos;
                            let wpos = cpos.0;

                            // TODO: Cache the matrices here to avoid recomputing

                            let transform_from = Mat4::<f32>::translation_3d(pos_other.0 - wpos)
                                * Mat4::from(ori_other.to_quat())
                                * Mat4::<f32>::translation_3d(voxel_collider.translation);
                            let transform_to = transform_from.inverted();
                            let ori_from = Mat4::from(ori_other.to_quat());
                            let ori_to = ori_from.inverted();

                            // The velocity of the collider, taking into account orientation.
                            let wpos_rel = (Mat4::<f32>::translation_3d(pos_other.0)
                                * Mat4::from(ori_other.to_quat())
                                * Mat4::<f32>::translation_3d(voxel_collider.translation))
                            .inverted()
                            .mul_point(wpos);
                            let wpos_last = (Mat4::<f32>::translation_3d(pos_other.0)
                                * Mat4::from(previous_cache_other.ori)
                                * Mat4::<f32>::translation_3d(voxel_collider.translation))
                            .mul_point(wpos_rel);
                            let vel_other = vel_other.0 + (wpos - wpos_last) / read.dt.0;

                            cpos.0 = transform_to.mul_point(Vec3::zero());
                            vel.0 = ori_to.mul_direction(vel.0 - vel_other);
                            let cylinder = (radius, z_min, z_max);
                            box_voxel_collision(
                                cylinder,
                                &voxel_collider.dyna,
                                entity,
                                &mut cpos,
                                transform_to.mul_point(tgt_pos - wpos),
                                &mut vel,
                                &mut physics_state_delta,
                                ori_to.mul_direction(vel_other),
                                &read.dt,
                                was_on_ground,
                                block_snap,
                                climbing,
                                |entity, vel| {
                                    land_on_grounds
                                        .push((entity, Vel(ori_from.mul_direction(vel.0))))
                                },
                            );

                            cpos.0 = transform_from.mul_point(cpos.0) + wpos;
                            vel.0 = ori_from.mul_direction(vel.0) + vel_other;
                            tgt_pos = cpos.0;

                            // union in the state updates, so that the state isn't just based on
                            // the most recent terrain that collision was attempted with
                            if physics_state_delta.on_ground {
                                physics_state.ground_vel = vel_other;
                            }
                            physics_state.on_ground |= physics_state_delta.on_ground;
                            physics_state.on_ceiling |= physics_state_delta.on_ceiling;
                            physics_state.on_wall = physics_state.on_wall.or_else(|| {
                                physics_state_delta
                                    .on_wall
                                    .map(|dir| ori_from.mul_direction(dir))
                            });
                            physics_state
                                .touch_entities
                                .append(&mut physics_state_delta.touch_entities);
                            physics_state.in_liquid =
                                match (physics_state.in_liquid, physics_state_delta.in_liquid) {
                                    // this match computes `x <|> y <|> liftA2 max x y`
                                    (Some(x), Some(y)) => Some(x.max(y)),
                                    (x @ Some(_), _) => x,
                                    (_, y @ Some(_)) => y,
                                    _ => None,
                                };
                        }
                    }

                    if tgt_pos != pos.0 {
                        pos_writes.push((entity, Pos(tgt_pos)));
                    }
                    if vel != old_vel {
                        vel_writes.push((entity, vel));
                    }

                    (pos_writes, vel_writes, land_on_grounds)
                },
            )
            .reduce(
                || (Vec::new(), Vec::new(), Vec::new()),
                |(mut pos_writes_a, mut vel_writes_a, mut land_on_grounds_a),
                 (mut pos_writes_b, mut vel_writes_b, mut land_on_grounds_b)| {
                    pos_writes_a.append(&mut pos_writes_b);
                    vel_writes_a.append(&mut vel_writes_b);
                    land_on_grounds_a.append(&mut land_on_grounds_b);
                    (pos_writes_a, vel_writes_a, land_on_grounds_a)
                },
            );
        drop(guard);
        job.cpu_stats.measure(ParMode::Single);

        let pos_writes: HashMap<Entity, Pos> = pos_writes.into_iter().collect();
        let vel_writes: HashMap<Entity, Vel> = vel_writes.into_iter().collect();
        for (entity, pos, vel) in
            (&read.entities, &mut write.positions, &mut write.velocities).join()
        {
            if let Some(new_pos) = pos_writes.get(&entity) {
                *pos = *new_pos;
            }

            if let Some(new_vel) = vel_writes.get(&entity) {
                *vel = *new_vel;
            }
        }

        for (ori, previous_phys_cache) in
            (&write.orientations, &mut write.previous_phys_cache).join()
        {
            previous_phys_cache.ori = ori.to_quat();
        }

        let mut event_emitter = read.event_bus.emitter();
        land_on_grounds.into_iter().for_each(|(entity, vel)| {
            event_emitter.emit(ServerEvent::LandOnGround { entity, vel: vel.0 });
        });
    }
}

impl<'a> System<'a> for Sys {
    type SystemData = PhysicsData<'a>;

    const NAME: &'static str = "phys";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(job: &mut Job<Self>, mut psd: Self::SystemData) {
        psd.reset();

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
        psd.maintain_pushback_cache();

        let spatial_grid = psd.construct_spatial_grid();
        psd.apply_pushback(job, &spatial_grid);

        psd.handle_movement_and_terrain(job);
    }
}

#[allow(clippy::too_many_arguments)]
fn box_voxel_collision<'a, T: BaseVol<Vox = Block> + ReadVol>(
    cylinder: (f32, f32, f32),
    terrain: &'a T,
    entity: Entity,
    pos: &mut Pos,
    tgt_pos: Vec3<f32>,
    vel: &mut Vel,
    physics_state: &mut PhysicsState,
    ground_vel: Vec3<f32>,
    dt: &DeltaTime,
    was_on_ground: bool,
    block_snap: bool,
    climbing: bool,
    mut land_on_ground: impl FnMut(Entity, Vel),
) {
    let (radius, z_min, z_max) = cylinder;

    // Probe distances
    let hdist = radius.ceil() as i32;
    // Neighbouring blocks iterator
    let near_iter = (-hdist..hdist + 1)
        .map(move |i| {
            (-hdist..hdist + 1).map(move |j| {
                (1 - Block::MAX_HEIGHT.ceil() as i32 + z_min.floor() as i32
                    ..z_max.ceil() as i32 + 1)
                    .map(move |k| (i, j, k))
            })
        })
        .flatten()
        .flatten();

    // Function for iterating over the blocks the player at a specific position
    // collides with
    fn collision_iter<'a, T: BaseVol<Vox = Block> + ReadVol>(
        pos: Vec3<f32>,
        terrain: &'a T,
        hit: &'a impl Fn(&Block) -> bool,
        height: &'a impl Fn(&Block) -> f32,
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
                    max: block_pos.map(|e| e as f32) + Vec3::new(1.0, 1.0, height(&block)),
                };

                if player_aabb.collides_with_aabb(block_aabb) {
                    return Some(block_aabb);
                }
            }

            None
        })
    }

    let z_range = z_min..z_max;
    // Function for determining whether the player at a specific position collides
    // with blocks with the given criteria
    fn collision_with<'a, T: BaseVol<Vox = Block> + ReadVol>(
        pos: Vec3<f32>,
        terrain: &'a T,
        hit: impl Fn(&Block) -> bool,
        near_iter: impl Iterator<Item = (i32, i32, i32)> + 'a,
        radius: f32,
        z_range: Range<f32>,
    ) -> bool {
        collision_iter(
            pos,
            terrain,
            &|block| block.is_solid() && hit(block),
            &Block::solid_height,
            near_iter,
            radius,
            z_range,
        )
        .count()
            > 0
    }

    physics_state.on_ground = false;

    let mut on_ground = false;
    let mut on_ceiling = false;
    let mut attempts = 0; // Don't loop infinitely here

    let mut pos_delta = tgt_pos - pos.0;

    // Don't jump too far at once
    let increments = (pos_delta.map(|e| e.abs()).reduce_partial_max() / 0.3)
        .ceil()
        .max(1.0);
    let old_pos = pos.0;
    fn block_true(_: &Block) -> bool { true }
    for _ in 0..increments as usize {
        pos.0 += pos_delta / increments;

        const MAX_ATTEMPTS: usize = 16;

        // While the player is colliding with the terrain...
        while collision_with(
            pos.0,
            &terrain,
            block_true,
            near_iter.clone(),
            radius,
            z_range.clone(),
        ) && attempts < MAX_ATTEMPTS
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
                                max: block_pos.map(|e| e as f32) + Vec3::new(1.0, 1.0, block.solid_height()),
                            },
                            block.solid_height(),
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
            if resolve_dir.z > 0.0
            /* && vel.0.z <= 0.0 */
            {
                on_ground = true;

                if !was_on_ground {
                    land_on_ground(entity, *vel);
                }
            } else if resolve_dir.z < 0.0 && vel.0.z >= 0.0 {
                on_ceiling = true;
            }

            // When the resolution direction is non-vertical, we must be colliding
            // with a wall If the space above is free...
            if !collision_with(Vec3::new(pos.0.x, pos.0.y, (pos.0.z + 0.1).ceil()), &terrain, block_true, near_iter.clone(), radius, z_range.clone())
                // ...and we're being pushed out horizontally...
                && resolve_dir.z == 0.0
                // ...and the vertical resolution direction is sufficiently great...
                && dir.z < -0.1
                // ...and we're falling/standing OR there is a block *directly* beneath our current origin (note: not hitbox)...
                // && terrain
                //     .get((pos.0 - Vec3::unit_z() * 0.1).map(|e| e.floor() as i32))
                //     .map(|block| block.is_solid())
                //     .unwrap_or(false)
                // ...and there is a collision with a block beneath our current hitbox...
                && collision_with(
                    pos.0 + resolve_dir - Vec3::unit_z() * 1.25,
                    &terrain,
                    block_true,
                    near_iter.clone(),
                    radius,
                    z_range.clone(),
                )
            {
                // ...block-hop!
                pos.0.z = (pos.0.z + 0.1).floor() + block_height;
                vel.0.z = vel.0.z.max(0.0);
                on_ground = true;
                break;
            } else {
                // Correct the velocity
                vel.0 = vel.0.map2(
                    resolve_dir,
                    |e, d| {
                        if d * e.signum() < 0.0 { 0.0 } else { e }
                    },
                );
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
        pos.0 - Vec3::unit_z() * 1.1,
        &terrain,
        block_true,
        near_iter.clone(),
        radius,
        z_range.clone(),
    ) && vel.0.z < 0.25
        && vel.0.z > -1.5
        && was_on_ground
        && block_snap
    {
        let snap_height = terrain
            .get(Vec3::new(pos.0.x, pos.0.y, pos.0.z - 0.1).map(|e| e.floor() as i32))
            .ok()
            .filter(|block| block.is_solid())
            .map(|block| block.solid_height())
            .unwrap_or(0.0);
        vel.0.z = 0.0;
        pos.0.z = (pos.0.z - 0.1).floor() + snap_height;
        physics_state.on_ground = true;
    }

    let dirs = [
        Vec3::unit_x(),
        Vec3::unit_y(),
        -Vec3::unit_x(),
        -Vec3::unit_y(),
    ];

    if let (wall_dir, true) = dirs.iter().fold((Vec3::zero(), false), |(a, hit), dir| {
        if collision_with(
            pos.0 + *dir * 0.01,
            &terrain,
            block_true,
            near_iter.clone(),
            radius,
            z_range.clone(),
        ) {
            (a + dir, true)
        } else {
            (a, hit)
        }
    }) {
        physics_state.on_wall = Some(wall_dir);
    } else {
        physics_state.on_wall = None;
    }

    if physics_state.on_ground || (physics_state.on_wall.is_some() && climbing) {
        vel.0 *= (1.0 - FRIC_GROUND.min(1.0)).powf(dt.0 * 60.0);
        physics_state.ground_vel = ground_vel;
    }

    // Figure out if we're in water
    physics_state.in_liquid = collision_iter(
        pos.0,
        &*terrain,
        &|block| block.is_liquid(),
        // The liquid part of a liquid block always extends 1 block high.
        &|_block| 1.0,
        near_iter,
        radius,
        z_min..z_max,
    )
    .max_by_key(|block_aabb| (block_aabb.max.z * 100.0) as i32)
    .map(|block_aabb| block_aabb.max.z - pos.0.z);
}
