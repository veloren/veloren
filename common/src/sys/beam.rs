use crate::{
    comp::{
        group, Beam, BeamSegment, Body, CharacterState, Energy, EnergyChange, EnergySource,
        HealthChange, HealthSource, Last, Loadout, Ori, Pos, Scale, Stats,
    },
    event::{EventBus, ServerEvent},
    state::{DeltaTime, Time},
    sync::{Uid, UidAllocator},
    DamageSource,
};
use specs::{saveload::MarkerAllocator, Entities, Join, Read, ReadStorage, System, WriteStorage};
use std::time::Duration;
use vek::*;

pub const BLOCK_ANGLE: f32 = 180.0;

/// This system is responsible for handling beams that heal or do damage
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
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
        ReadStorage<'a, Energy>,
        WriteStorage<'a, BeamSegment>,
        WriteStorage<'a, Beam>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_bus,
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
            energies,
            mut beam_segments,
            mut beams,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();

        let time = time.0;
        let dt = dt.0;

        // Beams
        for (entity, uid, pos, ori, beam_segment) in
            (&entities, &uids, &positions, &orientations, &beam_segments).join()
        {
            let creation_time = match beam_segment.creation {
                Some(time) => time,
                // Skip newly created beam segments
                None => continue,
            };

            let end_time = creation_time + beam_segment.duration.as_secs_f64();

            // If beam segment is out of time emit destroy event but still continue since it
            // may have traveled and produced effects a bit before reaching it's
            // end point
            if end_time < time {
                server_emitter.emit(ServerEvent::Destroy {
                    entity,
                    cause: HealthSource::World,
                });
            }

            // Determine area that was covered by the beam in the last tick
            let frame_time = dt.min((end_time - time) as f32);
            if frame_time <= 0.0 {
                continue;
            }

            // Note: min() probably uneeded
            let time_since_creation = (time - creation_time) as f32;
            let frame_start_dist =
                (beam_segment.speed * (time_since_creation - frame_time)).max(0.0);
            let frame_end_dist = (beam_segment.speed * time_since_creation).max(frame_start_dist);

            let beam_owner = beam_segment
                .owner
                .and_then(|uid| uid_allocator.retrieve_entity_internal(uid.into()));

            // Group to ignore collisions with
            // Might make this more nuanced if beams are used for non damage effects
            let group = beam_owner.and_then(|e| groups.get(e));

            let hit_entities = if let Some(beam) = beam_owner.and_then(|e| beams.get_mut(e)) {
                &mut beam.hit_entities
            } else {
                continue;
            };

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
            )
                .join()
            {
                // Check to see if entity has already been hit recently
                if hit_entities.iter().any(|&uid| uid == *uid_b) {
                    continue;
                }

                // Scales
                let scale_b = scale_b_maybe.map_or(1.0, |s| s.0);
                let rad_b = body_b.radius() * scale_b;
                let height_b = body_b.height() * scale_b;

                // Check if it is a hit
                let hit = entity != b
                    && !stats_b.is_dead
                    // Collision shapes
                    && (sphere_wedge_cylinder_collision(pos.0, frame_start_dist, frame_end_dist, *ori.0, beam_segment.angle, pos_b.0, rad_b, height_b)
                    || last_pos_b_maybe.map_or(false, |pos_maybe| {sphere_wedge_cylinder_collision(pos.0, frame_start_dist, frame_end_dist, *ori.0, beam_segment.angle, (pos_maybe.0).0, rad_b, height_b)}));

                if hit {
                    // See if entities are in the same group
                    let same_group = group
                        .map(|group_a| Some(group_a) == groups.get(b))
                        .unwrap_or(Some(*uid_b) == beam_segment.owner);

                    // If owner, shouldn't heal or damage
                    if Some(*uid_b) == beam_segment.owner {
                        continue;
                    }

                    let damage = if let Some(damage) = beam_segment.damages.get_damage(same_group) {
                        damage
                    } else {
                        continue;
                    };

                    let block = character_b.map(|c_b| c_b.is_block()).unwrap_or(false)
                        // TODO: investigate whether this calculation is proper for beams
                        && ori_b.0.angle_between(pos.0 - pos_b.0) < BLOCK_ANGLE.to_radians() / 2.0;

                    let change = damage.modify_damage(block, loadouts.get(b), beam_segment.owner);

                    if !matches!(damage.source, DamageSource::Healing) {
                        server_emitter.emit(ServerEvent::Damage {
                            uid: *uid_b,
                            change,
                        });
                        if beam_segment.lifesteal_eff > 0.0 {
                            server_emitter.emit(ServerEvent::Damage {
                                uid: beam_segment.owner.unwrap_or(*uid),
                                change: HealthChange {
                                    amount: (-change.amount as f32 * beam_segment.lifesteal_eff)
                                        as i32,
                                    cause: HealthSource::Healing {
                                        by: beam_segment.owner,
                                    },
                                },
                            });
                        }
                        if let Some(uid) = beam_segment.owner {
                            server_emitter.emit(ServerEvent::EnergyChange {
                                uid,
                                change: EnergyChange {
                                    amount: beam_segment.energy_regen as i32,
                                    source: EnergySource::HitEnemy,
                                },
                            });
                        }
                    } else if let Some(energy) = beam_owner.and_then(|o| energies.get(o)) {
                        if energy.current() > beam_segment.energy_cost {
                            if let Some(uid) = beam_segment.owner {
                                server_emitter.emit(ServerEvent::EnergyChange {
                                    uid,
                                    change: EnergyChange {
                                        amount: -(beam_segment.energy_cost as i32), // Stamina use
                                        source: EnergySource::Ability,
                                    },
                                })
                            }
                            server_emitter.emit(ServerEvent::Damage {
                                uid: *uid_b,
                                change,
                            });
                        }
                    }
                    // Adds entities that were hit to the hit_entities list on the beam, sees if it
                    // needs to purge the hit_entities list
                    hit_entities.push(*uid_b);
                }
            }
        }

        for beam in (&mut beams).join() {
            beam.timer = beam
                .timer
                .checked_add(Duration::from_secs_f32(dt))
                .unwrap_or(beam.tick_dur);
            if beam.timer >= beam.tick_dur {
                beam.hit_entities.clear();
                beam.timer = beam.timer.checked_sub(beam.tick_dur).unwrap_or_default();
            }
        }

        // Set start time on new beams
        // This change doesn't need to be recorded as it is not sent to the client
        beam_segments.set_event_emission(false);
        (&mut beam_segments).join().for_each(|beam_segment| {
            if beam_segment.creation.is_none() {
                beam_segment.creation = Some(time);
            }
        });
        beam_segments.set_event_emission(true);
    }
}

/// Assumes upright cylinder
/// See page 12 of https://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.396.7952&rep=rep1&type=pdf
#[allow(clippy::too_many_arguments)]
fn sphere_wedge_cylinder_collision(
    // Values for spherical wedge
    real_pos: Vec3<f32>,
    min_rad: f32, // Distance from beam origin to inner section of beam
    max_rad: f32, //Distance from beam origin to outer section of beam
    ori: Vec3<f32>,
    angle: f32,
    // Values for cylinder
    bottom_pos_b: Vec3<f32>, // Position of bottom of cylinder
    rad_b: f32,
    length_b: f32,
) -> bool {
    // Converts all coordinates so that the new origin is in the center of the
    // cylinder
    let center_pos_b = Vec3::new(
        bottom_pos_b.x,
        bottom_pos_b.y,
        bottom_pos_b.z + length_b / 2.0,
    );
    let pos = real_pos - center_pos_b;
    let pos_b = Vec3::zero();
    if pos.distance_squared(pos_b) > (max_rad + rad_b + length_b).powi(2) {
        // Does quick check if entity is too far (I'm not sure if necessary, but
        // probably makes detection more efficient)
        false
    } else if pos.z.abs() <= length_b / 2.0 {
        // Checks case 1: center of sphere is on same z-height as cylinder
        let pos2 = Vec2::<f32>::from(pos);
        let ori2 = Vec2::from(ori);
        let distance = pos2.distance(Vec2::zero());
        let in_range = distance < max_rad && distance > min_rad;
        // Done so that if distance = 0, atan() can still be calculated https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=6d2221bb9454debdfca8f9c52d1edb29
        let tangent_value1: f32 = rad_b / distance;
        let tangent_value2: f32 = length_b / 2.0 / distance;
        let in_angle = pos2.angle_between(-ori2) < angle + (tangent_value1).atan().abs()
            && pos.angle_between(-ori) < angle + (tangent_value2).atan().abs();
        in_range && in_angle
    } else {
        // Checks case 2: if sphere collides with top/bottom of cylinder, doesn't use
        // paper. Logic used here is it checks if line between centers passes through
        // either cap, then if the cap is within range, then if withing angle of beam.
        // If line
        let sign = if pos.z > 0.0 { 1.0 } else { -1.0 };
        let height = sign * length_b / 2.0;
        let (in_range, in_angle): (bool, bool);
        // Gets relatively how far along the line (between sphere and cylinder centers)
        // the endcap of the cylinder is, is between 0 and 1 when sphere center is not
        // in cylinder
        let intersect_frac = (length_b / 2.0 / pos.z).abs();
        // Gets the position of the cylinder edge closest to the sphere center
        let edge_pos = if let Some(vec) = Vec3::new(pos.x, pos.y, 0.0).try_normalized() {
            vec * rad_b
        } else {
            // Returns an arbitrary location that is still guaranteed to be on the cylinder
            // edge. This case should only happen when the sphere is directly above the
            // cylinder, in which case all positions on edge are equally close.
            Vec3::new(rad_b, 0.0, 0.0)
        };
        // Gets position on opposite edge of same endcap
        let opp_end_edge_pos = Vec3::new(-edge_pos.x, -edge_pos.y, height);
        // Gets position on same edge of opposite endcap
        let bot_end_edge_pos = Vec3::new(edge_pos.x, edge_pos.y, -height);
        // Gets point on line between sphere and cylinder centers that the z value is
        // equal to the endcap z location
        let intersect_point = Vec2::new(pos.x * intersect_frac, pos.y * intersect_frac);
        // Checks if line between sphere and cylinder center passes through cap of
        // cylinder
        if intersect_point.distance_squared(Vec2::zero()) <= rad_b.powi(2) {
            let distance_squared =
                Vec3::new(intersect_point.x, intersect_point.y, height).distance_squared(pos);
            in_range = distance_squared < max_rad.powi(2) && distance_squared > min_rad.powi(2);
            // Angle between (line between centers of cylinder and sphere) and either (line
            // between opposite edge of endcap and sphere center) or (line between close
            // edge of endcap on bottom of cylinder and sphere center). Whichever angle is
            // largest is used.
            let angle2 = (pos_b - pos)
                .angle_between(opp_end_edge_pos - pos)
                .max((pos_b - pos).angle_between(bot_end_edge_pos - pos));
            in_angle = pos.angle_between(-ori) < angle + angle2;
        } else {
            // TODO: Handle collision for this case more accurately
            // For this case, the nearest point will be the edge of the endcap
            let endcap_edge_pos = Vec3::new(edge_pos.x, edge_pos.y, height);
            let distance_squared = endcap_edge_pos.distance_squared(pos);
            in_range = distance_squared > min_rad.powi(2) && distance_squared < max_rad.powi(2);
            // Gets side positions on same endcap
            let side_end_edge_pos_1 = Vec3::new(edge_pos.y, -edge_pos.x, height);
            let side_end_edge_pos_2 = Vec3::new(-edge_pos.y, edge_pos.x, height);
            // Gets whichever angle is bigger, between sphere center and opposite edge,
            // sphere center and bottom edge, or half of sphere center and both the side
            // edges
            let angle2 = (pos_b - pos).angle_between(opp_end_edge_pos - pos).max(
                (pos_b - pos).angle_between(bot_end_edge_pos - pos).max(
                    (side_end_edge_pos_1 - pos).angle_between(side_end_edge_pos_2 - pos) / 2.0,
                ),
            );
            // Will be somewhat inaccurate, tends towards hitting when it shouldn't
            // Checks angle between orientation and line between sphere and cylinder centers
            in_angle = pos.angle_between(-ori) < angle + angle2;
        }
        in_range && in_angle
    }
}
