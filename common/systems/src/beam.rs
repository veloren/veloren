use common::{
    combat::{self, AttackOptions, AttackSource, AttackerInfo, TargetInfo},
    comp::{
        agent::{Sound, SoundKind},
        Alignment, Beam, Body, Buffs, CharacterState, Combo, Energy, Group, Health, Inventory, Ori,
        Player, Pos, Scale, Stats,
    },
    event::{self, EmitExt, EventBus},
    event_emitters,
    outcome::Outcome,
    resources::{DeltaTime, Time},
    terrain::TerrainGrid,
    uid::{IdMaps, Uid},
    vol::ReadVol,
    GroupTarget,
};
use common_ecs::{Job, Origin, ParMode, Phase, System};
use rand::Rng;
use rayon::iter::ParallelIterator;
use specs::{
    shred, Entities, LendJoin, ParJoin, Read, ReadExpect, ReadStorage, SystemData, WriteStorage,
};
use vek::*;

event_emitters! {
    struct ReadAttackEvents[AttackEmitters] {
        health_change: event::HealthChangeEvent,
        energy_change: event::EnergyChangeEvent,
        poise_change: event::PoiseChangeEvent,
        sound: event::SoundEvent,
        parry_hook: event::ParryHookEvent,
        kockback: event::KnockbackEvent,
        entity_attack_hoow: event::EntityAttackedHookEvent,
        combo_change: event::ComboChangeEvent,
        buff: event::BuffEvent,
    }
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    players: ReadStorage<'a, Player>,
    time: Read<'a, Time>,
    dt: Read<'a, DeltaTime>,
    terrain: ReadExpect<'a, TerrainGrid>,
    id_maps: Read<'a, IdMaps>,
    cached_spatial_grid: Read<'a, common::CachedSpatialGrid>,
    uids: ReadStorage<'a, Uid>,
    positions: ReadStorage<'a, Pos>,
    orientations: ReadStorage<'a, Ori>,
    alignments: ReadStorage<'a, Alignment>,
    scales: ReadStorage<'a, Scale>,
    bodies: ReadStorage<'a, Body>,
    healths: ReadStorage<'a, Health>,
    inventories: ReadStorage<'a, Inventory>,
    groups: ReadStorage<'a, Group>,
    energies: ReadStorage<'a, Energy>,
    stats: ReadStorage<'a, Stats>,
    combos: ReadStorage<'a, Combo>,
    character_states: ReadStorage<'a, CharacterState>,
    buffs: ReadStorage<'a, Buffs>,
    outcomes: Read<'a, EventBus<Outcome>>,
    events: ReadAttackEvents<'a>,
}

/// This system is responsible for handling beams that heal or do damage
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (ReadData<'a>, WriteStorage<'a, Beam>);

    const NAME: &'static str = "beam";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(job: &mut Job<Self>, (read_data, mut beams): Self::SystemData) {
        let mut outcomes_emitter = read_data.outcomes.emitter();

        (
            &read_data.positions,
            &read_data.orientations,
            &read_data.character_states,
            &mut beams,
        )
            .lend_join()
            .for_each(|(pos, ori, char_state, mut beam)| {
                // Clear hit entities list if list should be cleared
                if read_data.time.0 % beam.tick_dur.0 < read_data.dt.0 as f64 {
                    let (hit_entities, hit_durations) = beam.hit_entities_and_durations();
                    hit_durations.retain(|e, _| hit_entities.contains(e));
                    for entity in hit_entities {
                        *hit_durations.entry(*entity).or_insert(0) += 1;
                    }
                    beam.hit_entities.clear();
                }
                // Update start, end, and control positions of beam bezier
                let (offset, target_dir) = if let CharacterState::BasicBeam(c) = char_state {
                    (c.beam_offset, c.aim_dir)
                } else {
                    (Vec3::zero(), ori.look_dir())
                };
                beam.bezier.start = pos.0 + offset;
                const REL_CTRL_DIST: f32 = 0.3;
                let target_ctrl = beam.bezier.start + *target_dir * beam.range * REL_CTRL_DIST;
                let ctrl_translate = (target_ctrl - beam.bezier.ctrl) * read_data.dt.0
                    / (beam.duration.0 as f32 * REL_CTRL_DIST);
                beam.bezier.ctrl += ctrl_translate;
                let target_end = beam.bezier.start + *target_dir * beam.range;
                let end_translate =
                    (target_end - beam.bezier.end) * read_data.dt.0 / beam.duration.0 as f32;
                beam.bezier.end += end_translate;
            });

        job.cpu_stats.measure(ParMode::Rayon);

        // Beams
        // Emitters will append their events when dropped.
        let (_emitters, add_hit_entities, new_outcomes) = (
            &read_data.entities,
            &read_data.positions,
            &read_data.orientations,
            &read_data.uids,
            &beams,
        )
            .par_join()
            .fold(
                || (read_data.events.get_emitters(), Vec::new(), Vec::new()),
                |(mut emitters, mut add_hit_entities, mut outcomes),
                 (entity, pos, ori, uid, beam)| {
                    // Note: rayon makes it difficult to hold onto a thread-local RNG, if grabbing
                    // this becomes a bottleneck we can look into alternatives.
                    let mut rng = rand::thread_rng();
                    if rng.gen_bool(0.005) {
                        emitters.emit(event::SoundEvent {
                            sound: Sound::new(SoundKind::Beam, pos.0, 13.0, read_data.time.0),
                        });
                    }
                    outcomes.push(Outcome::Beam {
                        pos: pos.0,
                        specifier: beam.specifier,
                    });

                    // Group to ignore collisions with
                    // Might make this more nuanced if beams are used for non damage effects
                    let group = read_data.groups.get(entity);

                    // Go through all affectable entities by querying the spatial grid
                    let target_iter = read_data
                        .cached_spatial_grid
                        .0
                        .in_circle_aabr(beam.bezier.start.xy(), beam.range)
                        .filter_map(|target| {
                            read_data
                                .positions
                                .get(target)
                                .and_then(|l| read_data.healths.get(target).map(|r| (l, r)))
                                .and_then(|l| read_data.uids.get(target).map(|r| (l, r)))
                                .and_then(|l| read_data.bodies.get(target).map(|r| (l, r)))
                                .map(|(((pos_b, health_b), uid_b), body_b)| {
                                    (target, uid_b, pos_b, health_b, body_b)
                                })
                        });
                    target_iter.for_each(|(target, uid_b, pos_b, health_b, body_b)| {
                        // Check to see if entity has already been hit recently
                        if beam.hit_entities.iter().any(|&e| e == target) {
                            return;
                        }

                        // Scales
                        let scale_b = read_data.scales.get(target).map_or(1.0, |s| s.0);
                        let rad_b = body_b.max_radius() * scale_b;
                        let height_b = body_b.height() * scale_b;

                        // Check if it is a hit
                        // TODO: use Capsule Prism instead of cylinder
                        let hit = entity != target
                            && !health_b.is_dead
                            && conical_bezier_cylinder_collision(
                                beam.bezier,
                                beam.end_radius,
                                beam.range,
                                pos_b.0,
                                rad_b,
                                height_b,
                            );

                        // Finally, ensure that a hit has actually occurred by performing a raycast.
                        // We do this last because it's likely to be the
                        // most expensive operation.
                        let tgt_dist = pos.0.distance(pos_b.0);
                        let beam_dir = (beam.bezier.ctrl - beam.bezier.start)
                            / beam.bezier.start.distance(beam.bezier.ctrl).max(0.01);
                        let hit = hit
                            && read_data
                                .terrain
                                .ray(
                                    beam.bezier.start,
                                    beam.bezier.start + beam_dir * (tgt_dist + 1.0),
                                )
                                .until(|b| b.is_filled())
                                .cast()
                                .0
                                >= tgt_dist;

                        if hit {
                            // See if entities are in the same group
                            let same_group = group
                                .map(|group_a| Some(group_a) == read_data.groups.get(target))
                                .unwrap_or(false);

                            let target_group = if same_group {
                                GroupTarget::InGroup
                            } else {
                                GroupTarget::OutOfGroup
                            };

                            let attacker_info = Some(AttackerInfo {
                                entity,
                                uid: *uid,
                                group: read_data.groups.get(entity),
                                energy: read_data.energies.get(entity),
                                combo: read_data.combos.get(entity),
                                inventory: read_data.inventories.get(entity),
                                stats: read_data.stats.get(entity),
                            });

                            let target_info = TargetInfo {
                                entity: target,
                                uid: *uid_b,
                                inventory: read_data.inventories.get(target),
                                stats: read_data.stats.get(target),
                                health: read_data.healths.get(target),
                                pos: pos_b.0,
                                ori: read_data.orientations.get(target),
                                char_state: read_data.character_states.get(target),
                                energy: read_data.energies.get(target),
                                buffs: read_data.buffs.get(target),
                            };

                            let target_dodging = read_data
                                .character_states
                                .get(target)
                                .and_then(|cs| cs.attack_immunities())
                                .map_or(false, |i| i.beams);
                            // PvP check
                            let may_harm = combat::may_harm(
                                &read_data.alignments,
                                &read_data.players,
                                &read_data.id_maps,
                                Some(entity),
                                target,
                            );

                            let precision_from_flank = combat::precision_mult_from_flank(
                                beam.bezier.ctrl - beam.bezier.start,
                                target_info.ori,
                            );

                            let precision_from_time = {
                                if let Some(ticks) = beam.hit_durations.get(&target) {
                                    let dur = *ticks as f32 * beam.tick_dur.0 as f32;
                                    let mult =
                                        (dur / combat::BEAM_DURATION_PRECISION).clamp(0.0, 1.0);
                                    Some(combat::MAX_BEAM_DUR_PRECISION * mult)
                                } else {
                                    None
                                }
                            };

                            let precision_mult = match (precision_from_flank, precision_from_time) {
                                (Some(a), Some(b)) => Some(a.max(b)),
                                (Some(a), None) | (None, Some(a)) => Some(a),
                                (None, None) => None,
                            };

                            let attack_options = AttackOptions {
                                target_dodging,
                                may_harm,
                                target_group,
                                precision_mult,
                            };

                            beam.attack.apply_attack(
                                attacker_info,
                                &target_info,
                                ori.look_dir(),
                                attack_options,
                                1.0,
                                AttackSource::Beam,
                                *read_data.time,
                                &mut emitters,
                                |o| outcomes.push(o),
                                &mut rng,
                                0,
                            );

                            add_hit_entities.push((entity, target));
                        }
                    });
                    (emitters, add_hit_entities, outcomes)
                },
            )
            .reduce(
                || (read_data.events.get_emitters(), Vec::new(), Vec::new()),
                |(mut events_a, mut hit_entities_a, mut outcomes_a),
                 (events_b, mut hit_entities_b, mut outcomes_b)| {
                    events_a.append(events_b);
                    hit_entities_a.append(&mut hit_entities_b);
                    outcomes_a.append(&mut outcomes_b);
                    (events_a, hit_entities_a, outcomes_a)
                },
            );
        job.cpu_stats.measure(ParMode::Single);

        outcomes_emitter.emit_many(new_outcomes);

        for (entity, hit_entity) in add_hit_entities {
            if let Some(ref mut beam) = beams.get_mut(entity) {
                beam.hit_entities.push(hit_entity);
            }
        }
    }
}

/// Assumes upright cylinder
fn conical_bezier_cylinder_collision(
    // Values for spherical wedge
    bezier: QuadraticBezier3<f32>,
    max_rad: f32, // Radius at end_pos (radius is 0 at start_pos)
    range: f32,   // Used to decide number of steps in bezier function
    // Values for cylinder
    bottom_pos_b: Vec3<f32>, // Position of bottom of cylinder
    rad_b: f32,
    length_b: f32,
) -> bool {
    // This algorithm first determines the nearest point on the bezier to the point
    // in the middle of the cylinder. It then checks that the bezier cone's radius
    // at this point could allow it to be in the z bounds of the cylinder and within
    // the cylinder's radius.
    let center_pos_b = bottom_pos_b.with_z(bottom_pos_b.z + length_b / 2.0);
    let (t, closest_pos) =
        bezier.binary_search_point_by_steps(center_pos_b, (range * 5.0) as u16, 0.1);
    let bezier_rad = t * max_rad;
    let z_check = {
        let dist = (closest_pos.z - center_pos_b.z).abs();
        dist < bezier_rad + length_b / 2.0
    };
    let rad_check = {
        let dist_sqrd = closest_pos.xy().distance_squared(center_pos_b.xy());
        dist_sqrd < (bezier_rad + rad_b).powi(2)
    };
    z_check && rad_check
}
