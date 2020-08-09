use crate::{
    comp::{
        group, Body, CharacterState, Damage, DamageSource, HealthChange, HealthSource, Last,
        Loadout, Ori, PhysicsState, Pos, Scale, Shockwave, Stats,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    state::{DeltaTime, Time},
    sync::{Uid, UidAllocator},
    util::Dir,
};
use specs::{saveload::MarkerAllocator, Entities, Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

pub const BLOCK_ANGLE: f32 = 180.0;

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
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Loadout>,
        ReadStorage<'a, group::Group>,
        ReadStorage<'a, CharacterState>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, Shockwave>,
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
            stats,
            loadouts,
            groups,
            character_states,
            physics_states,
            mut shockwaves,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();
        let mut local_emitter = local_bus.emitter();

        let time = time.0;
        let dt = dt.0;

        // Shockwaves
        for (entity, uid, pos, ori, shockwave) in
            (&entities, &uids, &positions, &orientations, &shockwaves).join()
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
            if end_time < time {
                server_emitter.emit(ServerEvent::Destroy {
                    entity,
                    cause: HealthSource::World,
                });
            }

            // Determine area that was covered by the shockwave in the last tick
            let frame_time = dt.min((end_time - time) as f32);
            if frame_time <= 0.0 {
                continue;
            }

            // Note: min() probably uneeded
            let time_since_creation = (time - creation_time) as f32;
            let frame_start_dist = (shockwave.speed * (time_since_creation - frame_time)).max(0.0);
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
                ori_b,
                scale_b_maybe,
                character_b,
                stats_b,
                body_b,
                physics_state_b,
            ) in (
                &entities,
                &uids,
                &positions,
                // TODO: make sure that these are maintained on the client and remove `.maybe()`
                last_positions.maybe(),
                &orientations,
                scales.maybe(),
                character_states.maybe(),
                &stats,
                &bodies,
                &physics_states,
            )
                .join()
            {
                // 2D versions
                let pos_b2 = pos_b.0.xy();
                let last_pos_b2_maybe = last_pos_b_maybe.map(|p| (p.0).0.xy());

                // Scales
                let scale_b = scale_b_maybe.map_or(1.0, |s| s.0);
                let rad_b = body_b.radius() * scale_b;

                // Check if it is a hit
                let hit = entity != b
                    && !stats_b.is_dead
                    // Collision shapes
                    && {
                        // TODO: write code to collide rect with the arc strip so that we can do
                        // more complete collision detection for rapidly moving entities
                        arc_strip.collides_with_circle(Circle {
                            pos: pos_b2,
                            radius: rad_b,
                        }) || last_pos_b2_maybe.map_or(false, |pos| {
                            arc_strip.collides_with_circle(Circle { pos, radius: rad_b })
                        })
                    }
                    && (!shockwave.requires_ground || physics_state_b.on_ground);

                if hit {
                    // See if entities are in the same group
                    let same_group = group
                        .map(|group_a| Some(group_a) == groups.get(b))
                        .unwrap_or(Some(*uid_b) == shockwave.owner);

                    // Don't damage in the same group
                    if same_group {
                        continue;
                    }

                    // Weapon gives base damage
                    let source = DamageSource::Shockwave;

                    let mut damage = Damage {
                        healthchange: -(shockwave.damage as f32),
                        source,
                    };

                    let block = character_b.map(|c_b| c_b.is_block()).unwrap_or(false)
                        // TODO: investigate whether this calculation is proper for shockwaves
                        && ori_b.0.angle_between(pos.0 - pos_b.0) < BLOCK_ANGLE.to_radians() / 2.0;

                    if let Some(loadout) = loadouts.get(b) {
                        damage.modify_damage(block, loadout);
                    }

                    if damage.healthchange != 0.0 {
                        server_emitter.emit(ServerEvent::Damage {
                            uid: *uid_b,
                            change: HealthChange {
                                amount: damage.healthchange as i32,
                                cause: HealthSource::Attack {
                                    by: shockwave.owner.unwrap_or(*uid),
                                },
                            },
                        });
                    }
                    if shockwave.knockback != 0.0 {
                        if shockwave.knockback < 0.0 {
                            local_emitter.emit(LocalEvent::ApplyForce {
                                entity: b,
                                force: shockwave.knockback
                                    * *Dir::slerp(ori.0, Dir::new(Vec3::new(0.0, 0.0, -1.0)), 0.5),
                            });
                        } else {
                            local_emitter.emit(LocalEvent::ApplyForce {
                                entity: b,
                                force: shockwave.knockback
                                    * *Dir::slerp(ori.0, Dir::new(Vec3::new(0.0, 0.0, 1.0)), 0.5),
                            });
                        }
                    }
                }
            }
        }

        // Set start time on new shockwaves
        // This change doesn't need to be recorded as it is not sent to the client
        shockwaves.set_event_emission(false);
        (&mut shockwaves).join().for_each(|shockwave| {
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
    fn collides_with_circle(self, c: Circle) -> bool {
        // Quit if aabb's don't collide
        if (self.origin.x - c.pos.x).abs() > self.end + c.radius
            || (self.origin.y - c.pos.y).abs() > self.end + c.radius
        {
            return false;
        }

        let dist = self.origin.distance(c.pos);
        let half_angle = self.angle.to_radians() / 2.0;

        if dist > self.end + c.radius || dist + c.radius < self.start {
            // Completely inside or outside full ring
            return false;
        }

        let inside_edge = Circle {
            pos: self.origin,
            radius: self.start,
        };
        let outside_edge = Circle {
            pos: self.origin,
            radius: self.end,
        };
        let inner_corner_in_circle = || {
            let midpoint = self.dir.normalized() * self.start;
            c.contains_point(midpoint.rotated_z(half_angle) + self.origin)
                || c.contains_point(midpoint.rotated_z(-half_angle) + self.origin)
        };
        let arc_segment_in_circle = || {
            let midpoint = self.dir.normalized();
            let segment_in_circle = |angle| {
                let dir = midpoint.rotated_z(angle);
                let side = LineSegment2 {
                    start: dir * self.start + self.origin,
                    end: dir * self.end + self.origin,
                };
                c.contains_point(side.projected_point(c.pos))
            };
            segment_in_circle(half_angle) || segment_in_circle(-half_angle)
        };

        if dist > self.end {
            // Circle center is outside ring
            // Check intersection with line segments
            arc_segment_in_circle() || {
                // Check angle of intersection points on outside edge of ring
                let (p1, p2) = outside_edge.intersection_points(c, dist);
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
                    !inside_edge.is_approx_eq(c) && {
                        let (p1, p2) = inside_edge.intersection_points(c, dist);
                        self.dir.angle_between(p1 - self.origin) < half_angle
                            || self.dir.angle_between(p2 - self.origin) < half_angle
                    }
                )
        } else if c.radius > dist {
            // Circle center inside ring
            // but center of ring is inside the circle so we can't calculate the angle
            inner_corner_in_circle()
        } else {
            // Circle center inside ring
            // Calculate extra angle to account for circle radius
            let extra_angle = (c.radius / dist).asin();
            self.dir.angle_between(c.pos - self.origin) < half_angle + extra_angle
        }
    }
}

#[derive(Clone, Copy)]
struct Circle {
    pos: Vec2<f32>,
    radius: f32,
}
impl Circle {
    // Assumes an intersection is occuring at 2 points
    // Uses precalculated distance
    // https://www.xarg.org/2016/07/calculate-the-intersection-points-of-two-circles/
    fn intersection_points(self, other: Self, dist: f32) -> (Vec2<f32>, Vec2<f32>) {
        let e = (other.pos - self.pos) / dist;

        let x = (self.radius.powi(2) - other.radius.powi(2) + dist.powi(2)) / (2.0 * dist);
        let y = (self.radius.powi(2) - x.powi(2)).sqrt();

        let pxe = self.pos + x * e;
        let eyx = e.yx();

        let p1 = pxe + Vec2::new(-y, y) * eyx;
        let p2 = pxe + Vec2::new(y, -y) * eyx;

        (p1, p2)
    }

    fn contains_point(self, point: Vec2<f32>) -> bool {
        point.distance_squared(self.pos) < self.radius.powi(2)
    }

    fn is_approx_eq(self, other: Self) -> bool {
        (self.pos - other.pos).is_approx_zero() && self.radius - other.radius < 0.001
    }
}
