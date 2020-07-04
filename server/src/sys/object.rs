use common::{
    comp::{HealthSource, Object, PhysicsState, Pos},
    event::{EventBus, ServerEvent},
    state::DeltaTime,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use std::time::Duration;

/// This system is responsible for handling projectile effect triggers
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, Object>,
    );

    fn run(
        &mut self,
        (entities, dt, server_bus, positions, physics_states, mut objects): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();

        // Objects
        for (entity, pos, _physics, object) in
            (&entities, &positions, &physics_states, &mut objects).join()
        {
            match object {
                Object::Bomb { owner, timeout } => {
                    if let Some(t) = timeout.checked_sub(Duration::from_secs_f32(dt.0)) {
                        *timeout = t;
                    } else {
                        server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::Suicide,
                        });
                        server_emitter.emit(ServerEvent::Explosion {
                            pos: pos.0,
                            power: 4.0,
                            owner: *owner,
                        });
                    }
                },
            }
        }
    }
}
