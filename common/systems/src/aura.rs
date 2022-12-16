use common::{
    combat,
    comp::{
        aura::{AuraChange, AuraKey, AuraKind, AuraTarget},
        buff::{Buff, BuffCategory, BuffChange, BuffSource},
        group::Group,
        Alignment, Aura, Auras, BuffKind, Buffs, CharacterState, Health, Player, Pos, Stats,
    },
    event::{Emitter, EventBus, ServerEvent},
    resources::Time,
    uid::{Uid, UidAllocator},
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{
    saveload::MarkerAllocator, shred::ResourceId, Entities, Entity as EcsEntity, Join, Read,
    ReadStorage, SystemData, World,
};

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    players: ReadStorage<'a, Player>,
    time: Read<'a, Time>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    uid_allocator: Read<'a, UidAllocator>,
    cached_spatial_grid: Read<'a, common::CachedSpatialGrid>,
    positions: ReadStorage<'a, Pos>,
    char_states: ReadStorage<'a, CharacterState>,
    alignments: ReadStorage<'a, Alignment>,
    healths: ReadStorage<'a, Health>,
    groups: ReadStorage<'a, Group>,
    uids: ReadStorage<'a, Uid>,
    stats: ReadStorage<'a, Stats>,
    buffs: ReadStorage<'a, Buffs>,
    auras: ReadStorage<'a, Auras>,
}

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = ReadData<'a>;

    const NAME: &'static str = "aura";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, read_data: Self::SystemData) {
        let mut server_emitter = read_data.server_bus.emitter();

        // Iterate through all entities with an aura
        for (entity, pos, auras_comp, uid) in (
            &read_data.entities,
            &read_data.positions,
            &read_data.auras,
            &read_data.uids,
        )
            .join()
        {
            let mut expired_auras = Vec::<AuraKey>::new();
            // Iterate through the auras attached to this entity
            for (key, aura) in auras_comp.auras.iter() {
                // Tick the aura and subtract dt from it
                if let Some(end_time) = aura.end_time {
                    if read_data.time.0 > end_time.0 {
                        expired_auras.push(key);
                    }
                }
                let target_iter = read_data
                    .cached_spatial_grid
                    .0
                    .in_circle_aabr(pos.0.xy(), aura.radius)
                    .filter_map(|target| {
                        read_data
                            .positions
                            .get(target)
                            .and_then(|l| read_data.healths.get(target).map(|r| (l, r)))
                            .and_then(|l| read_data.uids.get(target).map(|r| (l, r)))
                            .map(|((target_pos, health), target_uid)| {
                                (
                                    target,
                                    target_pos,
                                    health,
                                    target_uid,
                                    read_data.stats.get(target),
                                )
                            })
                    });
                target_iter.for_each(|(target, target_pos, health, target_uid, stats)| {
                    let target_buffs = match read_data.buffs.get(target) {
                        Some(buff) => buff,
                        None => return,
                    };

                    // Ensure entity is within the aura radius
                    if target_pos.0.distance_squared(pos.0) < aura.radius.powi(2) {
                        // Ensure the entity is in the group we want to target
                        let same_group = |uid: Uid| {
                            read_data
                                .uid_allocator
                                .retrieve_entity_internal(uid.into())
                                .and_then(|e| read_data.groups.get(e))
                                .map_or(false, |owner_group| {
                                    Some(owner_group) == read_data.groups.get(target)
                                })
                                || *target_uid == uid
                        };

                        let is_target = match aura.target {
                            AuraTarget::GroupOf(uid) => same_group(uid),
                            AuraTarget::NotGroupOf(uid) => !same_group(uid),
                            AuraTarget::All => true,
                        };

                        if is_target {
                            activate_aura(
                                key,
                                aura,
                                *uid,
                                target,
                                health,
                                target_buffs,
                                stats,
                                &read_data,
                                &mut server_emitter,
                            );
                        }
                    }
                });
            }
            if !expired_auras.is_empty() {
                server_emitter.emit(ServerEvent::Aura {
                    entity,
                    aura_change: AuraChange::RemoveByKey(expired_auras),
                });
            }
        }
    }
}

#[warn(clippy::pedantic)]
//#[warn(clippy::nursery)]
fn activate_aura(
    key: AuraKey,
    aura: &Aura,
    applier: Uid,
    target: EcsEntity,
    health: &Health,
    target_buffs: &Buffs,
    stats: Option<&Stats>,
    read_data: &ReadData,
    server_emitter: &mut Emitter<ServerEvent>,
) {
    let should_activate = match aura.aura_kind {
        AuraKind::Buff { kind, source, .. } => {
            let conditions_held = match kind {
                BuffKind::CampfireHeal => {
                    // true if sitting or if owned and owner is sitting + not full health
                    health.current() < health.maximum()
                        && (read_data
                            .char_states
                            .get(target)
                            .map_or(false, CharacterState::is_sitting)
                            || read_data
                                .alignments
                                .get(target)
                                .and_then(|alignment| match alignment {
                                    Alignment::Owned(uid) => Some(uid),
                                    _ => None,
                                })
                                .and_then(|uid| {
                                    read_data
                                        .uid_allocator
                                        .retrieve_entity_internal((*uid).into())
                                })
                                .and_then(|owner| read_data.char_states.get(owner))
                                .map_or(false, CharacterState::is_sitting))
                },
                // Add other specific buff conditions here
                _ => true,
            };

            // TODO: this check will disable friendly fire with PvE switch.
            //
            // Which means that you can't apply debuffs on you and your group
            // even if it's intended mechanic.
            //
            // We don't have this for now, but think about this
            // when we will add this.
            let may_harm = || {
                let owner = match source {
                    BuffSource::Character { by } => {
                        read_data.uid_allocator.retrieve_entity_internal(by.into())
                    },
                    _ => None,
                };
                combat::may_harm(
                    &read_data.alignments,
                    &read_data.players,
                    &read_data.uid_allocator,
                    owner,
                    target,
                )
            };

            conditions_held && (kind.is_buff() || may_harm())
        },
    };

    if !should_activate {
        return;
    }

    // TODO: When more aura kinds (besides Buff) are
    // implemented, match on them here
    match aura.aura_kind {
        AuraKind::Buff {
            kind,
            data,
            category,
            source,
        } => {
            // Checks that target is not already receiving a buff
            // from an aura, where the buff is of the same kind,
            // and is of at least the same strength
            // and of at least the same duration.
            // If no such buff is present, adds the buff.
            let emit_buff = !target_buffs.buffs.iter().any(|(_, buff)| {
                buff.cat_ids
                    .iter()
                    .any(|cat_id| matches!(cat_id, BuffCategory::FromActiveAura(uid, aura_key) if *aura_key == key && *uid == applier))
                    && buff.kind == kind
                    && buff.data.strength >= data.strength
            });
            if emit_buff {
                server_emitter.emit(ServerEvent::Buff {
                    entity: target,
                    buff_change: BuffChange::Add(Buff::new(
                        kind,
                        data,
                        vec![category, BuffCategory::FromActiveAura(applier, key)],
                        source,
                        *read_data.time,
                        stats,
                        Some(health),
                    )),
                });
            }
        },
    }
}
