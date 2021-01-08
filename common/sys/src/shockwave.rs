use common::{
    comp::{
        group, Body, Health, HealthSource, Inventory, Last, Ori, PhysicsState, Pos, Scale,
        Shockwave, ShockwaveHitEntities,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    resources::{DeltaTime, Time},
    uid::{Uid, UidAllocator},
    util::Dir,
    GroupTarget,
};
use specs::{saveload::MarkerAllocator, Entities, Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

/// This system is responsible for handling accepted inputs like moving or
/// attacking
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, EventBus<LocalEvent>>,
        Read<'a, Time>,
        Read<'a, DeltaTime>,
        Read<'a, UidAllocator>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Last<Pos>>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, Scale>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Health>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, group::Group>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, Shockwave>,
        WriteStorage<'a, ShockwaveHitEntities>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_bus,
            local_bus,
            time,
            dt,
            uid_allocator,
            uids,
            positions,
            last_positions,
            orientations,
            scales,
            bodies,
            healths,
            inventories,
            groups,
            physics_states,
            mut shockwaves,
            mut shockwave_hit_lists,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();
        let _local_emitter = local_bus.emitter();

        let time = time.0;
        let dt = dt.0;

        // Shockwaves
        for (entity, uid, pos, ori, shockwave, shockwave_hit_list) in (
            &entities,
            &uids,
            &positions,
            &orientations,
            &shockwaves,
            &mut shockwave_hit_lists,
        )
            .join()
        {
            let creation_time = match shockwave.creation {
                Some(time) => time,
                // Skip newly created shockwaves
                None => continue,
            };

            let end_time = creation_time + shockwave.duration.as_secs_f64();

            // If shockwave is out of time emit destroy event but still continue since it
            // may have traveled and produced effects a bit before reaching it's
            // end point
            if time > end_time {
                server_emitter.emit(ServerEvent::Destroy {
                    entity,
                    cause: HealthSource::World,
                });
                continue;
            }

            // Determine area that was covered by the shockwave in the last tick
            let time_since_creation = (time - creation_time) as f32;
            let frame_start_dist = (shockwave.speed * (time_since_creation - dt)).max(0.0);
            let frame_end_dist = (shockwave.speed * time_since_creation).max(frame_start_dist);
            let pos2 = Vec2::from(pos.0);

            // From one frame to the next a shockwave travels over a strip of an arc
            // This is used for collision detection
            let arc_strip = ArcStrip {
                origin: pos2,
                // TODO: make sure this is not Vec2::new(0.0, 0.0)
                dir: ori.0.xy(),
                angle: shockwave.angle,
                start: frame_start_dist,
                end: frame_end_dist,
            };

            // Group to ignore collisions with
            // Might make this more nuanced if shockwaves are used for non damage effects
            let group = shockwave
                .owner
                .and_then(|uid| uid_allocator.retrieve_entity_internal(uid.into()))
                .and_then(|e| groups.get(e));

            // Go through all other effectable entities
            for (
                b,
                uid_b,
                pos_b,
                last_pos_b_maybe,
                scale_b_maybe,
                health_b,
                body_b,
                physics_state_b,
            ) in (
                &entities,
                &uids,
                &positions,
                // TODO: make sure that these are maintained on the client and remove `.maybe()`
                last_positions.maybe(),
                scales.maybe(),
                &healths,
                &bodies,
                &physics_states,
            )
                .join()
            {
                // Check to see if entity has already been hit
                if shockwave_hit_list
                    .hit_entities
                    .iter()
                    .any(|&uid| uid == *uid_b)
                {
                    continue;
                }

                // 2D versions
                let pos_b2 = pos_b.0.xy();
                let last_pos_b2_maybe = last_pos_b_maybe.map(|p| (p.0).0.xy());

                // Scales
                let scale_b = scale_b_maybe.map_or(1.0, |s| s.0);
                let rad_b = body_b.radius() * scale_b;

                // Angle checks
                let pos_b_ground = Vec3::new(pos_b.0.x, pos_b.0.y, pos.0.z);
                let max_angle = shockwave.vertical_angle.to_radians();

                // See if entities are in the same group
                let same_group = group
                    .map(|group_a| Some(group_a) == groups.get(b))
                    .unwrap_or(Some(*uid_b) == shockwave.owner);

                let target_group = if same_group {
                    GroupTarget::InGroup
                } else {
                    GroupTarget::OutOfGroup
                };

                // Check if it is a hit
                let hit = entity != b
                    && !health_b.is_dead
                    // Collision shapes
                    && {
                        // TODO: write code to collide rect with the arc strip so that we can do
                        // more complete collision detection for rapidly moving entities
                        arc_strip.collides_with_circle(Disk::new(pos_b2, rad_b)) || last_pos_b2_maybe.map_or(false, |pos| {
                            arc_strip.collides_with_circle(Disk::new(pos, rad_b))
                        })
                    }
                    && (pos_b_ground - pos.0).angle_between(pos_b.0 - pos.0) < max_angle
                    && (!shockwave.requires_ground || physics_state_b.on_ground);

                if hit {
                    for (target, damage) in shockwave.damages.iter() {
                        if let Some(target) = target {
                            if *target != target_group {
                                continue;
                            }
                        }

                        let owner_uid = shockwave.owner.unwrap_or(*uid);
                        let change = damage.modify_damage(inventories.get(b), Some(owner_uid));

                        server_emitter.emit(ServerEvent::Damage { entity: b, change });
                        shockwave_hit_list.hit_entities.push(*uid_b);

                        let kb_dir = Dir::new((pos_b.0 - pos.0).try_normalized().unwrap_or(*ori.0));
                        let impulse = shockwave.knockback.calculate_impulse(kb_dir);
                        if !impulse.is_approx_zero() {
                            server_emitter.emit(ServerEvent::Knockback { entity: b, impulse });
                        }
                    }
                }
            }
        }

        // Set start time on new shockwaves
        // This change doesn't need to be recorded as it is not sent to the client
        shockwaves.set_event_emission(false);
        (&mut shockwaves).join().for_each(|mut shockwave| {
            if shockwave.creation.is_none() {
                shockwave.creation = Some(time);
            }
        });
        shockwaves.set_event_emission(true);
    }
}

#[derive(Clone, Copy)]
struct ArcStrip {
    origin: Vec2<f32>,
    /// Normalizable direction
    dir: Vec2<f32>,
    /// Angle in degrees
    angle: f32,
    /// Start radius
    start: f32,
    /// End radius
    end: f32,
}

impl ArcStrip {
    fn collides_with_circle(self, d: Disk<f32, f32>) -> bool {
        // Quit if aabb's don't collide
        if (self.origin.x - d.center.x).abs() > self.end + d.radius
            || (self.origin.y - d.center.y).abs() > self.end + d.radius
        {
            return false;
        }

        let dist = self.origin.distance(d.center);
        let half_angle = self.angle.to_radians() / 2.0;

        if dist > self.end + d.radius || dist + d.radius < self.start {
            // Completely inside or outside full ring
            return false;
        }

        let inside_edge = Disk::new(self.origin, self.start);
        let outside_edge = Disk::new(self.origin, self.end);
        let inner_corner_in_circle = || {
            let midpoint = self.dir.normalized() * self.start;
            d.contains_point(midpoint.rotated_z(half_angle) + self.origin)
                || d.contains_point(midpoint.rotated_z(-half_angle) + self.origin)
        };
        let arc_segment_in_circle = || {
            let midpoint = self.dir.normalized();
            let segment_in_circle = |angle| {
                let dir = midpoint.rotated_z(angle);
                let side = LineSegment2 {
                    start: dir * self.start + self.origin,
                    end: dir * self.end + self.origin,
                };
                d.contains_point(side.projected_point(d.center))
            };
            segment_in_circle(half_angle) || segment_in_circle(-half_angle)
        };

        if dist > self.end {
            // Circle center is outside ring
            // Check intersection with line segments
            arc_segment_in_circle() || {
                // Check angle of intersection points on outside edge of ring
                let (p1, p2) = intersection_points(outside_edge, d, dist);
                self.dir.angle_between(p1 - self.origin) < half_angle
                    || self.dir.angle_between(p2 - self.origin) < half_angle
            }
        } else if dist < self.start {
            // Circle center is inside ring
            // Check angle of intersection points on inside edge of ring
            // Check if circle contains one of the inner points of the arc
            inner_corner_in_circle()
                || (
                    // Check that the circles aren't identical
                    inside_edge != d && {
                        let (p1, p2) = intersection_points(inside_edge, d, dist);
                        self.dir.angle_between(p1 - self.origin) < half_angle
                            || self.dir.angle_between(p2 - self.origin) < half_angle
                    }
                )
        } else if d.radius > dist {
            // Circle center inside ring
            // but center of ring is inside the circle so we can't calculate the angle
            inner_corner_in_circle()
        } else {
            // Circle center inside ring
            // Calculate extra angle to account for circle radius
            let extra_angle = (d.radius / dist).asin();
            self.dir.angle_between(d.center - self.origin) < half_angle + extra_angle
        }
    }
}

// Assumes an intersection is occuring at 2 points
// Uses precalculated distance
// https://www.xarg.org/2016/07/calculate-the-intersection-points-of-two-circles/
fn intersection_points(
    disk1: Disk<f32, f32>,
    disk2: Disk<f32, f32>,
    dist: f32,
) -> (Vec2<f32>, Vec2<f32>) {
    let e = (disk2.center - disk1.center) / dist;

    let x = (disk1.radius.powi(2) - disk2.radius.powi(2) + dist.powi(2)) / (2.0 * dist);
    let y = (disk1.radius.powi(2) - x.powi(2)).sqrt();

    let pxe = disk1.center + x * e;
    let eyx = e.yx();

    let p1 = pxe + Vec2::new(-y, y) * eyx;
    let p2 = pxe + Vec2::new(y, -y) * eyx;

    (p1, p2)
}
