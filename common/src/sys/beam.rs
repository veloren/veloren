use crate::{
    comp::{
        group, Beam, Body, CharacterState, Damage, DamageSource, Energy, EnergySource,
        HealthChange, HealthSource, Last, Loadout, Ori, Pos, Scale, Stats,
    },
    event::{EventBus, ServerEvent},
    state::{DeltaTime, Time},
    sync::{Uid, UidAllocator},
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
        WriteStorage<'a, Energy>,
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
            mut energies,
            mut beams,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();

        let time = time.0;
        let dt = dt.0;

        // Beams
        for (entity, uid, pos, ori, beam) in
            (&entities, &uids, &positions, &orientations, &beams).join()
        {
            let creation_time = match beam.creation {
                Some(time) => time,
                // Skip newly created beam segments
                None => continue,
            };

            let end_time = creation_time + beam.duration.as_secs_f64();

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
            let frame_start_dist = (beam.speed * (time_since_creation - frame_time)).max(0.0);
            let frame_end_dist = (beam.speed * time_since_creation).max(frame_start_dist);

            // Group to ignore collisions with
            // Might make this more nuanced if beams are used for non damage effects
            let group = beam
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
                // Scales
                let scale_b = scale_b_maybe.map_or(1.0, |s| s.0);
                let rad_b = body_b.radius() * scale_b;
                let height_b = body_b.height() * scale_b;

                // Check if it is a hit
                let hit = entity != b
                    && !stats_b.is_dead
                    // Collision shapes
                    && (sphere_wedge_cylinder_collision(pos.0, frame_start_dist, frame_end_dist, *ori.0, beam.angle, pos_b.0, rad_b, height_b)
                    || last_pos_b_maybe.map_or(false, |pos_maybe| {sphere_wedge_cylinder_collision(pos.0, frame_start_dist, frame_end_dist, *ori.0, beam.angle, (pos_maybe.0).0, rad_b, height_b)}));

                if hit {
                    // See if entities are in the same group
                    let same_group = group
                        .map(|group_a| Some(group_a) == groups.get(b))
                        .unwrap_or(Some(*uid_b) == beam.owner);

                    // If owner, shouldn't heal or damage
                    if Some(*uid_b) == beam.owner {
                        continue;
                    }
                    // Don't heal if outside group
                    // Don't damage in the same group
                    let (mut is_heal, mut is_damage) = (false, false);
                    if !same_group && (beam.damage > 0) {
                        is_damage = true;
                    }
                    if same_group && (beam.heal > 0) {
                        is_heal = true;
                    }
                    if !is_heal && !is_damage {
                        continue;
                    }

                    // Weapon gives base damage
                    let source = if is_heal {
                        DamageSource::Healing
                    } else {
                        DamageSource::Energy
                    };
                    let healthchange = if is_heal {
                        beam.heal as f32
                    } else {
                        -(beam.damage as f32)
                    };

                    let mut damage = Damage {
                        healthchange,
                        source,
                    };

                    let block = character_b.map(|c_b| c_b.is_block()).unwrap_or(false)
                        // TODO: investigate whether this calculation is proper for beams
                        && ori_b.0.angle_between(pos.0 - pos_b.0) < BLOCK_ANGLE.to_radians() / 2.0;

                    if let Some(loadout) = loadouts.get(b) {
                        damage.modify_damage(block, loadout);
                    }

                    if is_damage {
                        server_emitter.emit(ServerEvent::Damage {
                            uid: *uid_b,
                            change: HealthChange {
                                amount: damage.healthchange as i32,
                                cause: HealthSource::Energy { owner: beam.owner },
                            },
                        });
                        server_emitter.emit(ServerEvent::Damage {
                            uid: beam.owner.unwrap_or(*uid),
                            change: HealthChange {
                                amount: (-damage.healthchange * beam.lifesteal_eff) as i32,
                                cause: HealthSource::Healing { by: beam.owner },
                            },
                        });
                        if let Some(energy_mut) = beam
                            .owner
                            .and_then(|o| uid_allocator.retrieve_entity_internal(o.into()))
                            .and_then(|o| energies.get_mut(o))
                        {
                            energy_mut.change_by(beam.energy_regen as i32, EnergySource::HitEnemy);
                        }
                    }
                    if is_heal {
                        if let Some(energy_mut) = beam
                            .owner
                            .and_then(|o| uid_allocator.retrieve_entity_internal(o.into()))
                            .and_then(|o| energies.get_mut(o))
                        {
                            if energy_mut
                                .try_change_by(
                                    -(beam.energy_drain as i32), // Stamina use
                                    EnergySource::Ability,
                                )
                                .is_ok()
                            {
                                server_emitter.emit(ServerEvent::Damage {
                                    uid: *uid_b,
                                    change: HealthChange {
                                        amount: damage.healthchange as i32,
                                        cause: HealthSource::Healing { by: beam.owner },
                                    },
                                });
                            }
                        }
                    }
                }
            }
        }

        // Set start time on new beams
        // This change doesn't need to be recorded as it is not sent to the client
        beams.set_event_emission(false);
        (&mut beams).join().for_each(|beam| {
            if beam.creation.is_none() {
                beam.creation = Some(time);
            }
        });
        beams.set_event_emission(true);
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
        // Gets point on line between sphere and cylinder centers that the z value is
        // equal to the endcap z location
        let intersect_point = Vec2::new(pos.x * intersect_frac, pos.y * intersect_frac);
        // Checks if line between sphere and cylinder center passes through cap of
        // cylinder
        if intersect_point.distance_squared(Vec2::zero()) <= rad_b.powi(2) {
            let distance_squared =
                Vec3::new(intersect_point.x, intersect_point.y, height).distance_squared(pos);
            in_range = distance_squared < max_rad.powi(2) && distance_squared > min_rad.powi(2);
            // Changes position so I can compare this with origin instead of original
            // position with top of cylinder
            let mod_pos = Vec3::new(pos.x, pos.y, pos.z - height);
            // Angle between (line between center of endcap and sphere center) and (line
            // between edge of endcap and sphere center)
            let angle2 = (pos_b - mod_pos).angle_between(edge_pos - mod_pos);
            // The 1.25 gives margin for error
            in_angle = mod_pos.angle_between(-ori) < angle + (angle2 * 1.25);
        } else {
            // TODO: Handle collision for this case more accurately
            // For this case, the nearest point will be the edge of the endcap
            let endcap_edge_pos = Vec3::new(edge_pos.x, edge_pos.y, height);
            let distance_squared = endcap_edge_pos.distance_squared(pos);
            in_range = distance_squared > min_rad.powi(2) && distance_squared < max_rad.powi(2);
            // Gets position on opposite edge of same endcap
            let opp_end_edge_pos = Vec3::new(-edge_pos.x, -edge_pos.y, height);
            // Gets position on same edge of opposite endcap
            let bot_end_edge_pos = Vec3::new(edge_pos.x, edge_pos.y, -height);
            // Gets side positions on same endcap
            let side_end_edge_pos_1 = Vec3::new(edge_pos.y, -edge_pos.x, height);
            let side_end_edge_pos_2 = Vec3::new(-edge_pos.y, edge_pos.x, height);
            // Gets whichever angle is bigger, between half of sphere center and both
            // opposite edge and bottom edge, or sphere center and both the side edges
            let angle2 = (opp_end_edge_pos - pos)
                .angle_between(bot_end_edge_pos - pos)
                .min((side_end_edge_pos_1 - pos).angle_between(side_end_edge_pos_2 - pos));
            // Will be somewhat inaccurate, tends towards hitting when it shouldn't
            // Checks angle between orientation and line between sphere and cylinder centers
            in_angle = pos.angle_between(-ori) < angle + angle2;
        }
        in_range && in_angle
    }
}
