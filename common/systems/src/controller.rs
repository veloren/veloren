use common::{
    comp::{
        ability::Stance,
        agent::{Sound, SoundKind},
        Body, BuffChange, Collider, ControlEvent, Controller, Pos, Scale,
    },
    event::{EventBus, ServerEvent},
    terrain::TerrainGrid,
    uid::IdMaps,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{shred, Entities, Join, Read, ReadExpect, ReadStorage, SystemData, WriteStorage};
use vek::*;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    id_maps: Read<'a, IdMaps>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    terrain_grid: ReadExpect<'a, TerrainGrid>,
    positions: ReadStorage<'a, Pos>,
    bodies: ReadStorage<'a, Body>,
    scales: ReadStorage<'a, Scale>,
    colliders: ReadStorage<'a, Collider>,
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
                        if let Some(mountee_entity) = read_data.id_maps.uid_entity(mountee_uid) {
                            server_emitter.emit(ServerEvent::Mount(entity, mountee_entity));
                        }
                    },
                    ControlEvent::MountVolume(volume) => {
                        if let Some(block) = volume.get_block(
                            &read_data.terrain_grid,
                            &read_data.id_maps,
                            &read_data.colliders,
                        ) {
                            if block.is_mountable() {
                                server_emitter.emit(ServerEvent::MountVolume(entity, volume));
                            }
                        }
                    },
                    ControlEvent::SetPetStay(pet_uid, stay) => {
                        if let Some(pet_entity) = read_data.id_maps.uid_entity(pet_uid) {
                            server_emitter.emit(ServerEvent::SetPetStay(entity, pet_entity, stay));
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
                    ControlEvent::Interact(npc_uid, subject) => {
                        if let Some(npc_entity) = read_data.id_maps.uid_entity(npc_uid) {
                            server_emitter
                                .emit(ServerEvent::NpcInteract(entity, npc_entity, subject));
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
                        if let (Some(pos), Some(body), scale) = (
                            read_data.positions.get(entity),
                            read_data.bodies.get(entity),
                            read_data.scales.get(entity),
                        ) {
                            let sound = Sound::new(
                                SoundKind::Utterance(kind, *body),
                                pos.0
                                    + Vec3::unit_z() * body.eye_height(scale.map_or(1.0, |s| s.0)),
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
                    ControlEvent::ActivatePortal(portal_uid) => {
                        if let Some(portal) = read_data.id_maps.uid_entity(portal_uid) {
                            server_emitter.emit(ServerEvent::StartTeleporting { entity, portal });
                        }
                    },
                }
            }
        }
    }
}
