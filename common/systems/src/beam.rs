use common::{
    combat::{AttackSource, AttackerInfo, TargetInfo},
    comp::{
        Beam, BeamSegment, Body, CharacterState, Combo, Energy, Group, Health, HealthSource,
        Inventory, Ori, Pos, Scale, Stats,
    },
    event::{EventBus, ServerEvent},
    outcome::Outcome,
    resources::{DeltaTime, Time},
    terrain::TerrainGrid,
    uid::{Uid, UidAllocator},
    vol::ReadVol,
    GroupTarget,
};
use common_ecs::{Job, Origin, ParMode, Phase, System};
use rayon::iter::ParallelIterator;
use specs::{
    saveload::MarkerAllocator, shred::ResourceId, Entities, Join, ParJoin, Read, ReadExpect,
    ReadStorage, SystemData, World, Write, WriteStorage,
};
use std::time::Duration;
use vek::*;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    time: Read<'a, Time>,
    dt: Read<'a, DeltaTime>,
    terrain: ReadExpect<'a, TerrainGrid>,
    uid_allocator: Read<'a, UidAllocator>,
    uids: ReadStorage<'a, Uid>,
    positions: ReadStorage<'a, Pos>,
    orientations: ReadStorage<'a, Ori>,
    scales: ReadStorage<'a, Scale>,
    bodies: ReadStorage<'a, Body>,
    healths: ReadStorage<'a, Health>,
    inventories: ReadStorage<'a, Inventory>,
    groups: ReadStorage<'a, Group>,
    energies: ReadStorage<'a, Energy>,
    stats: ReadStorage<'a, Stats>,
    combos: ReadStorage<'a, Combo>,
    character_states: ReadStorage<'a, CharacterState>,
}

/// This system is responsible for handling beams that heal or do damage
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, BeamSegment>,
        WriteStorage<'a, Beam>,
        Write<'a, Vec<Outcome>>,
    );

    const NAME: &'static str = "beam";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        job: &mut Job<Self>,
        (read_data, mut beam_segments, mut beams, mut outcomes): Self::SystemData,
    ) {
        let mut server_emitter = read_data.server_bus.emitter();

        let time = read_data.time.0;
        let dt = read_data.dt.0;

        job.cpu_stats.measure(ParMode::Rayon);

        // Beams
        let (server_events, add_hit_entities, mut new_outcomes) = (
            &read_data.entities,
            &read_data.positions,
            &read_data.orientations,
            &beam_segments,
        )
            .par_join()
            .fold(|| (Vec::new(), Vec::new(), Vec::new()),
            |(mut server_events, mut add_hit_entities, mut outcomes),
            (entity, pos, ori, beam_segment)|
        {
            let creation_time = match beam_segment.creation {
                Some(time) => time,
                // Skip newly created beam segments
                None => return (server_events, add_hit_entities, outcomes),
            };
            let end_time = creation_time + beam_segment.duration.as_secs_f64();
            // If beam segment is out of time emit destroy event but still continue since it
            // may have traveled and produced effects a bit before reaching its
            // end point
            if end_time < time {
                server_events.push(ServerEvent::Destroy {
                    entity,
                    cause: HealthSource::World,
                });
            }

            // Determine area that was covered by the beam in the last tick
            let frame_time = dt.min((end_time - time) as f32);
            if frame_time <= 0.0 {
                return (server_events, add_hit_entities, outcomes);
            }
            // Note: min() probably uneeded
            let time_since_creation = (time - creation_time) as f32;
            let frame_start_dist =
                (beam_segment.speed * (time_since_creation - frame_time)).max(0.0);
            let frame_end_dist = (beam_segment.speed * time_since_creation).max(frame_start_dist);

            let beam_owner = beam_segment
                .owner
                .and_then(|uid| read_data.uid_allocator.retrieve_entity_internal(uid.into()));

            // Group to ignore collisions with
            // Might make this more nuanced if beams are used for non damage effects
            let group = beam_owner.and_then(|e| read_data.groups.get(e));

            let hit_entities = if let Some(beam) = beam_owner.and_then(|e| beams.get(e)) {
                &beam.hit_entities
            } else {
                return (server_events, add_hit_entities, outcomes);
            };

            // Go through all other effectable entities
            for (target, uid_b, pos_b, health_b, body_b) in (
                &read_data.entities,
                &read_data.uids,
                &read_data.positions,
                &read_data.healths,
                &read_data.bodies,
            )
                .join()
            {
                // Check to see if entity has already been hit recently
                if hit_entities.iter().any(|&uid| uid == *uid_b) {
                    continue;
                }

                // Scales
                let scale_b = read_data.scales.get(target).map_or(1.0, |s| s.0);
                let rad_b = body_b.radius() * scale_b;
                let height_b = body_b.height() * scale_b;

                // Check if it is a hit
                let hit = entity != target
                    && !health_b.is_dead
                    // Collision shapes
                    && sphere_wedge_cylinder_collision(pos.0, frame_start_dist, frame_end_dist, *ori.look_dir(), beam_segment.angle, pos_b.0, rad_b, height_b);

                // Finally, ensure that a hit has actually occurred by performing a raycast. We do this last because
                // it's likely to be the most expensive operation.
                let tgt_dist = pos.0.distance(pos_b.0);
                let hit = hit && read_data.terrain
                    .ray(pos.0, pos.0 + *ori.look_dir() * (tgt_dist + 1.0))
                    .until(|b| b.is_filled())
                    .cast().0 >= tgt_dist;

                if hit {
                    // See if entities are in the same group
                    let same_group = group
                        .map(|group_a| Some(group_a) == read_data.groups.get(target))
                        .unwrap_or(Some(*uid_b) == beam_segment.owner);

                    let target_group = if same_group {
                        GroupTarget::InGroup
                    } else {
                        GroupTarget::OutOfGroup
                    };

                    // If owner, shouldn't heal or damage
                    if Some(*uid_b) == beam_segment.owner {
                        continue;
                    }

                    let attacker_info =
                        beam_owner
                            .zip(beam_segment.owner)
                            .map(|(entity, uid)| AttackerInfo {
                                entity,
                                uid,
                                energy: read_data.energies.get(entity),
                                combo: read_data.combos.get(entity),
                            });

                    let target_info = TargetInfo {
                        entity: target,
                        inventory: read_data.inventories.get(target),
                        stats: read_data.stats.get(target),
                        health: read_data.healths.get(target),
                        pos: pos.0,
                        ori: read_data.orientations.get(target),
                        char_state: read_data.character_states.get(target),
                    };

                    beam_segment.properties.attack.apply_attack(
                        target_group,
                        attacker_info,
                        target_info,
                        ori.look_dir(),
                        false,
                        1.0,
                        AttackSource::Beam,
                        |e| server_events.push(e),
                        |o| outcomes.push(o),
                    );

                    add_hit_entities.push((beam_owner, *uid_b));
                }
            }
            (server_events, add_hit_entities, outcomes)
        }).reduce(|| (Vec::new(), Vec::new(), Vec::new()),
            |(mut events_a, mut hit_entities_a, mut outcomes_a),
            (mut events_b, mut hit_entities_b, mut outcomes_b)| {
                events_a.append(&mut events_b);
                hit_entities_a.append(&mut hit_entities_b);
                outcomes_a.append(&mut outcomes_b);
                (events_a, hit_entities_a, outcomes_a)
            });
        job.cpu_stats.measure(ParMode::Single);
        outcomes.append(&mut new_outcomes);

        for event in server_events {
            server_emitter.emit(event);
        }

        for (owner, hit_entity) in add_hit_entities {
            if let Some(ref mut beam) = owner.and_then(|e| beams.get_mut(e)) {
                beam.hit_entities.push(hit_entity);
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
        (&mut beam_segments).join().for_each(|mut beam_segment| {
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
