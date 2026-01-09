use common::{
    GroupTarget,
    combat::{self, AttackOptions, AttackSource, AttackerInfo, TargetInfo},
    comp::{
        Alignment, Arcing, Body, Buffs, CharacterState, Combo, Energy, Group, Health, Inventory,
        Mass, Ori, Player, Pos, Scale, Stats, aura::EnteredAuras,
    },
    event::{
        BuffEvent, ComboChangeEvent, DeleteEvent, EmitExt, EnergyChangeEvent,
        EntityAttackedHookEvent, EventBus, HealthChangeEvent, KnockbackEvent, ParryHookEvent,
        PoiseChangeEvent, TransformEvent,
    },
    event_emitters,
    outcome::Outcome,
    resources::Time,
    uid::{IdMaps, Uid},
    util::Dir,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Entities, Join, LendJoin, Read, ReadStorage, SystemData, WriteStorage, shred};

event_emitters! {
    struct Events[Emitters] {
        delete: DeleteEvent,
        health_change: HealthChangeEvent,
        energy_change: EnergyChangeEvent,
        parry_hook: ParryHookEvent,
        knockback: KnockbackEvent,
        buff: BuffEvent,
        poise_change: PoiseChangeEvent,
        combo_change: ComboChangeEvent,
        entity_attack_hook: EntityAttackedHookEvent,
        transform: TransformEvent,
    }
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    events: Events<'a>,
    time: Read<'a, Time>,
    id_maps: Read<'a, IdMaps>,
    groups: ReadStorage<'a, Group>,
    uids: ReadStorage<'a, Uid>,
    scales: ReadStorage<'a, Scale>,
    entered_auras: ReadStorage<'a, EnteredAuras>,
    healths: ReadStorage<'a, Health>,
    bodies: ReadStorage<'a, Body>,
    energies: ReadStorage<'a, Energy>,
    combos: ReadStorage<'a, Combo>,
    inventories: ReadStorage<'a, Inventory>,
    stats: ReadStorage<'a, Stats>,
    masses: ReadStorage<'a, Mass>,
    orientations: ReadStorage<'a, Ori>,
    character_states: ReadStorage<'a, CharacterState>,
    buffs: ReadStorage<'a, Buffs>,
    alignments: ReadStorage<'a, Alignment>,
    players: ReadStorage<'a, Player>,
}

/// This system is responsible for hit detection of arcing attacks. Arcing
/// attacks chain between nearby entities.
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Arcing>,
        WriteStorage<'a, Pos>,
        Read<'a, EventBus<Outcome>>,
    );

    const NAME: &'static str = "arc";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (read_data, mut arcs, mut positions, outcomes): Self::SystemData) {
        let mut emitters = read_data.events.get_emitters();
        let mut outcomes_emitter = outcomes.emitter();
        let mut rng = rand::rng();

        (&read_data.entities, &mut arcs)
            .lend_join()
            .for_each(|(entity, mut arc)| {
                // Delete arc entity if it should expire
                if (read_data.time.0 > arc.last_arc_time.0 + arc.properties.max_delay.0)
                    || ((arc.hit_entities.len() > arc.properties.arcs as usize)
                        && (read_data.time.0 > arc.last_arc_time.0 + arc.properties.min_delay.0))
                {
                    emitters.emit(DeleteEvent(entity));
                    return;
                }

                let last_target = arc
                    .hit_entities
                    .last()
                    .and_then(|uid| read_data.id_maps.uid_entity(*uid));

                // Update arc entity position to position of last hit target entity
                let arc_pos =
                    if let Some(tgt_pos) = last_target.and_then(|e| positions.get(e)).copied() {
                        if let Some(pos) = positions.get_mut(entity) {
                            *pos = tgt_pos;
                            tgt_pos
                        } else {
                            return;
                        }
                    } else {
                        return;
                    };

                // Skip hit detection if not yet min delay
                if read_data.time.0 < arc.last_arc_time.0 + arc.properties.min_delay.0 {
                    return;
                }

                let arc_owner = arc.owner.and_then(|uid| read_data.id_maps.uid_entity(uid));

                let arc_group = arc_owner.and_then(|e| read_data.groups.get(e));

                for (target, uid_b, pos_b, health_b, body_b) in (
                    &read_data.entities,
                    &read_data.uids,
                    &positions,
                    &read_data.healths,
                    &read_data.bodies,
                )
                    .join()
                {
                    // Check to see if entity has already been hit
                    if arc.hit_entities.contains(uid_b) {
                        continue;
                    }

                    // TODO: use Capsule Prism instead of Cylinder
                    let (arc_rad, arc_height) = if let Some(lt) = last_target {
                        let body = read_data.bodies.get(lt);
                        let scale = read_data.scales.get(lt).map_or(1.0, |s| s.0);
                        (
                            body.map_or(0.0, |b| b.max_radius() * scale),
                            body.map(|b| b.height() * scale),
                        )
                    } else {
                        (0.0, None)
                    };

                    let scale_b = read_data.scales.get(target).map_or(1.0, |s| s.0);
                    let rad_b = body_b.max_radius() * scale_b;

                    // If the z ranges of each cylinder overlap, there is no z delta and the
                    // shortest path will be a horizontal line, otherwise the shortest path will go
                    // from the bottom of one range to the top of the other range
                    let z_delta = {
                        let pos_bzh = pos_b.0.z + body_b.height() * scale_b;
                        let tgt_range = pos_b.0.z..=pos_bzh;
                        if let Some(arc_height) = arc_height {
                            let arc_pos_zh = arc_pos.0.z + arc_height;
                            let arc_range = arc_pos.0.z..=arc_pos_zh;
                            if tgt_range.contains(&arc_pos.0.z)
                                || tgt_range.contains(&arc_pos_zh)
                                || arc_range.contains(&pos_b.0.z)
                                || arc_range.contains(&pos_bzh)
                            {
                                0.0
                            } else {
                                (arc_pos.0.z - pos_bzh)
                                    .abs()
                                    .min((pos_b.0.z - arc_pos_zh).abs())
                            }
                        } else if tgt_range.contains(&arc_pos.0.z) {
                            0.0
                        } else {
                            (pos_b.0.z - arc_pos.0.z)
                                .abs()
                                .min((pos_bzh - arc_pos.0.z).abs())
                        }
                    };

                    // See if entities are in the same group
                    let same_group = arc_group
                        .map(|group_a| Some(group_a) == read_data.groups.get(target))
                        .unwrap_or(Some(*uid_b) == arc.owner);

                    let is_owner = Some(*uid_b) == arc.owner;

                    let target_group = if same_group {
                        GroupTarget::InGroup
                    } else {
                        GroupTarget::OutOfGroup
                    };

                    let hit = entity != target
                        && !health_b.is_dead
                        && (!is_owner || arc.properties.targets_owner)
                        && arc_pos.0.distance_squared(pos_b.0) + z_delta.powi(2)
                            < (arc.properties.distance + arc_rad + rad_b).powi(2);

                    if hit {
                        let allow_friendly_fire = arc_owner.is_some_and(|entity| {
                            combat::allow_friendly_fire(&read_data.entered_auras, entity, target)
                        });
                        let dir = Dir::from_unnormalized(pos_b.0 - arc_pos.0).unwrap_or_default();

                        let attacker_info =
                            arc_owner.zip(arc.owner).map(|(entity, uid)| AttackerInfo {
                                entity,
                                uid,
                                group: read_data.groups.get(entity),
                                energy: read_data.energies.get(entity),
                                combo: read_data.combos.get(entity),
                                inventory: read_data.inventories.get(entity),
                                stats: read_data.stats.get(entity),
                                mass: read_data.masses.get(entity),
                                pos: Some(arc_pos.0),
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
                            mass: read_data.masses.get(target),
                            player: read_data.players.get(target),
                        };

                        let target_dodging = read_data
                            .character_states
                            .get(target)
                            .and_then(|cs| cs.roll_attack_immunities())
                            .is_some_and(|i| i.arcs);
                        // PvP check
                        let permit_pvp = combat::permit_pvp(
                            &read_data.alignments,
                            &read_data.players,
                            &read_data.entered_auras,
                            &read_data.id_maps,
                            arc_owner,
                            target,
                        );
                        // Arcs aren't precise, and thus cannot be a precise strike
                        let precision_mult = None;
                        let attack_options = AttackOptions {
                            target_dodging,
                            permit_pvp,
                            allow_friendly_fire,
                            target_group,
                            precision_mult,
                        };

                        arc.properties.attack.apply_attack(
                            attacker_info,
                            &target_info,
                            dir,
                            attack_options,
                            1.0,
                            AttackSource::Arc,
                            *read_data.time,
                            &mut emitters,
                            |o| outcomes_emitter.emit(o),
                            &mut rng,
                            0,
                        );

                        // Once we hit the first entity, break the loop since we only arc to a
                        // single entity
                        arc.hit_entities.push(*uid_b);
                        arc.last_arc_time = *read_data.time;
                        break;
                    }
                }
            })
    }
}
