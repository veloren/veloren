use crate::{
    comp::{CharacterState, ControlEvent, Controller},
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
    sync::{Uid, UidAllocator},
};
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entities, Join, Read, ReadStorage, System, WriteStorage,
};
use std::time::Duration;

// const CHARGE_COST: i32 = 200;
// const ROLL_COST: i32 = 30;

pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, UidAllocator>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, EventBus<LocalEvent>>,
        Read<'a, DeltaTime>,
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
            read_dt,
            mut controllers,
            mut character_states,
            uids,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();
        let dt = Duration::from_secs_f32(read_dt.0);

        for (entity, _uid, controller, character_state) in (
            &entities,
            &uids,
            &mut controllers,
            // &last_controllers,
            &mut character_states,
        )
            .join()
        {
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
                    },
                    ControlEvent::Unmount => server_emitter.emit(ServerEvent::Unmount(entity)),
                    ControlEvent::InventoryManip(manip) => {
                        *character_state = CharacterState::Idle;
                        server_emitter.emit(ServerEvent::InventoryManip(entity, manip))
                    }, /*ControlEvent::Respawn =>
                        * server_emitter.emit(ServerEvent::Unmount(entity)), */
                }
            }
        }
    }
}
