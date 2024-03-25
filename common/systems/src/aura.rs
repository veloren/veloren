use std::collections::HashSet;

use common::{
    combat,
    comp::{
        aura::{AuraChange, AuraKey, AuraKind, AuraTarget, EnteredAuras},
        buff::{Buff, BuffCategory, BuffChange, BuffSource},
        group::Group,
        Alignment, Aura, Auras, BuffKind, Buffs, CharacterState, Health, Player, Pos, Stats,
    },
    event::{AuraEvent, BuffEvent, EmitExt},
    event_emitters,
    resources::Time,
    uid::{IdMaps, Uid},
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{shred, Entities, Entity as EcsEntity, Join, Read, ReadStorage, SystemData};

event_emitters! {
    struct Events[Emitters] {
        aura: AuraEvent,
        buff: BuffEvent,
    }
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    players: ReadStorage<'a, Player>,
    time: Read<'a, Time>,
    events: Events<'a>,
    id_maps: Read<'a, IdMaps>,
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
    entered_auras: ReadStorage<'a, EnteredAuras>,
}

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = ReadData<'a>;

    const NAME: &'static str = "aura";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, read_data: Self::SystemData) {
        let mut emitters = read_data.events.get_emitters();
        let mut active_auras: HashSet<(Uid, Uid, AuraKey)> = HashSet::new();

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
                        read_data.positions.get(target).and_then(|target_pos| {
                            Some((
                                target,
                                target_pos,
                                read_data.healths.get(target)?,
                                read_data.uids.get(target)?,
                                read_data.entered_auras.get(target)?,
                                read_data.stats.get(target),
                            ))
                        })
                    });
                target_iter.for_each(
                    |(target, target_pos, health, target_uid, entered_auras, stats)| {
                        let target_buffs = match read_data.buffs.get(target) {
                            Some(buff) => buff,
                            None => return,
                        };

                        // Ensure entity is within the aura radius
                        if target_pos.0.distance_squared(pos.0) < aura.radius.powi(2) {
                            // Ensure the entity is in the group we want to target
                            let same_group = |uid: Uid| {
                                read_data
                                    .id_maps
                                    .uid_entity(uid)
                                    .and_then(|e| read_data.groups.get(e))
                                    .map_or(false, |owner_group| {
                                        Some(owner_group) == read_data.groups.get(target)
                                    })
                                    || *target_uid == uid
                            };

                            let allow_friendly_fire = combat::allow_friendly_fire(
                                &read_data.entered_auras,
                                entity,
                                target,
                            );

                            if !(allow_friendly_fire && entity != target
                                || match aura.target {
                                    AuraTarget::GroupOf(uid) => same_group(uid),
                                    AuraTarget::NotGroupOf(uid) => !same_group(uid),
                                    AuraTarget::All => true,
                                })
                            {
                                return;
                            }

                            let did_activate = activate_aura(
                                key,
                                aura,
                                *uid,
                                target,
                                health,
                                target_buffs,
                                stats,
                                allow_friendly_fire,
                                &read_data,
                                &mut emitters,
                            );

                            if did_activate {
                                if entered_auras
                                    .auras
                                    .get(aura.aura_kind.as_ref())
                                    .map_or(true, |auras| !auras.contains(&(*uid, key)))
                                {
                                    emitters.emit(AuraEvent {
                                        entity: target,
                                        aura_change: AuraChange::EnterAura(
                                            *uid,
                                            key,
                                            *aura.aura_kind.as_ref(),
                                        ),
                                    });
                                }
                                active_auras.insert((*uid, *target_uid, key));
                            }
                        }
                    },
                );
            }
            if !expired_auras.is_empty() {
                emitters.emit(AuraEvent {
                    entity,
                    aura_change: AuraChange::RemoveByKey(expired_auras),
                });
            }
        }

        for (entity, entered_auras, uid) in (
            &read_data.entities,
            &read_data.entered_auras,
            &read_data.uids,
        )
            .join()
            .filter(|(_, active_auras, _)| !active_auras.auras.is_empty())
        {
            emitters.emit_many(
                entered_auras
                    .auras
                    .iter()
                    .flat_map(|(variant, entered_auras)| {
                        entered_auras.iter().zip(core::iter::repeat(*variant))
                    })
                    .filter_map(|((caster_uid, key), variant)| {
                        (!active_auras.contains(&(*caster_uid, *uid, *key))).then_some(AuraEvent {
                            entity,
                            aura_change: AuraChange::ExitAura(*caster_uid, *key, variant),
                        })
                    }),
            );
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
    allow_friendly_fire: bool,
    read_data: &ReadData,
    emitters: &mut impl EmitExt<BuffEvent>,
) -> bool {
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
                                .and_then(|uid| read_data.id_maps.uid_entity(*uid))
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
            let permit_pvp = || {
                let owner = match source {
                    BuffSource::Character { by } => read_data.id_maps.uid_entity(by),
                    _ => None,
                };
                combat::permit_pvp(
                    &read_data.alignments,
                    &read_data.players,
                    &read_data.entered_auras,
                    &read_data.id_maps,
                    owner,
                    target,
                )
            };

            conditions_held && (kind.is_buff() || allow_friendly_fire || permit_pvp())
        },
        AuraKind::FriendlyFire => true,
        AuraKind::ForcePvP => {
            // Only apply this aura to players
            read_data.players.contains(target)
        },
    };

    if !should_activate {
        return false;
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
                emitters.emit(BuffEvent {
                    entity: target,
                    buff_change: BuffChange::Add(Buff::new(
                        kind,
                        data,
                        vec![category, BuffCategory::FromActiveAura(applier, key)],
                        source,
                        *read_data.time,
                        stats,
                    )),
                });
            }
        },
        // No implementation needed for these auras
        AuraKind::FriendlyFire | AuraKind::ForcePvP => {},
    }

    true
}
