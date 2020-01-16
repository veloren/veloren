use crate::{
    comp::{ControlEvent, Controller},
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
    sync::{Uid, UidAllocator},
};
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entities, Join, Read, ReadStorage, System, WriteStorage,
};
use sphynx::{Uid, UidAllocator};

/// # Controller System
/// #### Responsible for validating and updating controller inputs
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, UidAllocator>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, EventBus<LocalEvent>>,
        Read<'a, DeltaTime>,
        WriteStorage<'a, Controller>,
        ReadStorage<'a, Uid>,
    );
    fn run(
        &mut self,
        (entities, uid_allocator, server_bus, _local_bus, _dt, mut controllers, uids): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();
        for (entity, _uid, controller) in (&entities, &uids, &mut controllers).join() {
            let inputs = &mut controller.inputs;

            // Update `inputs.move_dir`.
            inputs.move_dir = if inputs.move_dir.magnitude_squared() > 1.0 {
                // Cap move_dir to 1
                inputs.move_dir.normalized()
            } else {
                inputs.move_dir
            };

            // Update `inputs.look_dir`
            inputs
                .look_dir
                .try_normalized()
                .unwrap_or(inputs.move_dir.into());

            // Process other controller events
            for event in controller.events.drain(..) {
                match event {
                    ControlEvent::Mount(mountee_uid) => {
                        if let Some(mountee_entity) =
                            uid_allocator.retrieve_entity_internal(mountee_uid.id())
                        {
                            server_emitter.emit(ServerEvent::Mount(entity, mountee_entity));
                        }
                    }
                    ControlEvent::Unmount => server_emitter.emit(ServerEvent::Unmount(entity)),
                    ControlEvent::InventoryManip(manip) => {
                        server_emitter.emit(ServerEvent::InventoryManip(entity, manip))
                    } //ControlEvent::Respawn => server_emitter.emit(ServerEvent::Unmount(entity)),
                }
            }
        }
    }
}
