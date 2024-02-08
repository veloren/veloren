use common::{
    comp::{
        ability::Stance,
        agent::{Sound, SoundKind},
        Body, BuffChange, Collider, ControlEvent, Controller, Pos, Scale,
    },
    event::{self, EmitExt},
    event_emitters,
    terrain::TerrainGrid,
    uid::IdMaps,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{shred, Entities, Join, Read, ReadExpect, ReadStorage, SystemData, WriteStorage};
use vek::*;

event_emitters! {
    struct Events[EventEmitters] {
        mount: event::MountEvent,
        mount_volume: event::MountVolumeEvent,
        set_pet_stay: event::SetPetStayEvent,
        unmount: event::UnmountEvent,
        lantern: event::SetLanternEvent,
        npc_interact: event::NpcInteractEvent,
        initiate_invite: event::InitiateInviteEvent,
        invite_response: event::InviteResponseEvent,
        process_trade_action: event::ProcessTradeActionEvent,
        inventory_manip: event::InventoryManipEvent,
        group_manip: event::GroupManipEvent,
        respawn: event::RespawnEvent,
        sound: event::SoundEvent,
        change_ability: event::ChangeAbilityEvent,
        change_stance: event::ChangeStanceEvent,
        start_teleporting: event::StartTeleportingEvent,
        buff: event::BuffEvent,
    }
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    id_maps: Read<'a, IdMaps>,
    events: Events<'a>,
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
        let mut emitters = read_data.events.get_emitters();

        for (entity, controller) in (&read_data.entities, &mut controllers).join() {
            // Sanitize inputs to avoid clients sending bad data
            controller.inputs.sanitize();

            // Process other controller events
            for event in controller.events.drain(..) {
                match event {
                    ControlEvent::Mount(mountee_uid) => {
                        if let Some(mountee_entity) = read_data.id_maps.uid_entity(mountee_uid) {
                            emitters.emit(event::MountEvent(entity, mountee_entity));
                        }
                    },
                    ControlEvent::MountVolume(volume) => {
                        if let Some(block) = volume.get_block(
                            &read_data.terrain_grid,
                            &read_data.id_maps,
                            &read_data.colliders,
                        ) {
                            if block.is_mountable() {
                                emitters.emit(event::MountVolumeEvent(entity, volume));
                            }
                        }
                    },
                    ControlEvent::SetPetStay(pet_uid, stay) => {
                        if let Some(pet_entity) = read_data.id_maps.uid_entity(pet_uid) {
                            emitters.emit(event::SetPetStayEvent(entity, pet_entity, stay));
                        }
                    },
                    ControlEvent::RemoveBuff(buff_id) => {
                        emitters.emit(event::BuffEvent {
                            entity,
                            buff_change: BuffChange::RemoveFromController(buff_id),
                        });
                    },
                    ControlEvent::Unmount => emitters.emit(event::UnmountEvent(entity)),
                    ControlEvent::EnableLantern => {
                        emitters.emit(event::SetLanternEvent(entity, true))
                    },
                    ControlEvent::DisableLantern => {
                        emitters.emit(event::SetLanternEvent(entity, false))
                    },
                    ControlEvent::Interact(npc_uid, subject) => {
                        if let Some(npc_entity) = read_data.id_maps.uid_entity(npc_uid) {
                            emitters.emit(event::NpcInteractEvent(entity, npc_entity, subject));
                        }
                    },
                    ControlEvent::InitiateInvite(inviter_uid, kind) => {
                        emitters.emit(event::InitiateInviteEvent(entity, inviter_uid, kind));
                    },
                    ControlEvent::InviteResponse(response) => {
                        emitters.emit(event::InviteResponseEvent(entity, response));
                    },
                    ControlEvent::PerformTradeAction(trade_id, action) => {
                        emitters.emit(event::ProcessTradeActionEvent(entity, trade_id, action));
                    },
                    ControlEvent::InventoryEvent(event) => {
                        emitters.emit(event::InventoryManipEvent(entity, event.into()));
                    },
                    ControlEvent::GroupManip(manip) => {
                        emitters.emit(event::GroupManipEvent(entity, manip))
                    },
                    ControlEvent::Respawn => emitters.emit(event::RespawnEvent(entity)),
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
                            emitters.emit(event::SoundEvent { sound });
                        }
                    },
                    ControlEvent::ChangeAbility {
                        slot,
                        auxiliary_key,
                        new_ability,
                    } => {
                        emitters.emit(event::ChangeAbilityEvent {
                            entity,
                            slot,
                            auxiliary_key,
                            new_ability,
                        });
                    },
                    ControlEvent::LeaveStance => {
                        emitters.emit(event::ChangeStanceEvent {
                            entity,
                            stance: Stance::None,
                        });
                    },
                    ControlEvent::ActivatePortal(portal_uid) => {
                        if let Some(portal) = read_data.id_maps.uid_entity(portal_uid) {
                            emitters.emit(event::StartTeleportingEvent { entity, portal });
                        }
                    },
                }
            }
        }
    }
}
