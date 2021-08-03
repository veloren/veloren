use common::{
    combat,
    comp::{
        aura::{AuraChange, AuraKey, AuraKind, AuraTarget},
        buff::{Buff, BuffCategory, BuffChange, BuffSource},
        group::Group,
        Aura, Auras, BuffKind, Buffs, CharacterState, Health, Player, Pos,
    },
    event::{Emitter, EventBus, ServerEvent},
    resources::DeltaTime,
    uid::{Uid, UidAllocator},
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{
    saveload::MarkerAllocator, shred::ResourceId, Entities, Entity as EcsEntity, Join, Read,
    ReadStorage, SystemData, World, WriteStorage,
};
use std::time::Duration;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    players: ReadStorage<'a, Player>,
    dt: Read<'a, DeltaTime>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    uid_allocator: Read<'a, UidAllocator>,
    cached_spatial_grid: Read<'a, common::CachedSpatialGrid>,
    positions: ReadStorage<'a, Pos>,
    char_states: ReadStorage<'a, CharacterState>,
    healths: ReadStorage<'a, Health>,
    groups: ReadStorage<'a, Group>,
    uids: ReadStorage<'a, Uid>,
}

#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Auras>,
        WriteStorage<'a, Buffs>,
    );

    const NAME: &'static str = "aura";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (read_data, mut auras, mut buffs): Self::SystemData) {
        let mut server_emitter = read_data.server_bus.emitter();
        let dt = read_data.dt.0;

        auras.set_event_emission(false);
        buffs.set_event_emission(false);

        // Iterate through all buffs, on any buffs that are from an aura, sets the check
        // for whether the buff recently set by aura to false
        for (_, mut buffs_comp) in (&read_data.entities, &mut buffs).join() {
            for (_, buff) in buffs_comp.buffs.iter_mut() {
                if let Some(cat_id) = buff
                    .cat_ids
                    .iter_mut()
                    .find(|cat_id| matches!(cat_id, BuffCategory::FromAura(true)))
                {
                    *cat_id = BuffCategory::FromAura(false);
                }
            }
        }

        // Iterate through all entities with an aura
        for (entity, pos, mut auras_comp) in
            (&read_data.entities, &read_data.positions, &mut auras).join()
        {
            let mut expired_auras = Vec::<AuraKey>::new();
            // Iterate through the auras attached to this entity
            for (key, aura) in auras_comp.auras.iter_mut() {
                // Tick the aura and subtract dt from it
                if let Some(remaining_time) = &mut aura.duration {
                    if let Some(new_duration) =
                        remaining_time.checked_sub(Duration::from_secs_f32(dt))
                    {
                        *remaining_time = new_duration;
                    } else {
                        *remaining_time = Duration::default();
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
                                (target, target_pos, health, target_uid)
                            })
                    });
                target_iter.for_each(|(target, target_pos, health, target_uid)| {
                    let mut target_buffs = match buffs.get_mut(target) {
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
                                aura,
                                target,
                                health,
                                &mut target_buffs,
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
        auras.set_event_emission(true);
        buffs.set_event_emission(true);
    }
}

#[warn(clippy::pedantic)]
//#[warn(clippy::nursery)]
fn activate_aura(
    aura: &Aura,
    target: EcsEntity,
    health: &Health,
    target_buffs: &mut Buffs,
    read_data: &ReadData,
    server_emitter: &mut Emitter<ServerEvent>,
) {
    let should_activate = match aura.aura_kind {
        AuraKind::Buff { kind, source, .. } => {
            let conditions_held = match kind {
                BuffKind::CampfireHeal => {
                    let target_state = read_data.char_states.get(target);
                    matches!(target_state, Some(CharacterState::Sit))
                        && health.current() < health.maximum()
                },
                // Add other specific buff conditions here
                _ => true,
            };

            // TODO: this check will disable friendly fire with PvE switch.
            //
            // Which means that you can't apply debuffs on you and your group
            // even if it's intented mechanics.
            //
            // Not that we have this for now, but think about this
            // when we will add this.
            let may_harm = || {
                let owner = match source {
                    BuffSource::Character { by } => {
                        read_data.uid_allocator.retrieve_entity_internal(by.into())
                    },
                    _ => None,
                };
                owner.map_or(true, |attacker| {
                    let attacker = read_data.players.get(attacker);
                    let target = read_data.players.get(target);
                    combat::may_harm(attacker, target)
                })
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
                    .any(|cat_id| matches!(cat_id, BuffCategory::FromAura(_)))
                    && buff.kind == kind
                    && buff.data.strength >= data.strength
                    && buff.time.map_or(true, |dur| {
                        data.duration.map_or(false, |dur_2| dur >= dur_2)
                    })
            });
            if emit_buff {
                server_emitter.emit(ServerEvent::Buff {
                    entity: target,
                    buff_change: BuffChange::Add(Buff::new(
                        kind,
                        data,
                        vec![category, BuffCategory::FromAura(true)],
                        source,
                    )),
                });
            }
            // Finds all buffs on target that are from an aura, are of
            // the same buff kind, and are of at most the same strength.
            // For any such buffs, marks it as recently applied.
            for (_, buff) in target_buffs.buffs.iter_mut().filter(|(_, buff)| {
                buff.cat_ids
                    .iter()
                    .any(|cat_id| matches!(cat_id, BuffCategory::FromAura(_)))
                    && buff.kind == kind
                    && buff.data.strength <= data.strength
            }) {
                if let Some(cat_id) = buff
                    .cat_ids
                    .iter_mut()
                    .find(|cat_id| matches!(cat_id, BuffCategory::FromAura(false)))
                {
                    *cat_id = BuffCategory::FromAura(true);
                }
            }
        },
    }
}
