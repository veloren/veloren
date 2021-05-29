use common::{
    comp::{
        body::ship::figuredata::{VoxelCollider, VOXEL_COLLIDER_MANIFEST},
        fluid_dynamics::{Fluid, Wings},
        BeamSegment, Body, CharacterState, Collider, Density, Mass, Mounting, Ori, PhysicsState,
        Pos, PosVelDefer, PreviousPhysCache, Projectile, Scale, Shockwave, Sticky, Vel,
    },
    consts::{AIR_DENSITY, FRIC_GROUND, GRAVITY},
    event::{EventBus, ServerEvent},
    outcome::Outcome,
    resources::DeltaTime,
    states,
    terrain::{Block, TerrainGrid},
    uid::Uid,
    util::{Projection, SpatialGrid},
    vol::{BaseVol, ReadVol},
};
use common_base::{prof_span, span};
use common_ecs::{Job, Origin, ParMode, Phase, PhysicsMetrics, System};
use rayon::iter::ParallelIterator;
use specs::{
    shred::{ResourceId, World},
    Entities, Entity, Join, ParJoin, Read, ReadExpect, ReadStorage, SystemData, Write, WriteExpect,
    WriteStorage,
};
use std::ops::Range;
use vek::*;

/// The density of the fluid as a function of submersion ratio in given fluid
/// where it is assumed that any unsubmersed part is is air.
// TODO: Better suited partial submersion curve?
fn fluid_density(height: f32, fluid: &Fluid) -> Density {
    // If depth is less than our height (partial submersion), remove
    // fluid density based on the ratio of displacement to full volume.
    let immersion = fluid
        .depth()
        .map_or(1.0, |depth| (depth / height).clamp(0.0, 1.0));

    Density(fluid.density().0 * immersion + AIR_DENSITY * (1.0 - immersion))
}

#[allow(clippy::too_many_arguments)]
fn integrate_forces(
    dt: &DeltaTime,
    mut vel: Vel,
    (body, wings): (&Body, Option<&Wings>),
    density: &Density,
    mass: &Mass,
    fluid: &Fluid,
    gravity: f32,
) -> Vel {
    let dim = body.dimensions();
    let height = dim.z;
    let rel_flow = fluid.relative_flow(&vel);
    let fluid_density = fluid_density(height, fluid);
    debug_assert!(mass.0 > 0.0);
    debug_assert!(density.0 > 0.0);

    // Aerodynamic/hydrodynamic forces
    if !rel_flow.0.is_approx_zero() {
        debug_assert!(!rel_flow.0.map(|a| a.is_nan()).reduce_or());
        let impulse = dt.0 * body.aerodynamic_forces(&rel_flow, fluid_density.0, wings);
        debug_assert!(!impulse.map(|a| a.is_nan()).reduce_or());
        if !impulse.is_approx_zero() {
            let new_v = vel.0 + impulse / mass.0;
            // If the new velocity is in the opposite direction, it's because the forces
            // involved are too high for the current tick to handle. We deal with this by
            // removing the component of our velocity vector along the direction of force.
            // This way we can only ever lose velocity and will never experience a reverse
            // in direction from events such as falling into water at high velocities.
            if new_v.dot(vel.0) < 0.0 {
                // Multiply by a factor to prevent full stop, as this can cause things to get
                // stuck in high-density medium
                vel.0 -= vel.0.projected(&impulse) * 0.9;
            } else {
                vel.0 = new_v;
            }
        };
        debug_assert!(!vel.0.map(|a| a.is_nan()).reduce_or());
    };

    // Hydrostatic/aerostatic forces
    // modify gravity to account for the effective density as a result of buoyancy
    let down_force = dt.0 * gravity * (density.0 - fluid_density.0) / density.0;
    vel.0.z -= down_force;

    vel
}

fn calc_z_limit(
    char_state_maybe: Option<&CharacterState>,
    collider: Option<&Collider>,
) -> (f32, f32) {
    let modifier = if char_state_maybe.map_or(false, |c_s| c_s.is_dodge() || c_s.is_glide()) {
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
    mountings: ReadStorage<'a, Mounting>,
    projectiles: ReadStorage<'a, Projectile>,
    beams: ReadStorage<'a, BeamSegment>,
    shockwaves: ReadStorage<'a, Shockwave>,
    char_states: ReadStorage<'a, CharacterState>,
    bodies: ReadStorage<'a, Body>,
    character_states: ReadStorage<'a, CharacterState>,
    densities: ReadStorage<'a, Density>,
}

#[derive(SystemData)]
pub struct PhysicsWrite<'a> {
    physics_metrics: WriteExpect<'a, PhysicsMetrics>,
    cached_spatial_grid: Write<'a, common::CachedSpatialGrid>,
    physics_states: WriteStorage<'a, PhysicsState>,
    positions: WriteStorage<'a, Pos>,
    velocities: WriteStorage<'a, Vel>,
    pos_vel_defers: WriteStorage<'a, PosVelDefer>,
    orientations: WriteStorage<'a, Ori>,
    previous_phys_cache: WriteStorage<'a, PreviousPhysCache>,
    outcomes: Write<'a, Vec<Outcome>>,
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
        let lg2_cell_size = 5;
        let lg2_large_cell_size = 6;
        let radius_cutoff = 8;
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

    #[allow(clippy::nonminimal_bool)]
    fn apply_pushback(&mut self, job: &mut Job<Sys>, spatial_grid: &SpatialGrid) {
        span!(_guard, "Apply pushback");
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
            &read.masses,
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
            .map(|(e, p, v, vd, m, c, _, sticky, ph, pr, c_s)| {
                (e, p, v, vd, m, c, sticky, ph, pr, c_s)
            })
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
                    sticky,
                    physics,
                    projectile,
                    char_state_maybe,
                )| {
                    let is_sticky = sticky.is_some();
                    let is_mid_air = physics.on_wall.is_none() && physics.on_ground;
                    let mut entity_entity_collision_checks = 0;
                    let mut entity_entity_collisions = 0;

                    // TODO: quick fix for bad performance at extrememly high velocities
                    // use oriented rectangles at some threshold of displacement/radius
                    // to query the spatial grid and limit max displacement per tick somehow
                    if previous_cache.collision_boundary > 128.0 {
                        return PhysicsMetrics {
                            entity_entity_collision_checks,
                            entity_entity_collisions,
                        };
                    }

                    let z_limits = calc_z_limit(char_state_maybe, collider);

                    // Resets touch_entities in physics
                    physics.touch_entities.clear();

                    let is_projectile = projectile.is_some();

                    let mut vel_delta = Vec3::zero();

                    let query_center = previous_cache.center.xy();
                    let query_radius = previous_cache.collision_boundary;

                    spatial_grid
                        .in_circle_aabr(query_center, query_radius)
                        .filter_map(|entity| {
                            read.uids
                                .get(entity)
                                .and_then(|l| positions.get(entity).map(|r| (l, r)))
                                .and_then(|l| previous_phys_cache.get(entity).map(|r| (l, r)))
                                .and_then(|l| read.masses.get(entity).map(|r| (l, r)))
                                .map(|(((uid, pos), previous_cache), mass)| {
                                    (
                                        entity,
                                        uid,
                                        pos,
                                        previous_cache,
                                        mass,
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
                                            physics.touch_entities.insert(*other);
                                            entity_entity_collisions += 1;
                                        }

                                        // Don't apply repulsive force to projectiles or if
                                        // we're
                                        // colliding
                                        // with a terrain-like entity, or if we are a
                                        // terrain-like
                                        // entity
                                        //
                                        // Don't apply force when entity is a sticky which is on the
                                        // ground (or on the wall)
                                        if !(is_sticky && !is_mid_air)
                                            && diff.magnitude_squared() > 0.0
                                            && !is_projectile
                                            && !matches!(
                                                collider_other,
                                                Some(Collider::Voxel { .. })
                                            )
                                            && !matches!(collider, Some(Collider::Voxel { .. }))
                                        {
                                            let force = 400.0
                                                * (collision_dist - diff.magnitude())
                                                * mass_other.0
                                                / (mass.0 + mass_other.0);

                                            vel_delta +=
                                                Vec3::from(diff.normalized()) * force * step_delta;
                                        }

                                        collided = true;
                                    }
                                }
                            },
                        );

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

    fn construct_voxel_collider_spatial_grid(&mut self) -> SpatialGrid {
        span!(_guard, "Construct voxel collider spatial grid");
        let PhysicsData {
            ref read,
            ref write,
        } = self;
        // NOTE: i32 places certain constraints on how far out collision works
        // NOTE: uses the radius of the entity and their current position rather than
        // the radius of their bounding sphere for the current frame of movement
        // because the nonmoving entity is what is collided against in the inner
        // loop of the pushback collision code
        // TODO: optimize these parameters (especially radius cutoff)
        let lg2_cell_size = 7; // 128
        let lg2_large_cell_size = 8; // 256
        let radius_cutoff = 64;
        let mut spatial_grid = SpatialGrid::new(lg2_cell_size, lg2_large_cell_size, radius_cutoff);
        // TODO: give voxel colliders their own component type
        for (entity, pos, collider, ori) in (
            &read.entities,
            &write.positions,
            &read.colliders,
            &write.orientations,
        )
            .join()
        {
            let voxel_id = match collider {
                Collider::Voxel { id } => id,
                _ => continue,
            };

            if let Some(voxel_collider) = VOXEL_COLLIDER_MANIFEST.read().colliders.get(&*voxel_id) {
                let sphere = voxel_collider_bounding_sphere(voxel_collider, pos, ori);
                let radius = sphere.radius.ceil() as u32;
                let pos_2d = sphere.center.xy().map(|e| e as i32);
                const POS_TRUNCATION_ERROR: u32 = 1;
                spatial_grid.insert(pos_2d, radius + POS_TRUNCATION_ERROR, entity);
            }
        }

        spatial_grid
    }

    fn handle_movement_and_terrain(
        &mut self,
        job: &mut Job<Sys>,
        voxel_collider_spatial_grid: &SpatialGrid,
    ) {
        let PhysicsData {
            ref read,
            ref mut write,
        } = self;

        prof_span!(guard, "insert PosVelDefer");
        // NOTE: keep in sync with join below
        (
            &read.entities,
            read.colliders.mask(),
            &write.positions,
            &write.velocities,
            write.orientations.mask(),
            write.physics_states.mask(),
            !&write.pos_vel_defers, // This is the one we are adding
            write.previous_phys_cache.mask(),
            !&read.mountings,
        )
            .join()
            .map(|t| (t.0, *t.2, *t.3))
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|(entity, pos, vel)| {
                let _ = write.pos_vel_defers.insert(entity, PosVelDefer {
                    pos: Some(pos),
                    vel: Some(vel),
                });
            });
        drop(guard);

        // Apply movement inputs
        span!(guard, "Apply movement");
        let (positions, velocities) = (&write.positions, &mut write.velocities);

        // First pass: update velocity using air resistance and gravity for each entity.
        // We do this in a first pass because it helps keep things more stable for
        // entities that are anchored to other entities (such as airships).
        (
            positions,
            velocities,
            read.stickies.maybe(),
            &read.bodies,
            read.character_states.maybe(),
            &write.physics_states,
            &read.masses,
            &read.densities,
            !&read.mountings,
        )
            .par_join()
            .for_each_init(
                || {
                    prof_span!(guard, "velocity update rayon job");
                    guard
                },
                |_guard,
                 (
                    pos,
                    vel,
                    sticky,
                    body,
                    character_state,
                    physics_state,
                    mass,
                    density,
                    _,
                )| {
                    let in_loaded_chunk = read
                        .terrain
                        .get_key(read.terrain.pos_key(pos.0.map(|e| e.floor() as i32)))
                        .is_some();

                    // Apply physics only if in a loaded chunk
                    if in_loaded_chunk
                    // And not already stuck on a block (e.g., for arrows)
                    && !(physics_state.on_surface().is_some() && sticky.is_some())
                    {
                        // Clamp dt to an effective 10 TPS, to prevent gravity from slamming the
                        // players into the floor when stationary if other systems cause the server
                        // to lag (as observed in the 0.9 release party).
                        let dt = DeltaTime(read.dt.0.min(0.1));

                        match physics_state.in_fluid {
                            None => {
                                vel.0.z -= dt.0 * GRAVITY;
                            },
                            Some(fluid) => {
                                let wings = match character_state {
                                    Some(&CharacterState::Glide(states::glide::Data {
                                        aspect_ratio,
                                        planform_area,
                                        ori,
                                        ..
                                    })) => Some(Wings {
                                        aspect_ratio,
                                        planform_area,
                                        ori,
                                    }),

                                    _ => None,
                                };
                                vel.0 = integrate_forces(
                                    &dt,
                                    *vel,
                                    (body, wings.as_ref()),
                                    density,
                                    mass,
                                    &fluid,
                                    GRAVITY,
                                )
                                .0
                            },
                        }
                    }
                },
            );
        drop(guard);
        job.cpu_stats.measure(ParMode::Single);

        // Second pass: resolve collisions for terrain-like entities, this is required
        // in order to update their positions before resolving collisions for
        // non-terrain-like entities, since otherwise, collision is resolved
        // based on where the terrain-like entity was in the previous frame.
        Self::resolve_et_collision(job, read, write, voxel_collider_spatial_grid, true);

        // Third pass: resolve collisions for non-terrain-like entities
        Self::resolve_et_collision(job, read, write, voxel_collider_spatial_grid, false);
    }

    fn resolve_et_collision(
        job: &mut Job<Sys>,
        read: &PhysicsRead,
        write: &mut PhysicsWrite,
        voxel_collider_spatial_grid: &SpatialGrid,
        terrain_like_entities: bool,
    ) {
        let (positions, velocities, previous_phys_cache, orientations) = (
            &write.positions,
            &write.velocities,
            &write.previous_phys_cache,
            &write.orientations,
        );
        span!(guard, "Apply terrain collision");
        job.cpu_stats.measure(ParMode::Rayon);
        let (land_on_grounds, mut outcomes) = (
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
            &mut write.pos_vel_defers,
            previous_phys_cache,
            !&read.mountings,
        )
            .par_join()
            .filter(|tuple| matches!(tuple.3, Collider::Voxel { .. }) == terrain_like_entities)
            .map_init(
                || {
                    prof_span!(guard, "physics e<>t rayon job");
                    guard
                },
                |_guard,
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
                    pos_vel_defer,
                    _previous_cache,
                    _,
                )| {
                    let mut land_on_ground = None;
                    let mut outcomes = Vec::new();
                    // Defer the writes of positions and velocities to allow an inner loop over
                    // terrain-like entities
                    let old_vel = *vel;
                    let mut vel = *vel;

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
                    let block_snap =
                        body.map_or(false, |b| !matches!(b, Body::Object(_) | Body::Ship(_)));
                    let climbing =
                        character_state.map_or(false, |cs| matches!(cs, CharacterState::Climb(_)));

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
                                |entity, vel| land_on_ground = Some((entity, vel)),
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
                                |entity, vel| land_on_ground = Some((entity, vel)),
                            );

                            // Sticky things shouldn't move when on a surface
                            if physics_state.on_surface().is_some() && sticky.is_some() {
                                vel.0 = physics_state.ground_vel;
                            }

                            tgt_pos = cpos.0;
                        },
                        Collider::Point => {
                            let mut pos = *pos;

                            // If the velocity is exactly 0, a raycast may not pick up the current
                            // block. Handle this.
                            let (dist, block) = if let Some(block) = read
                                .terrain
                                .get(pos.0.map(|e| e.floor() as i32))
                                .ok()
                                .filter(|b| b.is_filled())
                            // TODO: `is_solid`, when arrows are special-cased
                            {
                                (0.0, Some(block))
                            } else {
                                let (dist, block) = read
                                    .terrain
                                    .ray(pos.0, pos.0 + pos_delta)
                                    .until(|block: &Block| block.is_filled())
                                    .ignore_error()
                                    .cast();
                                (dist, block.unwrap()) // Can't fail since we do ignore_error above
                            };

                            pos.0 += pos_delta.try_normalized().unwrap_or_else(Vec3::zero) * dist;

                            // TODO: Not all projectiles should count as sticky!
                            if sticky.is_some() {
                                if let Some((projectile, body)) = read
                                    .projectiles
                                    .get(entity)
                                    .filter(|_| vel.0.magnitude_squared() > 1.0 && block.is_some())
                                    .zip(read.bodies.get(entity).copied())
                                {
                                    outcomes.push(Outcome::ProjectileHit {
                                        pos: pos.0,
                                        body,
                                        vel: vel.0,
                                        source: projectile.owner,
                                        target: None,
                                    });
                                }
                            }

                            if block.is_some() {
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

                                // Sticky things shouldn't move
                                if sticky.is_some() {
                                    vel.0 = physics_state.ground_vel;
                                }
                            }

                            physics_state.in_fluid = read
                                .terrain
                                .get(pos.0.map(|e| e.floor() as i32))
                                .ok()
                                .and_then(|vox| vox.is_liquid().then_some(1.0))
                                .map(|depth| Fluid::Water {
                                    depth,
                                    vel: Vel::zero(),
                                })
                                .or_else(|| match physics_state.in_fluid {
                                    Some(Fluid::Water { .. }) | None => Some(Fluid::Air {
                                        elevation: pos.0.z,
                                        vel: Vel::default(),
                                    }),
                                    fluid => fluid,
                                });

                            tgt_pos = pos.0;
                        },
                    }

                    // Compute center and radius of tick path bounding sphere for the entity
                    // for broad checks of whether it will collide with a voxel collider
                    let path_sphere = {
                        // TODO: duplicated with maintain_pushback_cache, make a common function
                        // to call to compute all this info?
                        let z_limits = calc_z_limit(character_state, Some(collider));
                        let z_limits = (z_limits.0 * scale, z_limits.1 * scale);
                        let half_height = (z_limits.1 - z_limits.0) / 2.0;

                        let entity_center = pos.0 + (z_limits.0 + half_height) * Vec3::unit_z();
                        let path_center = entity_center + pos_delta / 2.0;

                        let flat_radius = collider.get_radius() * scale;
                        let radius = (flat_radius.powi(2) + half_height.powi(2)).sqrt();
                        let path_bounding_radius = radius + (pos_delta / 2.0).magnitude();

                        Sphere {
                            center: path_center,
                            radius: path_bounding_radius,
                        }
                    };
                    // Collide with terrain-like entities
                    let query_center = path_sphere.center.xy();
                    let query_radius = path_sphere.radius;
                    voxel_collider_spatial_grid
                        .in_circle_aabr(query_center, query_radius)
                        .filter_map(|entity| {
                            positions
                                .get(entity)
                                .and_then(|l| velocities.get(entity).map(|r| (l, r)))
                                .and_then(|l| previous_phys_cache.get(entity).map(|r| (l, r)))
                                .and_then(|l| read.colliders.get(entity).map(|r| (l, r)))
                                .and_then(|l| orientations.get(entity).map(|r| (l, r)))
                                .map(|((((pos, vel), previous_cache), collider), ori)| {
                                    (entity, pos, vel, previous_cache, collider, ori)
                                })
                        })
                        .for_each(
                            |(
                                entity_other,
                                pos_other,
                                vel_other,
                                previous_cache_other,
                                collider_other,
                                ori_other,
                            )| {
                                if entity == entity_other {
                                    return;
                                }

                                let voxel_id = if let Collider::Voxel { id } = collider_other {
                                    id
                                } else {
                                    return;
                                };

                                // use bounding cylinder regardless of our collider
                                // TODO: extract point-terrain collision above to its own
                                // function
                                let radius = collider.get_radius();
                                let (z_min, z_max) = collider.get_z_limits(1.0);

                                let radius = radius.min(0.45) * scale;
                                let z_min = z_min * scale;
                                let z_max = z_max.clamped(1.2, 1.95) * scale;

                                if let Some(voxel_collider) =
                                    VOXEL_COLLIDER_MANIFEST.read().colliders.get(voxel_id)
                                {
                                    // TODO: cache/precompute sphere?
                                    let voxel_sphere = voxel_collider_bounding_sphere(
                                        voxel_collider,
                                        pos_other,
                                        ori_other,
                                    );
                                    // Early check
                                    if voxel_sphere.center.distance_squared(path_sphere.center)
                                        > (voxel_sphere.radius + path_sphere.radius).powi(2)
                                    {
                                        return;
                                    }

                                    let mut physics_state_delta = physics_state.clone();
                                    // deliberately don't use scale yet here, because the
                                    // 11.0/0.8 thing is
                                    // in the comp::Scale for visual reasons
                                    let mut cpos = *pos;
                                    let wpos = cpos.0;

                                    // TODO: Cache the matrices here to avoid recomputing

                                    let transform_from =
                                        Mat4::<f32>::translation_3d(pos_other.0 - wpos)
                                            * Mat4::from(ori_other.to_quat())
                                            * Mat4::<f32>::translation_3d(
                                                voxel_collider.translation,
                                            );
                                    let transform_to = transform_from.inverted();
                                    let ori_from = Mat4::from(ori_other.to_quat());
                                    let ori_to = ori_from.inverted();

                                    // The velocity of the collider, taking into account
                                    // orientation.
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
                                            land_on_ground =
                                                Some((entity, Vel(ori_from.mul_direction(vel.0))));
                                        },
                                    );

                                    cpos.0 = transform_from.mul_point(cpos.0) + wpos;
                                    vel.0 = ori_from.mul_direction(vel.0) + vel_other;
                                    tgt_pos = cpos.0;

                                    // union in the state updates, so that the state isn't just
                                    // based on the most
                                    // recent terrain that collision was attempted with
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
                                    physics_state.in_fluid = match (
                                        physics_state.in_fluid,
                                        physics_state_delta.in_fluid,
                                    ) {
                                        (Some(x), Some(y)) => x
                                            .depth()
                                            .and_then(|xh| {
                                                y.depth()
                                                    .map(|yh| xh > yh)
                                                    .unwrap_or(true)
                                                    .then_some(x)
                                            })
                                            .or(Some(y)),
                                        (x @ Some(_), _) => x,
                                        (_, y @ Some(_)) => y,
                                        _ => None,
                                    };
                                }
                            },
                        );

                    if tgt_pos != pos.0 {
                        pos_vel_defer.pos = Some(Pos(tgt_pos));
                    } else {
                        pos_vel_defer.pos = None;
                    }

                    if vel != old_vel {
                        pos_vel_defer.vel = Some(vel);
                    } else {
                        pos_vel_defer.vel = None;
                    }

                    (land_on_ground, outcomes)
                },
            )
            .fold(
                || (Vec::new(), Vec::new()),
                |(mut land_on_grounds, mut all_outcomes), (land_on_ground, mut outcomes)| {
                    land_on_ground.map(|log| land_on_grounds.push(log));
                    all_outcomes.append(&mut outcomes);
                    (land_on_grounds, all_outcomes)
                },
            )
            .reduce(
                || (Vec::new(), Vec::new()),
                |(mut land_on_grounds_a, mut outcomes_a),
                 (mut land_on_grounds_b, mut outcomes_b)| {
                    land_on_grounds_a.append(&mut land_on_grounds_b);
                    outcomes_a.append(&mut outcomes_b);
                    (land_on_grounds_a, outcomes_a)
                },
            );
        drop(guard);
        job.cpu_stats.measure(ParMode::Single);

        write.outcomes.append(&mut outcomes);

        prof_span!(guard, "write deferred pos and vel");
        for (_, pos, vel, pos_vel_defer, _) in (
            &read.entities,
            &mut write.positions,
            &mut write.velocities,
            &mut write.pos_vel_defers,
            &read.colliders,
        )
            .join()
            .filter(|tuple| matches!(tuple.4, Collider::Voxel { .. }) == terrain_like_entities)
        {
            if let Some(new_pos) = pos_vel_defer.pos.take() {
                *pos = new_pos;
            }
            if let Some(new_vel) = pos_vel_defer.vel.take() {
                *vel = new_vel;
            }
        }
        drop(guard);

        prof_span!(guard, "record ori into phys_cache");
        for (ori, previous_phys_cache, _) in (
            &write.orientations,
            &mut write.previous_phys_cache,
            &read.colliders,
        )
            .join()
            .filter(|tuple| matches!(tuple.2, Collider::Voxel { .. }) == terrain_like_entities)
        {
            previous_phys_cache.ori = ori.to_quat();
        }
        drop(guard);

        let mut event_emitter = read.event_bus.emitter();
        land_on_grounds.into_iter().for_each(|(entity, vel)| {
            event_emitter.emit(ServerEvent::LandOnGround { entity, vel: vel.0 });
        });
    }

    fn update_cached_spatial_grid(&mut self) {
        span!(_guard, "Update cached spatial grid");
        let PhysicsData {
            ref read,
            ref mut write,
        } = self;

        let spatial_grid = &mut write.cached_spatial_grid.0;
        spatial_grid.clear();
        (
            &read.entities,
            &write.positions,
            read.scales.maybe(),
            read.colliders.maybe(),
        )
            .join()
            .for_each(|(entity, pos, scale, collider)| {
                let scale = scale.map(|s| s.0).unwrap_or(1.0);
                let radius_2d =
                    (collider.map(|c| c.get_radius()).unwrap_or(0.5) * scale).ceil() as u32;
                let pos_2d = pos.0.xy().map(|e| e as i32);
                const POS_TRUNCATION_ERROR: u32 = 1;
                spatial_grid.insert(pos_2d, radius_2d + POS_TRUNCATION_ERROR, entity);
            });
    }
}

impl<'a> System<'a> for Sys {
    type SystemData = PhysicsData<'a>;

    const NAME: &'static str = "phys";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(job: &mut Job<Self>, mut physics_data: Self::SystemData) {
        physics_data.reset();

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
        physics_data.maintain_pushback_cache();

        let spatial_grid = physics_data.construct_spatial_grid();
        physics_data.apply_pushback(job, &spatial_grid);

        let voxel_collider_spatial_grid = physics_data.construct_voxel_collider_spatial_grid();
        physics_data.handle_movement_and_terrain(job, &voxel_collider_spatial_grid);

        // Spatial grid used by other systems
        physics_data.update_cached_spatial_grid();
    }
}

#[allow(clippy::too_many_arguments)]
fn box_voxel_collision<'a, T: BaseVol<Vox = Block> + ReadVol>(
    cylinder: (f32, f32, f32), // effective collision cylinder
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
        .next()
        .is_some()
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
        while let Some((_block_pos, block_aabb, block_height)) =
            (attempts < MAX_ATTEMPTS).then(|| {
                // Calculate the player's AABB
                let player_aabb = Aabb {
                    min: pos.0 + Vec3::new(-radius, -radius, z_min),
                    max: pos.0 + Vec3::new(radius, radius, z_max),
                };

                // Determine the block that we are colliding with most (based on minimum
                // collision axis) (if we are colliding with one)
                near_iter
                    .clone()
                    // Calculate the block's position in world space
                    .map(|(i, j, k)| pos.0.map(|e| e.floor() as i32) + Vec3::new(i, j, k))
                    // Make sure the block is actually solid
                    .filter_map(|block_pos| {
                        terrain
                            .get(block_pos)
                            .ok()
                            .filter(|block| block.is_solid())
                            .map(|block| (block_pos, block))
                    })
                    // Calculate block AABB
                    .map(|(block_pos, block)| {
                        (
                            block_pos,
                            Aabb {
                                min: block_pos.map(|e| e as f32),
                                max: block_pos.map(|e| e as f32) + Vec3::new(1.0, 1.0, block.solid_height()),
                            },
                            block.solid_height(),
                        )
                    })
                    // Determine whether the block's AABB collides with the player's AABB
                    .filter(|(_, block_aabb, _)| block_aabb.collides_with_aabb(player_aabb))
                    // Find the maximum of the minimum collision axes (this bit is weird, trust me that it works)
                    .min_by_key(|(_, block_aabb, _)| {
                        ordered_float::OrderedFloat((block_aabb.center() - player_aabb.center() - Vec3::unit_z() * 0.5)
                            .map(f32::abs)
                            .sum())
                    })
            }).flatten()
        {
            // Calculate the player's AABB
            let player_aabb = Aabb {
                min: pos.0 + Vec3::new(-radius, -radius, z_min),
                max: pos.0 + Vec3::new(radius, radius, z_max),
            };

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
            // with a wall If we're being pushed out horizontally...
            if resolve_dir.z == 0.0
                // ...and the vertical resolution direction is sufficiently great...
                && dir.z < -0.1
                // ...and the space above is free...
                && !collision_with(Vec3::new(pos.0.x, pos.0.y, (pos.0.z + 0.1).ceil()), &terrain, block_true, near_iter.clone(), radius, z_range.clone())
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
                // Push the character on to the block very slightly to avoid jitter due to imprecision
                if (vel.0 * resolve_dir).xy().magnitude_squared() < 1.0f32.powi(2) {
                    pos.0 -= resolve_dir.normalized() * 0.05;
                }
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
    ) && vel.0.z <= 0.0
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

    let player_aabb = Aabb {
        min: pos.0 + Vec3::new(-radius, -radius, z_range.start),
        max: pos.0 + Vec3::new(radius, radius, z_range.end),
    };
    let player_voxel_pos = pos.0.map(|e| e.floor() as i32);

    let dirs = [
        Vec3::unit_x(),
        Vec3::unit_y(),
        -Vec3::unit_x(),
        -Vec3::unit_y(),
    ];
    let player_wall_aabbs = dirs.map(|dir| {
        let pos = pos.0 + dir * 0.01;
        Aabb {
            min: pos + Vec3::new(-radius, -radius, z_range.start),
            max: pos + Vec3::new(radius, radius, z_range.end),
        }
    });

    // Find liquid immersion and wall collision all in one round of iteration
    let mut max_liquid_z = None::<f32>;
    let mut wall_dir_collisions = [false; 4];
    near_iter.for_each(|(i, j, k)| {
        let block_pos = player_voxel_pos + Vec3::new(i, j, k);

        if let Some(block) = terrain.get(block_pos).ok().copied() {
            // Check for liquid blocks
            if block.is_liquid() {
                let liquid_aabb = Aabb {
                    min: block_pos.map(|e| e as f32),
                    // The liquid part of a liquid block always extends 1 block high.
                    max: block_pos.map(|e| e as f32) + Vec3::one(),
                };
                if player_aabb.collides_with_aabb(liquid_aabb) {
                    max_liquid_z = Some(match max_liquid_z {
                        Some(z) => z.max(liquid_aabb.max.z),
                        None => liquid_aabb.max.z,
                    });
                }
            }
            // Check for walls
            if block.is_solid() {
                let block_aabb = Aabb {
                    min: block_pos.map(|e| e as f32),
                    max: block_pos.map(|e| e as f32) + Vec3::new(1.0, 1.0, block.solid_height()),
                };

                for dir in 0..4 {
                    if player_wall_aabbs[dir].collides_with_aabb(block_aabb) {
                        wall_dir_collisions[dir] = true;
                    }
                }
            }
        }
    });

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
    if physics_state.on_ground || (physics_state.on_wall.is_some() && climbing) {
        vel.0 *= (1.0 - FRIC_GROUND.min(1.0)).powf(dt.0 * 60.0);
        physics_state.ground_vel = ground_vel;
    }

    physics_state.in_fluid = max_liquid_z
        .map(|max_z| max_z - pos.0.z) // NOTE: assumes min_z == 0.0
        .map(|depth| {
            physics_state
                .in_liquid()
                // This is suboptimal because it doesn't check for true depth,
                // so it can cause problems for situations like swimming down
                // a river and spawning or teleporting in(/to) water
                .map(|old_depth| (old_depth + old_pos.z - pos.0.z).max(depth))
                .unwrap_or(depth)
        })
        .map(|depth| Fluid::Water {
            depth,
            vel: Vel::zero(),
        })
        .or_else(|| match physics_state.in_fluid {
            Some(Fluid::Water { .. }) | None => Some(Fluid::Air {
                elevation: pos.0.z,
                vel: Vel::default(),
            }),
            fluid => fluid,
        });
}

fn voxel_collider_bounding_sphere(
    voxel_collider: &VoxelCollider,
    pos: &Pos,
    ori: &Ori,
) -> Sphere<f32, f32> {
    let origin_offset = voxel_collider.translation;
    use common::vol::SizedVol;
    let lower_bound = voxel_collider.dyna.lower_bound().map(|e| e as f32);
    let upper_bound = voxel_collider.dyna.upper_bound().map(|e| e as f32);
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
        radius,
    }
}
