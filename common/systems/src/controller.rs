use common::{
    comp::{
        ability::Stance,
        agent::{Sound, SoundKind},
        Body, BuffChange, ControlEvent, Controller, Pos,
    },
    event::{EventBus, ServerEvent},
    uid::UidAllocator,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{
    saveload::{Marker, MarkerAllocator},
    shred::ResourceId,
    Entities, Join, Read, ReadStorage, SystemData, World, WriteStorage,
};
use vek::*;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    uid_allocator: Read<'a, UidAllocator>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    positions: ReadStorage<'a, Pos>,
    bodies: ReadStorage<'a, Body>,
}

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (ReadData<'a>, WriteStorage<'a, Controller>);

    const NAME: &'static str = "controller";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (read_data, mut controllers): Self::SystemData) {
        let mut server_emitter = read_data.server_bus.emitter();

        for (entity, controller) in (&read_data.entities, &mut controllers).join() {
            // Sanitize inputs to avoid clients sending bad data
            controller.inputs.sanitize();

            // Process other controller events
            for event in controller.events.drain(..) {
                match event {
                    ControlEvent::Mount(mountee_uid) => {
                        if let Some(mountee_entity) = read_data
                            .uid_allocator
                            .retrieve_entity_internal(mountee_uid.id())
                        {
                            server_emitter.emit(ServerEvent::Mount(entity, mountee_entity));
                        }
                    },
                    ControlEvent::RemoveBuff(buff_id) => {
                        server_emitter.emit(ServerEvent::Buff {
                            entity,
                            buff_change: BuffChange::RemoveFromController(buff_id),
                        });
                    },
                    ControlEvent::Unmount => server_emitter.emit(ServerEvent::Unmount(entity)),
                    ControlEvent::EnableLantern => {
                        server_emitter.emit(ServerEvent::EnableLantern(entity))
                    },
                    ControlEvent::DisableLantern => {
                        server_emitter.emit(ServerEvent::DisableLantern(entity))
                    },
                    ControlEvent::Interact(npc_uid) => {
                        if let Some(npc_entity) = read_data
                            .uid_allocator
                            .retrieve_entity_internal(npc_uid.id())
                        {
                            server_emitter.emit(ServerEvent::NpcInteract(entity, npc_entity));
                        }
                    },
                    ControlEvent::InitiateInvite(inviter_uid, kind) => {
                        server_emitter.emit(ServerEvent::InitiateInvite(entity, inviter_uid, kind));
                    },
                    ControlEvent::InviteResponse(response) => {
                        server_emitter.emit(ServerEvent::InviteResponse(entity, response));
                    },
                    ControlEvent::PerformTradeAction(trade_id, action) => {
                        server_emitter
                            .emit(ServerEvent::ProcessTradeAction(entity, trade_id, action));
                    },
                    ControlEvent::InventoryEvent(event) => {
                        server_emitter.emit(ServerEvent::InventoryManip(entity, event.into()));
                    },
                    ControlEvent::GroupManip(manip) => {
                        server_emitter.emit(ServerEvent::GroupManip(entity, manip))
                    },
                    ControlEvent::Respawn => server_emitter.emit(ServerEvent::Respawn(entity)),
                    ControlEvent::Utterance(kind) => {
                        if let (Some(pos), Some(body)) = (
                            read_data.positions.get(entity),
                            read_data.bodies.get(entity),
                        ) {
                            let sound = Sound::new(
                                SoundKind::Utterance(kind, *body),
                                pos.0 + Vec3::unit_z() * body.eye_height(),
                                8.0, // TODO: Come up with a better way of determining this
                                1.0,
                            );
                            server_emitter.emit(ServerEvent::Sound { sound });
                        }
                    },
                    ControlEvent::ChangeAbility {
                        slot,
                        auxiliary_key,
                        new_ability,
                    } => {
                        server_emitter.emit(ServerEvent::ChangeAbility {
                            entity,
                            slot,
                            auxiliary_key,
                            new_ability,
                        });
                    },
                    ControlEvent::LeaveStance => {
                        server_emitter.emit(ServerEvent::ChangeStance {
                            entity,
                            stance: Stance::None,
                        });
                    },
                }
            }
        }
    }
}
