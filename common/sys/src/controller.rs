use common::{
    comp::{
        slot::{EquipSlot, Slot},
        BuffChange, CharacterState, ControlEvent, Controller, InventoryManip,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    metrics::SysMetrics,
    resources::DeltaTime,
    span,
    uid::{Uid, UidAllocator},
};
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage,
};
use vek::*;

// const CHARGE_COST: i32 = 200;
// const ROLL_COST: i32 = 30;

pub struct Sys;

impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, UidAllocator>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, EventBus<LocalEvent>>,
        Read<'a, DeltaTime>,
        ReadExpect<'a, SysMetrics>,
        WriteStorage<'a, Controller>,
        WriteStorage<'a, CharacterState>,
        ReadStorage<'a, Uid>,
    );

    fn run(
        &mut self,
        (
            entities,
            uid_allocator,
            server_bus,
            _local_bus,
            _dt,
            sys_metrics,
            mut controllers,
            mut character_states,
            uids,
        ): Self::SystemData,
    ) {
        let start_time = std::time::Instant::now();
        span!(_guard, "run", "controller::Sys::run");
        let mut server_emitter = server_bus.emitter();

        for (entity, _uid, controller, character_state) in
            (&entities, &uids, &mut controllers, &mut character_states).join()
        {
            let mut inputs = &mut controller.inputs;

            // Note(imbris): I avoided incrementing the duration with inputs.tick() because
            // this is being done manually in voxygen right now so it would double up on
            // speed of time.
            // Perhaphs the duration aspects of inputs could be
            // calculated exclusively on the server (since the client can't be
            // trusted anyway). It needs to be considered if these calculations
            // being on the client are critical for responsiveness/client-side prediction.
            inputs.tick_freshness();

            // Update `inputs.move_dir`.
            inputs.move_dir = if inputs.move_dir.magnitude_squared() > 1.0 {
                // Cap move_dir to 1
                inputs.move_dir.normalized()
            } else {
                inputs.move_dir
            };
            inputs.move_z = inputs.move_z.clamped(-1.0, 1.0);

            // Process other controller events
            for event in controller.events.drain(..) {
                match event {
                    ControlEvent::Mount(mountee_uid) => {
                        if let Some(mountee_entity) =
                            uid_allocator.retrieve_entity_internal(mountee_uid.id())
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
                    ControlEvent::InventoryManip(manip) => {
                        // Unwield if a wielded equipment slot is being modified, to avoid entering
                        // a barehanded wielding state.
                        if character_state.is_wield() {
                            match manip {
                                InventoryManip::Drop(Slot::Equip(EquipSlot::Mainhand))
                                | InventoryManip::Swap(_, Slot::Equip(EquipSlot::Mainhand))
                                | InventoryManip::Swap(Slot::Equip(EquipSlot::Mainhand), _) => {
                                    *character_state = CharacterState::Idle;
                                },
                                _ => (),
                            }
                        }
                        server_emitter.emit(ServerEvent::InventoryManip(entity, manip))
                    },
                    ControlEvent::GroupManip(manip) => {
                        server_emitter.emit(ServerEvent::GroupManip(entity, manip))
                    },
                    ControlEvent::Respawn => server_emitter.emit(ServerEvent::Respawn(entity)),
                }
            }
        }
        sys_metrics.controller_ns.store(
            start_time.elapsed().as_nanos() as i64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}
