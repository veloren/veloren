use common::{
    comp::{
        aura::{AuraChange, AuraKey, AuraKind, AuraTarget},
        buff,
        group::Group,
        Auras, BuffKind, Buffs, CharacterState, Health, Pos,
    },
    event::{EventBus, ServerEvent},
    resources::DeltaTime,
    uid::UidAllocator,
};
use specs::{
    saveload::MarkerAllocator, shred::ResourceId, Entities, Join, Read, ReadStorage, System,
    SystemData, World, WriteStorage,
};
use std::time::Duration;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    dt: Read<'a, DeltaTime>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    uid_allocator: Read<'a, UidAllocator>,
    positions: ReadStorage<'a, Pos>,
    char_states: ReadStorage<'a, CharacterState>,
    healths: ReadStorage<'a, Health>,
    groups: ReadStorage<'a, Group>,
    buffs: ReadStorage<'a, Buffs>,
}

pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (ReadData<'a>, WriteStorage<'a, Auras>);

    fn run(&mut self, (read_data, mut auras): Self::SystemData) {
        let mut server_emitter = read_data.server_bus.emitter();
        let dt = read_data.dt.0;

        auras.set_event_emission(false);

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
                for (target, target_pos, target_buffs, health) in (
                    &read_data.entities,
                    &read_data.positions,
                    &read_data.buffs,
                    &read_data.healths,
                )
                    .join()
                {
                    // Ensure entity is within the aura radius
                    if target_pos.0.distance_squared(pos.0) < aura.radius.powi(2) {
                        if let AuraTarget::GroupOf(uid) = aura.target {
                            let same_group = read_data
                                .uid_allocator
                                .retrieve_entity_internal(uid.into())
                                .and_then(|e| read_data.groups.get(e))
                                .map_or(false, |owner_group| {
                                    Some(owner_group) == read_data.groups.get(target)
                                });

                            if !same_group {
                                continue;
                            }
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
                                // Checks if the buff is not active so it isn't applied
                                // every tick, but rather only once it runs out
                                // TODO: Check for stronger buff of same kind so it can replace
                                // active buff.
                                if !target_buffs.contains(kind) {
                                    // Conditions for different buffs are in this match
                                    // statement
                                    let apply_buff = match kind {
                                        BuffKind::CampfireHeal => {
                                            matches!(
                                                read_data.char_states.get(target),
                                                Some(CharacterState::Sit)
                                            ) && health.current() < health.maximum()
                                        },
                                        // Add other specific buff conditions here
                                        _ => true,
                                    };
                                    if apply_buff {
                                        use buff::*;
                                        server_emitter.emit(ServerEvent::Buff {
                                            entity: target,
                                            buff_change: BuffChange::Add(Buff::new(
                                                kind,
                                                data,
                                                vec![category],
                                                source,
                                            )),
                                        });
                                    }
                                }
                            },
                        }
                    }
                }
            }
            if !expired_auras.is_empty() {
                server_emitter.emit(ServerEvent::Aura {
                    entity,
                    aura_change: AuraChange::RemoveByKey(expired_auras),
                });
            }
        }
        auras.set_event_emission(true);
    }
}
