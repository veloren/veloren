use common::{
    comp::{
        Body, CharacterState, Collider, Density, Immovable, Mass, Ori, PhysicsState, Pos,
        PosVelOriDefer, PreviousPhysCache, Projectile, Scale, Stats, Sticky, Vel,
        body::ship::figuredata::VOXEL_COLLIDER_MANIFEST,
        fluid_dynamics::{Fluid, Wings},
        inventory::item::armor::Friction,
    },
    consts::{AIR_DENSITY, GRAVITY},
    event::{EmitExt, EventBus, LandOnGroundEvent},
    event_emitters,
    link::Is,
    mounting::{Rider, VolumeRider},
    outcome::Outcome,
    resources::{DeltaTime, GameMode, TimeOfDay},
    states,
    terrain::{CoordinateConversions, TerrainGrid},
    uid::Uid,
    util::{Projection, SpatialGrid},
    weather::WeatherGrid,
};
use common_base::{prof_span, span};
use common_ecs::{Job, Origin, ParMode, Phase, PhysicsMetrics, System};
use rayon::iter::ParallelIterator;
use specs::{
    Entities, Join, LendJoin, ParJoin, Read, ReadExpect, ReadStorage, SystemData, Write,
    WriteExpect, WriteStorage, shred,
};
use vek::*;

mod collision;
mod weather;
use collision::*;

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

fn integrate_forces(
    dt: &DeltaTime,
    mut vel: Vel,
    (body, wings): (&Body, Option<&Wings>),
    density: &Density,
    mass: &Mass,
    fluid: &Fluid,
    gravity: f32,
    scale: Option<Scale>,
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
        // HACK: We should really use the latter logic (i.e: `aerodynamic_forces`) for
        // liquids, but it results in pretty serious dt-dependent problems that
        // are extremely difficult to resolve. This is a compromise: for liquids
        // only, we calculate drag using an incorrect (but still visually plausible)
        // model that is much more resistant to differences in dt. Because it's
        // physically incorrect anyway, there are magic coefficients that
        // exist simply to get us closer to what water 'should' feel like.
        if fluid.is_liquid() {
            let fric = body
                .drag_coefficient_liquid(fluid_density.0, scale.map_or(1.0, |s| s.0))
                .powf(0.75)
                * 0.02;

            let fvel = fluid.flow_vel();

            // Drag is relative to fluid velocity, so compensate before applying drag
            vel.0 = (vel.0 - fvel.0) * (1.0 / (1.0 + fric)).powf(dt.0 * 10.0) + fvel.0;
        } else {
            let impulse = dt.0
                * body.aerodynamic_forces(
                    &rel_flow,
                    fluid_density.0,
                    wings,
                    scale.map_or(1.0, |s| s.0),
                );
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
            }
        }
        debug_assert!(!vel.0.map(|a| a.is_nan()).reduce_or());
    };

    // Hydrostatic/aerostatic forces
    // modify gravity to account for the effective density as a result of buoyancy
    let down_force = dt.0 * gravity * (density.0 - fluid_density.0) / density.0;
    vel.0.z -= down_force;

    vel
}

fn calc_z_limit(char_state_maybe: Option<&CharacterState>, collider: &Collider) -> (f32, f32) {
    let modifier = if char_state_maybe.is_some_and(|c_s| c_s.is_dodge() || c_s.is_glide()) {
        0.5
    } else {
        1.0
    };
    collider.get_z_limits(modifier)
}

event_emitters! {
    struct Events[Emitters] {
        land_on_ground: LandOnGroundEvent,
    }
}

/// This system applies forces and calculates new positions and velocities.
#[derive(Default)]
pub struct Sys;

#[derive(SystemData)]
pub struct PhysicsRead<'a> {
    entities: Entities<'a>,
    events: Events<'a>,
    uids: ReadStorage<'a, Uid>,
    terrain: ReadExpect<'a, TerrainGrid>,
    dt: Read<'a, DeltaTime>,
    game_mode: ReadExpect<'a, GameMode>,
    scales: ReadStorage<'a, Scale>,
    stickies: ReadStorage<'a, Sticky>,
    immovables: ReadStorage<'a, Immovable>,
    masses: ReadStorage<'a, Mass>,
    colliders: ReadStorage<'a, Collider>,
    is_riders: ReadStorage<'a, Is<Rider>>,
    is_volume_riders: ReadStorage<'a, Is<VolumeRider>>,
    projectiles: ReadStorage<'a, Projectile>,
    character_states: ReadStorage<'a, CharacterState>,
    bodies: ReadStorage<'a, Body>,
    densities: ReadStorage<'a, Density>,
    stats: ReadStorage<'a, Stats>,
    weather: Option<Read<'a, WeatherGrid>>,
    time_of_day: Read<'a, TimeOfDay>,
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
    outcomes: Read<'a, EventBus<Outcome>>,
}

#[derive(SystemData)]
pub struct PhysicsData<'a> {
    read: PhysicsRead<'a>,
    write: PhysicsWrite<'a>,
}

impl PhysicsData<'_> {
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
        )
            .join()
            .map(|(e, _, _, _, _)| e)
            .collect::<Vec<_>>()
        {
            let _ = self
                .write
                .previous_phys_cache
                .insert(entity, PreviousPhysCache {
                    velocity: Vec3::zero(),
                    velocity_dt: Vec3::zero(),
                    in_fluid: None,
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
        for (_, vel, position, ori, phys_state, phys_cache, collider, scale, cs) in (
            &self.read.entities,
            &self.write.velocities,
            &self.write.positions,
            &self.write.orientations,
            &self.write.physics_states,
            &mut self.write.previous_phys_cache,
            &self.read.colliders,
            self.read.scales.maybe(),
            self.read.character_states.maybe(),
        )
            .join()
        {
            let scale = scale.map(|s| s.0).unwrap_or(1.0);
            let z_limits = calc_z_limit(cs, collider);
            let (z_min, z_max) = z_limits;
            let (z_min, z_max) = (z_min * scale, z_max * scale);
            let half_height = (z_max - z_min) / 2.0;

            phys_cache.velocity_dt = vel.0 * self.read.dt.0;
            phys_cache.velocity = vel.0;
            phys_cache.in_fluid = phys_state.in_fluid;
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
                    if len < f32::EPSILON * 10.0 {
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
        for (entity, pos, phys_cache, _, _) in (
            &read.entities,
            &write.positions,
            &write.previous_phys_cache,
            write.velocities.mask(),
            !&read.projectiles, // Not needed because they are skipped in the inner loop below
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
            read.is_riders.maybe(),
            read.is_volume_riders.maybe(),
            read.stickies.maybe(),
            read.immovables.maybe(),
            &mut write.physics_states,
            // TODO: if we need to avoid collisions for other things consider
            // moving whether it should interact into the collider component
            // or into a separate component.
            read.projectiles.maybe(),
            read.character_states.maybe(),
        )
            .par_join()
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
                    is_rider,
                    is_volume_rider,
                    sticky,
                    immovable,
                    physics,
                    projectile,
                    char_state_maybe,
                )| {
                    let is_sticky = sticky.is_some();
                    let is_immovable = immovable.is_some();
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
                                read.character_states.get(entity),
                                read.is_riders.get(entity),
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
                                other_is_rider_maybe,
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
                                        is_immovable,
                                        is_projectile,
                                        // entity we colliding with
                                        *other,
                                        // symetrical collider context
                                        ColliderData {
                                            pos,
                                            previous_cache,
                                            z_limits,
                                            collider,
                                            mass: *mass,
                                        },
                                        ColliderData {
                                            pos: pos_other,
                                            previous_cache: previous_cache_other,
                                            z_limits: z_limits_other,
                                            collider: collider_other,
                                            mass: *mass_other,
                                        },
                                        vel,
                                        is_rider.is_some()
                                            || is_volume_rider.is_some()
                                            || other_is_rider_maybe.is_some(),
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
        for (entity, pos, collider, scale, ori) in (
            &read.entities,
            &write.positions,
            &read.colliders,
            read.scales.maybe(),
            &write.orientations,
        )
            .join()
        {
            let vol = collider.get_vol(&voxel_colliders_manifest);

            if let Some(vol) = vol {
                let sphere = voxel_collider_bounding_sphere(vol, pos, ori, scale);
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

        prof_span!(guard, "Apply Weather");
        if let Some(weather) = &read.weather {
            for (_, state, pos, phys) in (
                &read.entities,
                &read.character_states,
                &write.positions,
                &mut write.physics_states,
            )
                .join()
            {
                // Always reset air_vel to zero
                let mut air_vel = Vec3::zero();

                'simulation: {
                    // Don't simulate for non-gliding, for now
                    if !state.is_glide() {
                        break 'simulation;
                    }

                    let pos_2d = pos.0.as_().xy();
                    let chunk_pos: Vec2<i32> = pos_2d.wpos_to_cpos();
                    let Some(current_chunk) = &read.terrain.get_key(chunk_pos) else {
                        // oopsie
                        break 'simulation;
                    };

                    let meta = current_chunk.meta();

                    // Skip simulating for entites deeply under the ground
                    if pos.0.z < meta.alt() - 25.0 {
                        break 'simulation;
                    }

                    // If couldn't simulate wind for some reason, skip
                    if let Ok(simulated_vel) =
                        weather::simulated_wind_vel(pos, weather, &read.terrain, &read.time_of_day)
                    {
                        air_vel = simulated_vel
                    };
                }

                phys.in_fluid = phys.in_fluid.map(|f| match f {
                    Fluid::Air { elevation, .. } => Fluid::Air {
                        vel: Vel(air_vel),
                        elevation,
                    },
                    fluid => fluid,
                });
            }
        }

        drop(guard);

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
            !&read.is_riders,
            !&read.is_volume_riders,
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
            read.scales.maybe(),
            !&read.is_riders,
            !&read.is_volume_riders,
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
                    scale,
                    _,
                    _,
                )| {
                    let in_loaded_chunk = read
                        .terrain
                        .contains_key(read.terrain.pos_key(pos.0.map(|e| e.floor() as i32)));

                    // Apply physics only if in a loaded chunk
                    if in_loaded_chunk
                    // And not already stuck on a block (e.g., for arrows)
                    && !(physics_state.on_surface().is_some() && sticky.is_some())
                    // HACK: Special-case boats. Experimentally, clients are *bad* at making guesses about movement,
                    // and this is a particular problem for volume entities since careful control of velocity is
                    // required for nice movement of entities on top of the volume. Special-case volume entities here
                    // to prevent weird drag/gravity guesses messing with movement, relying on the client's hermite
                    // interpolation instead.
                    && !(matches!(body, Body::Ship(_)) && matches!(&*read.game_mode, GameMode::Client))
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
                                    scale.copied(),
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
        for (pos, ori, previous_phys_cache) in (
            &write.positions,
            &write.orientations,
            &mut write.previous_phys_cache,
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
        let (land_on_grounds, outcomes) = (
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
            !&read.is_riders,
            !&read.is_volume_riders,
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
                    physics_state,
                    pos_vel_ori_defer,
                    previous_cache,
                    _,
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

                    if let Some(state) = character_state {
                        let footwear = state.footwear().unwrap_or(Friction::Normal);
                        if footwear != physics_state.footwear {
                            physics_state.footwear = footwear;
                        }
                    }

                    let in_loaded_chunk = read
                        .terrain
                        .contains_key(read.terrain.pos_key(pos.0.map(|e| e.floor() as i32)));

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
                        body.is_some_and(|b| !matches!(b, Body::Object(_) | Body::Ship(_)));
                    let climbing =
                        character_state.is_some_and(|cs| matches!(cs, CharacterState::Climb(_)));

                    let friction_factor = |vel: Vec3<f32>| {
                        if let Some(Body::Ship(ship)) = body
                            && ship.has_wheels()
                        {
                            vel.try_normalized()
                                .and_then(|dir| {
                                    Some(orientations.get(entity)?.right().dot(dir).abs())
                                })
                                .unwrap_or(1.0)
                                .max(0.2)
                        } else {
                            1.0
                        }
                    };

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
                                &read.dt,
                                was_on_ground,
                                block_snap,
                                climbing,
                                |entity, vel, surface_normal| {
                                    land_on_ground = Some((entity, vel, surface_normal))
                                },
                                read,
                                &ori,
                                friction_factor,
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
                                &read.dt,
                                was_on_ground,
                                block_snap,
                                climbing,
                                |entity, vel, surface_normal| {
                                    land_on_ground = Some((entity, vel, surface_normal))
                                },
                                read,
                                &ori,
                                friction_factor,
                            );

                            // Sticky things shouldn't move when on a surface
                            if physics_state.on_surface().is_some() && sticky.is_some() {
                                vel.0 = physics_state.ground_vel;
                            }

                            tgt_pos = cpos.0;
                        },
                        Collider::Point => {
                            let mut pos = *pos;

                            point_voxel_collision(
                                entity,
                                &mut pos,
                                pos_delta,
                                &mut vel,
                                physics_state,
                                sticky.is_some(),
                                &mut outcomes,
                                read,
                            );

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
                            positions.get(entity).and_then(|pos| {
                                Some((
                                    entity,
                                    pos,
                                    velocities.get(entity)?,
                                    previous_phys_cache.get(entity)?,
                                    read.colliders.get(entity)?,
                                    read.scales.get(entity),
                                    orientations.get(entity)?,
                                ))
                            })
                        })
                        .for_each(
                            |(
                                entity_other,
                                pos_other,
                                vel_other,
                                previous_cache_other,
                                collider_other,
                                scale_other,
                                ori_other,
                            )| {
                                if entity == entity_other {
                                    return;
                                }

                                let voxel_collider =
                                    collider_other.get_vol(&voxel_colliders_manifest);

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
                                        scale_other,
                                    );
                                    // Early check
                                    if voxel_sphere.center.distance_squared(path_sphere.center)
                                        > (voxel_sphere.radius + path_sphere.radius).powi(2)
                                    {
                                        return;
                                    }

                                    let mut physics_state_delta = physics_state.clone();

                                    // Helper function for computing a transformation matrix and its
                                    // inverse. Should
                                    // be much cheaper than using `Mat4::inverted`.
                                    let from_to_matricies =
                                        |entity_rpos: Vec3<f32>, collider_ori: Quaternion<f32>| {
                                            (
                                                Mat4::<f32>::translation_3d(entity_rpos)
                                                    * Mat4::from(collider_ori)
                                                    * Mat4::scaling_3d(previous_cache_other.scale)
                                                    * Mat4::translation_3d(
                                                        voxel_collider.translation,
                                                    ),
                                                Mat4::<f32>::translation_3d(
                                                    -voxel_collider.translation,
                                                ) * Mat4::scaling_3d(
                                                    1.0 / previous_cache_other.scale,
                                                ) * Mat4::from(collider_ori.inverse())
                                                    * Mat4::translation_3d(-entity_rpos),
                                            )
                                        };

                                    // Compute matrices that allow us to transform to and from the
                                    // coordinate space of
                                    // the collider. We have two variants of each, one for the
                                    // current state and one for
                                    // the previous state. This allows us to 'perfectly' track
                                    // change in position
                                    // between ticks, which prevents entities falling through voxel
                                    // colliders due to spurious
                                    // issues like differences in ping/variable dt.
                                    // TODO: Cache the matrices here to avoid recomputing for each
                                    // entity on them
                                    let (_transform_last_from, transform_last_to) =
                                        from_to_matricies(
                                            previous_cache_other.pos.unwrap_or(*pos_other).0
                                                - previous_cache.pos.unwrap_or(*pos).0,
                                            previous_cache_other.ori,
                                        );
                                    let (transform_from, transform_to) =
                                        from_to_matricies(pos_other.0 - pos.0, ori_other.to_quat());

                                    // Compute the velocity of the collider, accounting for changes
                                    // in orientation
                                    // from the last tick. We then model this change as a change in
                                    // surface velocity
                                    // for the collider.
                                    let vel_other = {
                                        let pos_rel =
                                            (Mat4::<f32>::translation_3d(
                                                -voxel_collider.translation,
                                            ) * Mat4::from(ori_other.to_quat().inverse()))
                                            .mul_point(pos.0 - pos_other.0);
                                        let rpos_last =
                                            (Mat4::<f32>::from(previous_cache_other.ori)
                                                * Mat4::translation_3d(voxel_collider.translation))
                                            .mul_point(pos_rel);
                                        vel_other.0
                                            + (pos.0 - (pos_other.0 + rpos_last)) / read.dt.0
                                    };

                                    {
                                        // Transform the entity attributes into the coordinate space
                                        // of the collider ready
                                        // for collision resolution
                                        let mut rpos =
                                            Pos(transform_last_to.mul_point(Vec3::zero()));
                                        vel.0 = previous_cache_other.ori.inverse()
                                            * (vel.0 - vel_other);

                                        // Perform collision resolution
                                        box_voxel_collision(
                                            (radius, z_min, z_max),
                                            &voxel_collider.volume(),
                                            entity,
                                            &mut rpos,
                                            transform_to.mul_point(tgt_pos - pos.0),
                                            &mut vel,
                                            &mut physics_state_delta,
                                            &read.dt,
                                            was_on_ground,
                                            block_snap,
                                            climbing,
                                            |entity, vel, surface_normal| {
                                                land_on_ground = Some((
                                                    entity,
                                                    Vel(previous_cache_other.ori * vel.0
                                                        + vel_other),
                                                    previous_cache_other.ori * surface_normal,
                                                ));
                                            },
                                            read,
                                            &ori,
                                            |vel| friction_factor(previous_cache_other.ori * vel),
                                        );

                                        // Transform entity attributes back into world space now
                                        // that we've performed
                                        // collision resolution with them
                                        tgt_pos = transform_from.mul_point(rpos.0) + pos.0;
                                        vel.0 = previous_cache_other.ori * vel.0 + vel_other;
                                    }

                                    // Collision resolution may also change the physics state. Since
                                    // we may be interacting
                                    // with multiple colliders at once (along with the regular
                                    // terrain!) we keep track
                                    // of a physics state 'delta' and try to sensibly resolve them
                                    // against one-another at each step.
                                    if physics_state_delta.on_ground.is_some() {
                                        // TODO: Do we need to do this? Perhaps just take the
                                        // ground_vel regardless?
                                        physics_state.ground_vel = previous_cache_other.ori
                                            * physics_state_delta.ground_vel
                                            + vel_other;
                                    }
                                    if physics_state_delta.on_surface().is_some() {
                                        // If the collision resulted in us being on a surface,
                                        // rotate us with the
                                        // collider. Really this should be modelled via friction or
                                        // something, but
                                        // our physics model doesn't really take orientation into
                                        // consideration.
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
                                            .map(|dir| previous_cache_other.ori * dir)
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

        write.outcomes.emitter().emit_many(outcomes);

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

        let mut emitters = read.events.get_emitters();
        emitters.emit_many(
            land_on_grounds
                .into_iter()
                .map(|(entity, vel, surface_normal)| LandOnGroundEvent {
                    entity,
                    vel: vel.0,
                    surface_normal,
                }),
        );
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
