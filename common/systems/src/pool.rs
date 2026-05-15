use common::{
    GroupTarget,
    combat::{self, AttackOptions, AttackSource, AttackerInfo, TargetInfo},
    comp::{
        Alignment, Body, Buffs, CharacterState, Combo, Energy, Group, Health, Inventory, Mass, Ori,
        PhysicsState, Player, Pos, Scale, Stats, ability::Dodgeable, aura::EnteredAuras,
        pool::Pool,
    },
    event::{
        BuffEvent, ComboChangeEvent, DeleteEvent, EmitExt, EnergyChangeEvent,
        EntityAttackedHookEvent, EventBus, HealthChangeEvent, KnockbackEvent, ParryHookEvent,
        PoiseChangeEvent, TransformEvent,
    },
    event_emitters,
    outcome::Outcome,
    resources::Time,
    terrain::TerrainGrid,
    uid::{IdMaps, Uid},
    util::Dir,
    vol::ReadVol,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{
    Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, SystemData, WriteStorage, shred,
};
use vek::*;

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

    terrain: ReadExpect<'a, TerrainGrid>,
    id_maps: Read<'a, IdMaps>,
    groups: ReadStorage<'a, Group>,
    uids: ReadStorage<'a, Uid>,
    positions: ReadStorage<'a, Pos>,
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
    scales: ReadStorage<'a, Scale>,
    entered_auras: ReadStorage<'a, EnteredAuras>,
    outcomes: Read<'a, EventBus<Outcome>>,
    physics_states: ReadStorage<'a, PhysicsState>,
}

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (ReadData<'a>, WriteStorage<'a, Pool>);

    const NAME: &'static str = "pool";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (read_data, mut pools): Self::SystemData) {
        let mut emitters = read_data.events.get_emitters();
        let mut outcomes_emitter = read_data.outcomes.emitter();
        let mut rng = rand::rng();

        (&read_data.entities, &mut pools, &read_data.positions)
            .lend_join()
            .for_each(|(pool_entity, mut pool, pool_pos)| {
                // Expire the pool after its duration.
                if read_data.time.0 > pool.start_time.0 + pool.properties.duration.0 {
                    emitters.emit(DeleteEvent(pool_entity));
                    return;
                }

                // Only tick at the configured interval.
                if read_data.time.0 < pool.last_tick.0 + pool.properties.tick_dur.0 {
                    return;
                }
                pool.last_tick = *read_data.time;

                let pool_owner = pool.owner.and_then(|uid| read_data.id_maps.uid_entity(uid));
                let pool_group = pool_owner.and_then(|e| read_data.groups.get(e));

                for (target, uid_b, pos_b, health_b, body_b) in (
                    &read_data.entities,
                    &read_data.uids,
                    &read_data.positions,
                    &read_data.healths,
                    &read_data.bodies,
                )
                    .join()
                {
                    // Skip self and dead entities.
                    if pool_entity == target || health_b.is_dead {
                        continue;
                    }

                    let scale_b = read_data.scales.get(target).map_or(1.0, |s| s.0);
                    let rad_b = body_b.max_radius() * scale_b;

                    // Broad-phase distance check.
                    if pool_pos.0.distance_squared(pos_b.0)
                        > (pool.properties.radius + rad_b).powi(2)
                    {
                        continue;
                    }

                    // Line-of-sight check: cast a ray from the pool surface
                    // (slightly elevated) to the target entity centre.  If
                    // terrain fills the ray before we reach the target, skip.
                    let ray_origin = pool_pos.0 + Vec3::unit_z() * 0.5;
                    let tgt_dist = ray_origin.distance(pos_b.0);
                    let ray_dist = read_data
                        .terrain
                        .ray(ray_origin, pos_b.0)
                        .until(|b: &_| b.is_filled())
                        .cast()
                        .0;
                    if ray_dist < tgt_dist * 0.9 {
                        // Terrain occludes the target.
                        continue;
                    }

                    let same_group = pool_group
                        .map(|group_a| Some(group_a) == read_data.groups.get(target))
                        .unwrap_or(Some(*uid_b) == pool.owner);

                    let target_group = if same_group {
                        GroupTarget::InGroup
                    } else {
                        GroupTarget::OutOfGroup
                    };

                    let allow_friendly_fire = pool_owner.is_some_and(|owner_entity| {
                        combat::allow_friendly_fire(&read_data.entered_auras, owner_entity, target)
                    });

                    let dir = Dir::from_unnormalized(pos_b.0 - pool_pos.0).unwrap_or_default();

                    let attacker_info =
                        pool_owner
                            .zip(pool.owner)
                            .map(|(entity, uid)| AttackerInfo {
                                entity,
                                uid,
                                group: read_data.groups.get(entity),
                                energy: read_data.energies.get(entity),
                                combo: read_data.combos.get(entity),
                                inventory: read_data.inventories.get(entity),
                                stats: read_data.stats.get(entity),
                                mass: read_data.masses.get(entity),
                                pos: Some(pool_pos.0),
                            });

                    let target_info = TargetInfo {
                        entity: target,
                        uid: *uid_b,
                        inventory: read_data.inventories.get(target),
                        stats: read_data.stats.get(target),
                        health: Some(health_b),
                        pos: pos_b.0,
                        ori: read_data.orientations.get(target),
                        char_state: read_data.character_states.get(target),
                        energy: read_data.energies.get(target),
                        buffs: read_data.buffs.get(target),
                        mass: read_data.masses.get(target),
                        player: read_data.players.get(target),
                    };

                    //TODO: Consider making pool hardcoded jump dodgeable only (like ground
                    // shockwaves)
                    let target_dodging = match pool.properties.dodgeable {
                        Dodgeable::Roll => read_data
                            .character_states
                            .get(target)
                            .and_then(|cs| cs.roll_attack_immunities())
                            .is_some_and(|i| i.pools),
                        Dodgeable::Jump => read_data
                            .physics_states
                            .get(target)
                            .is_some_and(|ps| ps.on_ground.is_none()),
                        Dodgeable::No => false,
                    };

                    let permit_pvp = combat::permit_pvp(
                        &read_data.alignments,
                        &read_data.players,
                        &read_data.entered_auras,
                        &read_data.id_maps,
                        pool_owner,
                        target,
                    );

                    let attack_options = AttackOptions {
                        target_dodging,
                        permit_pvp,
                        allow_friendly_fire,
                        target_group,
                        precision_mult: None,
                    };

                    pool.properties.attack.apply_attack(
                        attacker_info,
                        &target_info,
                        dir,
                        attack_options,
                        1.0,
                        AttackSource::Pool,
                        *read_data.time,
                        &mut emitters,
                        |o| outcomes_emitter.emit(o),
                        &mut rng,
                        0,
                    );
                }
            });
    }
}
