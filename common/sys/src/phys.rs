use common::{
    comp::{
        body::ship::figuredata::VOXEL_COLLIDER_MANIFEST, BeamSegment, CharacterState, Collider,
        Gravity, Mass, Mounting, Ori, PhysicsState, Pos, PreviousPhysCache, Projectile, Scale,
        Shockwave, Sticky, Vel,
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
pub const FRIC_AIR: f32 = 0.0125;
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
pub struct PhysicsSystemDataRead<'a> {
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
}

#[derive(SystemData)]
pub struct PhysicsSystemDataWrite<'a> {
    physics_metrics: WriteExpect<'a, PhysicsMetrics>,
    physics_states: WriteStorage<'a, PhysicsState>,
    positions: WriteStorage<'a, Pos>,
    velocities: WriteStorage<'a, Vel>,
    orientations: WriteStorage<'a, Ori>,
    previous_phys_cache: WriteStorage<'a, PreviousPhysCache>,
}

#[derive(SystemData)]
pub struct PhysicsSystemData<'a> {
    r: PhysicsSystemDataRead<'a>,
    w: PhysicsSystemDataWrite<'a>,
}

impl<'a> PhysicsSystemData<'a> {
    /// Add/reset physics state components
    fn reset(&mut self) {
        span!(guard, "Add/reset physics state components");
        for (entity, _, _, _, _) in (
            &self.r.entities,
            &self.r.colliders,
            &self.w.positions,
            &self.w.velocities,
            &self.w.orientations,
        )
            .join()
        {
            let _ = self
                .w
                .physics_states
                .entry(entity)
                .map(|e| e.or_insert_with(Default::default));
        }
        drop(guard);
    }

    fn maintain_pushback_cache(&mut self) {
        span!(guard, "Maintain pushback cache");
        //Add PreviousPhysCache for all relevant entities
        for entity in (
            &self.r.entities,
            &self.w.velocities,
            &self.w.positions,
            !&self.w.previous_phys_cache,
            !&self.r.mountings,
            !&self.r.beams,
            !&self.r.shockwaves,
        )
            .join()
            .map(|(e, _, _, _, _, _, _)| e)
            .collect::<Vec<_>>()
        {
            let _ = self
                .w
                .previous_phys_cache
                .insert(entity, PreviousPhysCache {
                    velocity_dt: Vec3::zero(),
                    center: Vec3::zero(),
                    collision_boundary: 0.0,
                    scale: 0.0,
                    scaled_radius: 0.0,
                });
        }

        //Update PreviousPhysCache
        for (_, vel, position, mut phys_cache, collider, scale, cs, _, _, _) in (
            &self.r.entities,
            &self.w.velocities,
            &self.w.positions,
            &mut self.w.previous_phys_cache,
            self.r.colliders.maybe(),
            self.r.scales.maybe(),
            self.r.char_states.maybe(),
            !&self.r.mountings,
            !&self.r.beams,
            !&self.r.shockwaves,
        )
            .join()
        {
            let scale = scale.map(|s| s.0).unwrap_or(1.0);
            let z_limits = calc_z_limit(cs, collider);
            let z_limits = (z_limits.0 * scale, z_limits.1 * scale);
            let half_height = (z_limits.1 - z_limits.0) / 2.0;

            phys_cache.velocity_dt = vel.0 * self.r.dt.0;
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
        drop(guard);
    }

    fn apply_pushback(&mut self, job: &mut Job<Sys>) {
        span!(guard, "Apply pushback");
        job.cpu_stats.measure(ParMode::Rayon);
        let PhysicsSystemData {
            r: ref psdr,
            w: ref mut psdw,
        } = self;
        let (positions, previous_phys_cache) = (&psdw.positions, &psdw.previous_phys_cache);
        let metrics = (
            &psdr.entities,
            positions,
            &mut psdw.velocities,
            previous_phys_cache,
            psdr.masses.maybe(),
            psdr.colliders.maybe(),
            !&psdr.mountings,
            psdr.stickies.maybe(),
            &mut psdw.physics_states,
            // TODO: if we need to avoid collisions for other things consider moving whether it
            // should interact into the collider component or into a separate component
            psdr.projectiles.maybe(),
            psdr.char_states.maybe(),
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
                        &psdr.entities,
                        &psdr.uids,
                        positions,
                        previous_phys_cache,
                        psdr.masses.maybe(),
                        psdr.colliders.maybe(),
                        !&psdr.projectiles,
                        !&psdr.mountings,
                        !&psdr.beams,
                        !&psdr.shockwaves,
                        psdr.char_states.maybe(),
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
                        let z_limits_other = calc_z_limit(char_state_other_maybe, collider_other);

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
                            let pos_other = pos_other.0 + previous_cache_other.velocity_dt * factor;

                            let diff = pos.xy() - pos_other.xy();

                            if diff.magnitude_squared() <= collision_dist.powi(2)
                                && pos.z + z_limits.1 * previous_cache.scale
                                    >= pos_other.z + z_limits_other.0 * previous_cache_other.scale
                                && pos.z + z_limits.0 * previous_cache.scale
                                    <= pos_other.z + z_limits_other.1 * previous_cache_other.scale
                            {
                                if !collided {
                                    physics.touch_entities.push(*other);
                                    entity_entity_collisions += 1;
                                }

                                // Don't apply repulsive force to projectiles or if we're colliding
                                // with a terrain-like entity, or if we are a terrain-like entity
                                if diff.magnitude_squared() > 0.0
                                    && !is_projectile
                                    && !matches!(collider_other, Some(Collider::Voxel { .. }))
                                    && !matches!(collider, Some(Collider::Voxel { .. }))
                                {
                                    let force =
                                        400.0 * (collision_dist - diff.magnitude()) * mass_other
                                            / (mass + mass_other);

                                    vel_delta += Vec3::from(diff.normalized()) * force * step_delta;
                                }

                                collided = true;
                            }
                        }
                    }

                    // Change velocity
                    vel.0 += vel_delta * psdr.dt.0;

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
        psdw.physics_metrics.entity_entity_collision_checks =
            metrics.entity_entity_collision_checks;
        psdw.physics_metrics.entity_entity_collisions = metrics.entity_entity_collisions;
        drop(guard);
    }

    fn handle_movement_and_terrain(&mut self, job: &mut Job<Sys>) {
        let PhysicsSystemData {
            r: ref psdr,
            w: ref mut psdw,
        } = self;
        // Apply movement inputs
        span!(guard, "Apply movement and terrain collision");
        let (positions, velocities, previous_phys_cache, orientations) = (&psdw.positions, &psdw.velocities, &psdw.previous_phys_cache, &psdw.orientations);
        let (pos_writes, vel_writes, land_on_grounds) = (
            &psdr.entities,
            psdr.scales.maybe(),
            psdr.stickies.maybe(),
            &psdr.colliders,
            positions,
            velocities,
            orientations,
            &mut psdw.physics_states,
            previous_phys_cache,
            !&psdr.mountings,
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
                    mut physics_state,
                    previous_cache,
                    _,
                )| {
                    // defer the writes of positions to allow an inner loop over terrain-like
                    // entities
                    let old_pos = *pos;
                    let mut pos = *pos;
                    let mut vel = *vel;
                    if sticky.is_some() && physics_state.on_surface().is_some() {
                        vel.0 = Vec3::zero();
                        return (pos_writes, vel_writes, land_on_grounds);
                    }

                    let scale = if let Collider::Voxel { .. } = collider {
                        scale.map(|s| s.0).unwrap_or(1.0)
                    } else {
                        // TODO: Use scale & actual proportions when pathfinding is good
                        // enough to manage irregular entity sizes
                        1.0
                    };

                    let old_vel = vel;
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
                    let in_loaded_chunk = psdr
                        .terrain
                        .get_key(psdr.terrain.pos_key(pos.0.map(|e| e.floor() as i32)))
                        .is_some();
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
                        } * psdr.gravities.get(entity).map(|g| g.0).unwrap_or_default();
                    vel.0 = integrate_forces(psdr.dt.0, vel.0, downward_force, friction);

                    // Don't move if we're not in a loaded chunk
                    let pos_delta = if in_loaded_chunk {
                        // this is an approximation that allows most framerates to
                        // behave in a similar manner.
                        let dt_lerp = 0.2;
                        (vel.0 * dt_lerp + old_vel.0 * (1.0 - dt_lerp)) * psdr.dt.0
                    } else {
                        Vec3::zero()
                    };

                    let was_on_ground = physics_state.on_ground;

                    match &*collider {
                        Collider::Voxel { .. } => {
                            // for now, treat entities with voxel colliders as their bounding
                            // cylinders for the purposes of colliding them with terrain
                            // Actually no, make them smaller to avoid lag
                            let radius = collider.get_radius() * scale * 0.1;
                            let (z_min, z_max) = collider.get_z_limits(scale);

                            let cylinder = (radius, z_min, z_max);
                            cylinder_voxel_collision(
                                cylinder,
                                &*psdr.terrain,
                                entity,
                                &mut pos,
                                pos_delta,
                                &mut vel,
                                &mut physics_state,
                                Vec3::zero(),
                                &psdr.dt,
                                true,
                                was_on_ground,
                                |entity, vel| land_on_grounds.push((entity, vel)),
                            );
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
                            cylinder_voxel_collision(
                                cylinder,
                                &*psdr.terrain,
                                entity,
                                &mut pos,
                                pos_delta,
                                &mut vel,
                                &mut physics_state,
                                Vec3::zero(),
                                &psdr.dt,
                                true,
                                was_on_ground,
                                |entity, vel| land_on_grounds.push((entity, vel)),
                            );
                        },
                        Collider::Point => {
                            let (dist, block) = psdr
                                .terrain
                                .ray(pos.0, pos.0 + pos_delta)
                                .until(|block: &Block| block.is_filled())
                                .ignore_error()
                                .cast();

                            pos.0 += pos_delta.try_normalized().unwrap_or(Vec3::zero()) * dist;

                            // Can't fail since we do ignore_error above
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

                            physics_state.in_liquid = psdr
                                .terrain
                                .get(pos.0.map(|e| e.floor() as i32))
                                .ok()
                                .and_then(|vox| vox.is_liquid().then_some(1.0));
                        },
                    }

                    // Collide with terrain-like entities
                    for (
                        entity_other,
                        other,
                        pos_other,
                        vel_other,
                        previous_cache_other,
                        mass_other,
                        collider_other,
                        ori_other,
                        _,
                        _,
                        _,
                        _,
                        char_state_other_maybe,
                    ) in (
                        &psdr.entities,
                        &psdr.uids,
                        positions,
                        velocities,
                        previous_phys_cache,
                        psdr.masses.maybe(),
                        &psdr.colliders,
                        orientations,
                        !&psdr.projectiles,
                        !&psdr.mountings,
                        !&psdr.beams,
                        !&psdr.shockwaves,
                        psdr.char_states.maybe(),
                    )
                        .join()
                    {
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

                        if let Collider::Voxel { id } = collider_other {
                            // use bounding cylinder regardless of our collider
                            // TODO: extract point-terrain collision above to its own function
                            let radius = collider.get_radius();
                            let (z_min, z_max) = collider.get_z_limits(1.0);

                            let radius = radius.min(0.45) * scale;
                            let z_min = z_min * scale;
                            let z_max = z_max.clamped(1.2, 1.95) * scale;

                            if let Some(voxel_collider) = VOXEL_COLLIDER_MANIFEST.read().colliders.get(id) {
                                let mut physics_state_delta = physics_state.clone();
                                // deliberately don't use scale yet here, because the 11.0/0.8
                                // thing is in the comp::Scale for visual reasons
                                let transform_from = Mat4::<f32>::translation_3d(pos_other.0)
                                    * Mat4::from(ori_other.0)
                                    * Mat4::<f32>::translation_3d(voxel_collider.translation);
                                let transform_to = transform_from.inverted();
                                pos.0 = transform_to.mul_point(pos.0);
                                vel.0 = transform_to.mul_direction(vel.0);
                                let cylinder = (radius, z_min, z_max);
                                cylinder_voxel_collision(
                                    cylinder,
                                    &voxel_collider.dyna,
                                    entity,
                                    &mut pos,
                                    transform_to.mul_direction(pos_delta),
                                    &mut vel,
                                    &mut physics_state_delta,
                                    transform_to.mul_direction(vel_other.0),
                                    &psdr.dt,
                                    false,
                                    was_on_ground,
                                    |entity, vel| land_on_grounds.push((entity, Vel(transform_from.mul_direction(vel.0)))),
                                );

                                pos.0 = transform_from.mul_point(pos.0);
                                vel.0 = transform_from.mul_direction(vel.0);

                                // union in the state updates, so that the state isn't just based on
                                // the most recent terrain that collision was attempted with
                                if physics_state_delta.on_ground {
                                    physics_state.ground_vel = vel_other.0;
                                }
                                physics_state.on_ground |= physics_state_delta.on_ground;
                                physics_state.on_ceiling |= physics_state_delta.on_ceiling;
                                physics_state.on_wall =
                                    physics_state.on_wall.or(physics_state_delta.on_wall);
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
                    }
                    if pos != old_pos {
                        pos_writes.push((entity, pos));
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
        for (entity, pos, vel) in (&psdr.entities, &mut psdw.positions, &mut psdw.velocities).join() {
            if let Some(new_pos) = pos_writes.get(&entity) {
                *pos = *new_pos;
            }

            if let Some(new_vel) = vel_writes.get(&entity) {
                *vel = *new_vel;
            }
        }

        let mut event_emitter = psdr.event_bus.emitter();
        land_on_grounds.into_iter().for_each(|(entity, vel)| {
            event_emitter.emit(ServerEvent::LandOnGround { entity, vel: vel.0 });
        });
    }
}

impl<'a> System<'a> for Sys {
    type SystemData = PhysicsSystemData<'a>;

    const NAME: &'static str = "phys";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    #[allow(clippy::blocks_in_if_conditions)] // TODO: Pending review in #587
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
        psd.apply_pushback(job);

        psd.handle_movement_and_terrain(job);
    }
}

fn cylinder_voxel_collision<'a, T: BaseVol<Vox = Block> + ReadVol>(
    cylinder: (f32, f32, f32),
    terrain: &'a T,
    entity: Entity,
    pos: &mut Pos,
    mut pos_delta: Vec3<f32>,
    vel: &mut Vel,
    physics_state: &mut PhysicsState,
    ground_vel: Vec3<f32>,
    dt: &DeltaTime,
    apply_velocity_step: bool, // Stupid hack
    was_on_ground: bool,
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

    // Don't jump too far at once
    let increments = (pos_delta.map(|e| e.abs()).reduce_partial_max() / 0.3)
        .ceil()
        .max(1.0);
    let old_pos = pos.0;
    fn block_true(_: &Block) -> bool { true }
    for _ in 0..increments as usize {
        if apply_velocity_step {
            pos.0 += pos_delta / increments;
        }

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
            if resolve_dir.z > 0.0 && vel.0.z <= 0.0 {
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
                        if d * e.signum() < 0.0 { if d < 0.0 { d.max(0.0) } else { d.min(0.0) } } else { e }
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

        vel.0 = ground_vel + (vel.0 - ground_vel) * (1.0 - FRIC_GROUND.min(1.0)).powf(dt.0 * 60.0);
        physics_state.ground_vel = ground_vel;
    // If the space below us is free, then "snap" to the ground
    } else if collision_with(
        pos.0 - Vec3::unit_z() * 1.05,
        &terrain,
        block_true,
        near_iter.clone(),
        radius,
        z_range.clone(),
    ) && vel.0.z < 0.01
        && vel.0.z > -1.5
        && was_on_ground
        && !collision_with(
            pos.0 - Vec3::unit_z() * 0.05,
            &terrain,
            |block| block.solid_height() >= (pos.0.z - 0.05).rem_euclid(1.0),
            near_iter.clone(),
            radius,
            z_range.clone(),
        )
    {
        let snap_height = terrain
            .get(Vec3::new(pos.0.x, pos.0.y, pos.0.z - 0.05).map(|e| e.floor() as i32))
            .ok()
            .filter(|block| block.is_solid())
            .map(|block| block.solid_height())
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

    // Figure out if we're in water
    physics_state.in_liquid = collision_iter(
        pos.0,
        &*terrain,
        &|block| block.is_liquid(),
        // The liquid part of a liquid block always extends 1 block high.
        &|_block| 1.0,
        near_iter.clone(),
        radius,
        z_min..z_max,
    )
    .max_by_key(|block_aabb| (block_aabb.max.z * 100.0) as i32)
    .map(|block_aabb| block_aabb.max.z - pos.0.z);
}
