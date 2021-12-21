use common::{
    comp::{
        body::ship::figuredata::{VoxelCollider, VOXEL_COLLIDER_MANIFEST},
        fluid_dynamics::{Fluid, LiquidKind, Wings},
        Body, CharacterState, Collider, Density, Mass, Mounting, Ori, PhysicsState, Pos,
        PosVelOriDefer, PreviousPhysCache, Projectile, Scale, Stats, Sticky, Vel,
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
                // Multiply by a factor to prevent full stop,
                // as this can cause things to get stuck in high-density medium
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

fn calc_z_limit(char_state_maybe: Option<&CharacterState>, collider: &Collider) -> (f32, f32) {
    let modifier = if char_state_maybe.map_or(false, |c_s| c_s.is_dodge() || c_s.is_glide()) {
        0.5
    } else {
        1.0
    };
    collider.get_z_limits(modifier)
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
    char_states: ReadStorage<'a, CharacterState>,
    bodies: ReadStorage<'a, Body>,
    character_states: ReadStorage<'a, CharacterState>,
    densities: ReadStorage<'a, Density>,
    stats: ReadStorage<'a, Stats>,
}

#[derive(SystemData)]
pub struct PhysicsWrite<'a> {
    physics_metrics: WriteExpect<'a, PhysicsMetrics>,
    cached_spatial_grid: Write<'a, common::CachedSpatialGrid>,
    physics_states: WriteStorage<'a, PhysicsState>,
    positions: WriteStorage<'a, Pos>,
    velocities: WriteStorage<'a, Vel>,
    pos_vel_ori_defers: WriteStorage<'a, PosVelOriDefer>,
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
            &self.read.colliders,
            &self.write.velocities,
            &self.write.positions,
            !&self.write.previous_phys_cache,
            !&self.read.mountings,
        )
            .join()
            .map(|(e, _, _, _, _, _)| e)
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
                    neighborhood_radius: 0.0,
                    origins: None,
                    pos: None,
                    ori: Quaternion::identity(),
                });
        }

        // Update PreviousPhysCache
        for (_, vel, position, ori, mut phys_cache, collider, scale, cs, _) in (
            &self.read.entities,
            &self.write.velocities,
            &self.write.positions,
            &self.write.orientations,
            &mut self.write.previous_phys_cache,
            &self.read.colliders,
            self.read.scales.maybe(),
            self.read.char_states.maybe(),
            !&self.read.mountings,
        )
            .join()
        {
            let scale = scale.map(|s| s.0).unwrap_or(1.0);
            let z_limits = calc_z_limit(cs, collider);
            let (z_min, z_max) = z_limits;
            let (z_min, z_max) = (z_min * scale, z_max * scale);
            let half_height = (z_max - z_min) / 2.0;

            phys_cache.velocity_dt = vel.0 * self.read.dt.0;
            let entity_center = position.0 + Vec3::new(0.0, 0.0, z_min + half_height);
            let flat_radius = collider.bounding_radius() * scale;
            let radius = (flat_radius.powi(2) + half_height.powi(2)).sqrt();

            // Move center to the middle between OLD and OLD+VEL_DT
            // so that we can reduce the collision_boundary.
            phys_cache.center = entity_center + phys_cache.velocity_dt / 2.0;
            phys_cache.collision_boundary = radius + (phys_cache.velocity_dt / 2.0).magnitude();
            phys_cache.scale = scale;
            phys_cache.scaled_radius = flat_radius;

            let neighborhood_radius = match collider {
                Collider::CapsulePrism { radius, .. } => radius * scale,
                Collider::Voxel { .. } | Collider::Volume(_) | Collider::Point => flat_radius,
            };
            phys_cache.neighborhood_radius = neighborhood_radius;

            let ori = ori.to_quat();
            let origins = match collider {
                Collider::CapsulePrism { p0, p1, .. } => {
                    let a = p1 - p0;
                    let len = a.magnitude();
                    // If origins are close enough, our capsule prism is cylinder
                    // with one origin which we don't even need to rotate.
                    //
                    // Other advantage of early-return is that we don't
                    // later divide by zero and return NaN
                    if len < std::f32::EPSILON * 10.0 {
                        Some((*p0, *p0))
                    } else {
                        // Apply orientation to origins of prism.
                        //
                        // We do this by building line between them,
                        // rotate it and then split back to origins.
                        // (Otherwise we will need to do the same with each
                        // origin).
                        //
                        // Cast it to 3d and then convert it back to 2d
                        // to apply quaternion.
                        let a = a.with_z(0.0);
                        let a = ori * a;
                        let a = a.xy();
                        // Previous operation could shrink x and y coordinates
                        // if orientation had Z parameter.
                        // Make sure we have the same length as before
                        // (and scale it, while we on it).
                        let a = a.normalized() * scale * len;
                        let p0 = -a / 2.0;
                        let p1 = a / 2.0;

                        Some((p0, p1))
                    }
                },
                Collider::Voxel { .. } | Collider::Volume(_) | Collider::Point => None,
            };
            phys_cache.origins = origins;
            phys_cache.ori = ori;
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
        for (entity, pos, phys_cache, _, _, _, _) in (
            &read.entities,
            &write.positions,
            &write.previous_phys_cache,
            write.velocities.mask(),
            !&read.projectiles, // Not needed because they are skipped in the inner loop below
            !&read.mountings,
            read.colliders.mask(),
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
            &read.colliders,
            !&read.mountings,
            read.stickies.maybe(),
            &mut write.physics_states,
            // TODO: if we need to avoid collisions for other things consider
            // moving whether it should interact into the collider component
            // or into a separate component.
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
                    let is_mid_air = physics.on_surface().is_none();
                    let mut entity_entity_collision_checks = 0;
                    let mut entity_entity_collisions = 0;

                    // TODO: quick fix for bad performance. At extrememly high
                    // velocities use oriented rectangles at some threshold of
                    // displacement/radius to query the spatial grid and limit
                    // max displacement per tick somehow.
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
                            let uid = read.uids.get(entity)?;
                            let pos = positions.get(entity)?;
                            let previous_cache = previous_phys_cache.get(entity)?;
                            let mass = read.masses.get(entity)?;
                            let collider = read.colliders.get(entity)?;

                            Some((
                                entity,
                                uid,
                                pos,
                                previous_cache,
                                mass,
                                collider,
                                read.char_states.get(entity),
                            ))
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

                                let mut collision_registered = false;

                                for i in 0..increments {
                                    let factor = i as f32 * step_delta;
                                    // We are not interested if collision succeed
                                    // or no as of now.
                                    // Collision reaction is done inside.
                                    let _ = resolve_e2e_collision(
                                        // utility variables for our entity
                                        &mut collision_registered,
                                        &mut entity_entity_collisions,
                                        factor,
                                        physics,
                                        char_state_maybe,
                                        &mut vel_delta,
                                        step_delta,
                                        // physics flags
                                        is_mid_air,
                                        is_sticky,
                                        is_projectile,
                                        // entity we colliding with
                                        *other,
                                        // symetrical collider context
                                        pos,
                                        pos_other,
                                        previous_cache,
                                        previous_cache_other,
                                        z_limits,
                                        z_limits_other,
                                        collider,
                                        collider_other,
                                        *mass,
                                        *mass_other,
                                    );
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

        let voxel_colliders_manifest = VOXEL_COLLIDER_MANIFEST.read();

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
            let vol = match collider {
                Collider::Voxel { id } => voxel_colliders_manifest.colliders.get(&*id),
                Collider::Volume(vol) => Some(&**vol),
                _ => None,
            };

            if let Some(vol) = vol {
                let sphere = voxel_collider_bounding_sphere(vol, pos, ori);
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

        prof_span!(guard, "insert PosVelOriDefer");
        // NOTE: keep in sync with join below
        (
            &read.entities,
            read.colliders.mask(),
            &write.positions,
            &write.velocities,
            &write.orientations,
            write.orientations.mask(),
            write.physics_states.mask(),
            !&write.pos_vel_ori_defers, // This is the one we are adding
            write.previous_phys_cache.mask(),
            !&read.mountings,
        )
            .join()
            .map(|t| (t.0, *t.2, *t.3, *t.4))
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|(entity, pos, vel, ori)| {
                let _ = write.pos_vel_ori_defers.insert(entity, PosVelOriDefer {
                    pos: Some(pos),
                    vel: Some(vel),
                    ori: Some(ori),
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
                        // Clamp dt to an effective 10 TPS, to prevent gravity
                        // from slamming the players into the floor when
                        // stationary if other systems cause the server
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

        // Update cached 'old' physics values to the current values ready for the next
        // tick
        prof_span!(guard, "record ori into phys_cache");
        for (pos, ori, previous_phys_cache, _) in (
            &write.positions,
            &write.orientations,
            &mut write.previous_phys_cache,
            &read.colliders,
        )
            .join()
        {
            // Note: updating ori with the rest of the cache values above was attempted but
            // it did not work (investigate root cause?)
            previous_phys_cache.pos = Some(*pos);
            previous_phys_cache.ori = ori.to_quat();
        }
        drop(guard);
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
            &mut write.pos_vel_ori_defers,
            previous_phys_cache,
            !&read.mountings,
        )
            .par_join()
            .filter(|tuple| tuple.3.is_voxel() == terrain_like_entities)
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
                    ori,
                    body,
                    character_state,
                    mut physics_state,
                    pos_vel_ori_defer,
                    previous_cache,
                    _,
                )| {
                    let mut land_on_ground = None;
                    let mut outcomes = Vec::new();
                    // Defer the writes of positions, velocities and orientations
                    // to allow an inner loop over terrain-like entities.
                    let old_vel = *vel;
                    let mut vel = *vel;
                    let old_ori = *ori;
                    let mut ori = *ori;

                    let scale = if collider.is_voxel() {
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

                    // What's going on here?
                    // Because collisions need to be resolved against multiple
                    // colliders, this code takes the current position and
                    // propagates it forward according to velocity to find a
                    // 'target' position.
                    //
                    // This is where we'd ideally end up at the end of the tick,
                    // assuming no collisions. Then, we refine this target by
                    // stepping from the original position to the target for
                    // every obstacle, refining the target position as we go.
                    //
                    // It's not perfect, but it works pretty well in practice.
                    // Oddities can occur on the intersection between multiple
                    // colliders, but it's not like any game physics system
                    // resolves these sort of things well anyway.
                    // At the very least, we don't do things that result in glitchy
                    // velocities or entirely broken position snapping.
                    let mut tgt_pos = pos.0 + pos_delta;

                    let was_on_ground = physics_state.on_ground.is_some();
                    let block_snap =
                        body.map_or(false, |b| !matches!(b, Body::Object(_) | Body::Ship(_)));
                    let climbing =
                        character_state.map_or(false, |cs| matches!(cs, CharacterState::Climb(_)));

                    match &collider {
                        Collider::Voxel { .. } | Collider::Volume(_) => {
                            // For now, treat entities with voxel colliders
                            // as their bounding cylinders for the purposes of
                            // colliding them with terrain.
                            //
                            // Additionally, multiply radius by 0.1 to make
                            // the cylinder smaller to avoid lag.
                            let radius = collider.bounding_radius() * scale * 0.1;
                            let (_, z_max) = collider.get_z_limits(scale);
                            let z_min = 0.0;

                            let mut cpos = *pos;
                            let cylinder = (radius, z_min, z_max);
                            box_voxel_collision(
                                cylinder,
                                &*read.terrain,
                                entity,
                                &mut cpos,
                                tgt_pos,
                                &mut vel,
                                physics_state,
                                Vec3::zero(),
                                &read.dt,
                                was_on_ground,
                                block_snap,
                                climbing,
                                |entity, vel| land_on_ground = Some((entity, vel)),
                                read,
                            );
                            tgt_pos = cpos.0;
                        },
                        Collider::CapsulePrism {
                            z_min: _,
                            z_max,
                            p0: _,
                            p1: _,
                            radius: _,
                        } => {
                            // Scale collider
                            let radius = collider.bounding_radius().min(0.45) * scale;
                            let z_min = 0.0;
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
                                physics_state,
                                Vec3::zero(),
                                &read.dt,
                                was_on_ground,
                                block_snap,
                                climbing,
                                |entity, vel| land_on_ground = Some((entity, vel)),
                                read,
                            );

                            // Sticky things shouldn't move when on a surface
                            if physics_state.on_surface().is_some() && sticky.is_some() {
                                vel.0 = physics_state.ground_vel;
                            }

                            tgt_pos = cpos.0;
                        },
                        Collider::Point => {
                            let mut pos = *pos;

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
                            if sticky.is_some() {
                                if let Some((projectile, body)) = read
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
                            }

                            if block.is_some() {
                                let block_center = pos.0.map(|e| e.floor()) + 0.5;
                                let block_rpos = (pos.0 - block_center)
                                    .try_normalized()
                                    .unwrap_or_else(Vec3::zero);

                                // See whether we're on the top/bottom of a block,
                                // or the side
                                if block_rpos.z.abs()
                                    > block_rpos.xy().map(|e| e.abs()).reduce_partial_max()
                                {
                                    if block_rpos.z > 0.0 {
                                        physics_state.on_ground = block.copied();
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

                            tgt_pos = pos.0;
                        },
                    }

                    // Compute center and radius of tick path bounding sphere
                    // for the entity for broad checks of whether it will
                    // collide with a voxel collider
                    let path_sphere = {
                        // TODO: duplicated with maintain_pushback_cache,
                        // make a common function to call to compute all this info?
                        let z_limits = calc_z_limit(character_state, collider);
                        let z_limits = (z_limits.0 * scale, z_limits.1 * scale);
                        let half_height = (z_limits.1 - z_limits.0) / 2.0;

                        let entity_center = pos.0 + (z_limits.0 + half_height) * Vec3::unit_z();
                        let path_center = entity_center + pos_delta / 2.0;

                        let flat_radius = collider.bounding_radius() * scale;
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

                    let voxel_colliders_manifest = VOXEL_COLLIDER_MANIFEST.read();

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

                                let voxel_collider = match collider_other {
                                    Collider::Voxel { id } => {
                                        voxel_colliders_manifest.colliders.get(id)
                                    },
                                    Collider::Volume(vol) => Some(&**vol),
                                    _ => None,
                                };

                                // use bounding cylinder regardless of our collider
                                // TODO: extract point-terrain collision above to its own
                                // function
                                let radius = collider.bounding_radius();
                                let (_, z_max) = collider.get_z_limits(1.0);

                                let radius = radius.min(0.45) * scale;
                                let z_min = 0.0;
                                let z_max = z_max.clamped(1.2, 1.95) * scale;

                                if let Some(voxel_collider) = voxel_collider {
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

                                    let transform_last_from = Mat4::<f32>::translation_3d(
                                        previous_cache_other.pos.unwrap_or(*pos_other).0
                                            - previous_cache.pos.unwrap_or(Pos(wpos)).0,
                                    ) * Mat4::from(
                                        previous_cache_other.ori,
                                    ) * Mat4::<f32>::translation_3d(
                                        voxel_collider.translation,
                                    );
                                    let transform_last_to = transform_last_from.inverted();

                                    let transform_from =
                                        Mat4::<f32>::translation_3d(pos_other.0 - wpos)
                                            * Mat4::from(ori_other.to_quat())
                                            * Mat4::<f32>::translation_3d(
                                                voxel_collider.translation,
                                            );
                                    let transform_to = transform_from.inverted();

                                    let ori_last_from = Mat4::from(previous_cache_other.ori);
                                    let ori_last_to = ori_last_from.inverted();

                                    let ori_from = Mat4::from(ori_other.to_quat());

                                    // The velocity of the collider, taking into account
                                    // orientation.
                                    let pos_rel = (Mat4::<f32>::translation_3d(Vec3::zero())
                                        * Mat4::from(ori_other.to_quat())
                                        * Mat4::<f32>::translation_3d(voxel_collider.translation))
                                    .inverted()
                                    .mul_point(wpos - pos_other.0);
                                    let rpos_last = (Mat4::<f32>::translation_3d(Vec3::zero())
                                        * Mat4::from(previous_cache_other.ori)
                                        * Mat4::<f32>::translation_3d(voxel_collider.translation))
                                    .mul_point(pos_rel);
                                    let vel_other = vel_other.0
                                        + (wpos - (pos_other.0 + rpos_last)) / read.dt.0;

                                    cpos.0 = transform_last_to.mul_point(Vec3::zero());
                                    vel.0 = ori_last_to.mul_direction(vel.0 - vel_other);
                                    let cylinder = (radius, z_min, z_max);
                                    box_voxel_collision(
                                        cylinder,
                                        &voxel_collider.volume(),
                                        entity,
                                        &mut cpos,
                                        transform_to.mul_point(tgt_pos - wpos),
                                        &mut vel,
                                        &mut physics_state_delta,
                                        ori_last_to.mul_direction(vel_other),
                                        &read.dt,
                                        was_on_ground,
                                        block_snap,
                                        climbing,
                                        |entity, vel| {
                                            land_on_ground =
                                                Some((entity, Vel(ori_from.mul_direction(vel.0))));
                                        },
                                        read,
                                    );

                                    cpos.0 = transform_from.mul_point(cpos.0) + wpos;
                                    vel.0 = ori_from.mul_direction(vel.0) + vel_other;
                                    tgt_pos = cpos.0;

                                    // union in the state updates, so that the state isn't just
                                    // based on the most
                                    // recent terrain that collision was attempted with
                                    if physics_state_delta.on_ground.is_some() {
                                        physics_state.ground_vel = vel_other;

                                        // Rotate if on ground
                                        ori = ori.rotated(
                                            ori_other.to_quat()
                                                * previous_cache_other.ori.inverse(),
                                        );
                                    }
                                    physics_state.on_ground =
                                        physics_state.on_ground.or(physics_state_delta.on_ground);
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
                        pos_vel_ori_defer.pos = Some(Pos(tgt_pos));
                    } else {
                        pos_vel_ori_defer.pos = None;
                    }
                    if vel != old_vel {
                        pos_vel_ori_defer.vel = Some(vel);
                    } else {
                        pos_vel_ori_defer.vel = None;
                    }
                    if ori != old_ori {
                        pos_vel_ori_defer.ori = Some(ori);
                    } else {
                        pos_vel_ori_defer.ori = None;
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
        for (_, pos, vel, ori, pos_vel_ori_defer, _) in (
            &read.entities,
            &mut write.positions,
            &mut write.velocities,
            &mut write.orientations,
            &mut write.pos_vel_ori_defers,
            &read.colliders,
        )
            .join()
            .filter(|tuple| tuple.5.is_voxel() == terrain_like_entities)
        {
            if let Some(new_pos) = pos_vel_ori_defer.pos.take() {
                *pos = new_pos;
            }
            if let Some(new_vel) = pos_vel_ori_defer.vel.take() {
                *vel = new_vel;
            }
            if let Some(new_ori) = pos_vel_ori_defer.ori.take() {
                *ori = new_ori;
            }
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
                    (collider.map(|c| c.bounding_radius()).unwrap_or(0.5) * scale).ceil() as u32;
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

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
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
    read: &PhysicsRead,
) {
    // FIXME: Review these
    #![allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
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
        hit: impl Fn(&Block) -> bool,
        near_aabb: Aabb<i32>,
        radius: f32,
        z_range: Range<f32>,
    ) -> bool {
        let player_aabb = player_aabb(pos, radius, z_range);

        // Calculate the world space near aabb
        let near_aabb = move_aabb(near_aabb, pos);

        let mut collision = false;
        // TODO: could short-circuit here
        terrain.for_each_in(near_aabb, |block_pos, block| {
            if block.is_solid() && hit(&block) {
                let block_aabb = Aabb {
                    min: block_pos.map(|e| e as f32),
                    max: block_pos.map(|e| e as f32) + Vec3::new(1.0, 1.0, block.solid_height()),
                };
                if player_aabb.collides_with_aabb(block_aabb) {
                    collision = true;
                }
            }
        });

        collision
    }

    // Should be easy to just make clippy happy if we want?
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn always_hits(_: &Block) -> bool { true }

    let (radius, z_min, z_max) = cylinder;

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
    let increments = ((pos_delta.map(|e| e.abs()).reduce_partial_max() / 0.3).ceil() as usize)
        .clamped(1, MAX_INCREMENTS);
    let old_pos = pos.0;
    for _ in 0..increments {
        //prof_span!("increment");
        const MAX_ATTEMPTS: usize = 16;
        pos.0 += pos_delta / increments as f32;

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
                ordered_float::OrderedFloat(
                    (block_aabb.center() - player_aabb.center() - Vec3::unit_z() * 0.5)
                        .map(f32::abs)
                        .sum(),
                )
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
                    if block_aabb.collides_with_aabb(player_aabb) {
                        most_colliding = match most_colliding {
                            // Select the minimum of the value from `player_overlap`
                            other @ Some((_, other_block_aabb, _))
                                if {
                                    // TODO: comment below is outdated (as of ~1 year ago)
                                    // Find the maximum of the minimum collision axes (this bit
                                    // is weird, trust me that it works)
                                    player_overlap(block_aabb) >= player_overlap(other_block_aabb)
                                } =>
                            {
                                other
                            },
                            _ => Some((block_pos, block_aabb, block)),
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

                if !was_on_ground {
                    land_on_ground(entity, *vel);
                }
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
                    always_hits,
                    near_aabb,
                    radius,
                    z_range.clone(),
                )
            }
            // ...and there is a collision with a block beneath our current hitbox...
            && {
                //prof_span!("collision beneath");
                collision_with(
                    pos.0 + resolve_dir - Vec3::unit_z() * 1.25,
                    &terrain,
                    always_hits,
                    near_aabb,
                    radius,
                    z_range.clone(),
                )
            } {
                // ...block-hop!
                pos.0.z = pos.0.z.max(block_aabb.max.z);
                vel.0.z = vel.0.z.max(0.0);
                // Push the character on to the block very slightly
                // to avoid jitter due to imprecision
                if (vel.0 * resolve_dir).xy().magnitude_squared() < 1.0_f32.powi(2) {
                    pos.0 -= resolve_dir.normalized() * 0.05;
                }
                on_ground = Some(block);
                break;
            }

            // If not, correct the velocity
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
    } else if vel.0.z <= 0.0 && was_on_ground && block_snap && {
        //prof_span!("snap check");
        collision_with(
            pos.0 - Vec3::unit_z() * 1.1,
            &terrain,
            always_hits,
            near_aabb,
            radius,
            z_range.clone(),
        )
    } {
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
                if player_wall_aabbs[dir].collides_with_aabb(block_aabb) {
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

    let ground_fric = physics_state
        .on_ground
        .map(|b| b.get_friction())
        .unwrap_or(0.0);
    let wall_fric = if physics_state.on_wall.is_some() && climbing {
        FRIC_GROUND
    } else {
        0.0
    };
    let fric = ground_fric.max(wall_fric);
    if fric > 0.0 {
        vel.0 *= (1.0 - fric.min(1.0) * fric_mod).powf(dt.0 * 60.0);
        physics_state.ground_vel = ground_vel;
    }

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

            Fluid::Liquid {
                kind,
                depth: new_depth,
                vel: Vel::zero(),
            }
        })
        .or_else(|| match physics_state.in_fluid {
            Some(Fluid::Liquid { .. }) | None => Some(Fluid::Air {
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
        radius,
    }
}

/// Returns whether interesction between entities occured
#[allow(clippy::too_many_arguments)]
fn resolve_e2e_collision(
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
    is_projectile: bool,
    // entity we colliding with
    other: Uid,
    // symetrical collider context
    pos: &Pos,
    pos_other: &Pos,
    previous_cache: &PreviousPhysCache,
    previous_cache_other: &PreviousPhysCache,
    z_limits: (f32, f32),
    z_limits_other: (f32, f32),
    collider: &Collider,
    collider_other: &Collider,
    mass: Mass,
    mass_other: Mass,
) -> bool {
    // Find the distance betwen our collider and
    // collider we collide with and get vector of pushback.
    //
    // If we aren't colliding, just skip step.

    // Get positions
    let pos = pos.0 + previous_cache.velocity_dt * factor;
    let pos_other = pos_other.0 + previous_cache_other.velocity_dt * factor;

    // Compare Z ranges
    let (z_min, z_max) = z_limits;
    let ceiling = pos.z + z_max * previous_cache.scale;
    let floor = pos.z + z_min * previous_cache.scale;

    let (z_min_other, z_max_other) = z_limits_other;
    let ceiling_other = pos_other.z + z_max_other * previous_cache_other.scale;
    let floor_other = pos_other.z + z_min_other * previous_cache_other.scale;

    let in_z_range = ceiling >= floor_other && floor <= ceiling_other;

    if !in_z_range {
        return false;
    }

    let ours = ColliderContext {
        pos,
        previous_cache,
    };
    let theirs = ColliderContext {
        pos: pos_other,
        previous_cache: previous_cache_other,
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
        physics.touch_entities.insert(other);
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
    let forced_movement = matches!(char_state_maybe, Some(cs) if cs.is_forced_movement());

    // Don't apply repulsive force to projectiles,
    // or if we're colliding with a terrain-like entity,
    // or if we are a terrain-like entity.
    //
    // Don't apply force when entity is a sticky which is on the ground
    // (or on the wall).
    if !forced_movement
        && (!is_sticky || is_mid_air)
        && diff.magnitude_squared() > 0.0
        && !is_projectile
        && !collider_other.is_voxel()
        && !collider.is_voxel()
    {
        const ELASTIC_FORCE_COEFFICIENT: f32 = 400.0;
        let mass_coefficient = mass_other.0 / (mass.0 + mass_other.0);
        let distance_coefficient = collision_dist - diff.magnitude();
        let force = ELASTIC_FORCE_COEFFICIENT * distance_coefficient * mass_coefficient;

        *vel_delta += Vec3::from(diff.normalized()) * force * step_delta;
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
    const DIFF_THRESHOLD: f32 = std::f32::EPSILON;
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
    #![allow(clippy::many_single_char_names)]

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
        // TODO: can use postfix .into_iter() when switching to Rust 2021
        IntoIterator::into_iter([
            (n.projected_point(m.start), m.start),
            (n.projected_point(m.end), m.end),
            (n.start, m.projected_point(n.start)),
            (n.end, m.projected_point(n.end)),
        ])
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
    // TODO: can code beloew be deduplicated? :think:
    let we = c0.pos.xy();
    let other = c1.pos.xy();
    if c0.previous_cache.scaled_radius > c1.previous_cache.scaled_radius {
        let our_radius = c0.previous_cache.neighborhood_radius;
        let their_radius = c1.previous_cache.scaled_radius;
        let collision_dist = our_radius + their_radius;

        let (p0_offset, p1_offset) = match c0.previous_cache.origins {
            Some(origins) => origins,
            None => return (we - other, collision_dist),
        };
        let segment = LineSegment2 {
            start: we + p0_offset,
            end: we + p1_offset,
        };

        let projection = segment.projected_point(other) - other;

        (projection, collision_dist)
    } else {
        let our_radius = c0.previous_cache.scaled_radius;
        let their_radius = c1.previous_cache.neighborhood_radius;
        let collision_dist = our_radius + their_radius;

        let (p0_offset_other, p1_offset_other) = match c1.previous_cache.origins {
            Some(origins) => origins,
            None => return (we - other, collision_dist),
        };
        let segment_other = LineSegment2 {
            start: other + p0_offset_other,
            end: other + p1_offset_other,
        };

        let projection = we - segment_other.projected_point(we);

        (projection, collision_dist)
    }
}
