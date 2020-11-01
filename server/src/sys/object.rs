use common::{
    comp::{HealthChange, HealthSource, Object, PhysicsState, Pos, Vel},
    effect::Effect,
    event::{EventBus, ServerEvent},
    span,
    state::DeltaTime,
    Explosion, RadiusEffect,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};

/// This system is responsible for handling misc object behaviours
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, Object>,
    );

    fn run(
        &mut self,
        (entities, _dt, server_bus, positions, velocities, physics_states, mut objects): Self::SystemData,
    ) {
        span!(_guard, "run", "object::Sys::run");
        let mut server_emitter = server_bus.emitter();

        // Objects
        for (entity, pos, vel, physics, object) in (
            &entities,
            &positions,
            &velocities,
            &physics_states,
            &mut objects,
        )
            .join()
        {
            match object {
                Object::Bomb { owner } => {
                    if physics.on_surface().is_some() {
                        server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::Suicide,
                        });
                        server_emitter.emit(ServerEvent::Explosion {
                            pos: pos.0,
                            explosion: Explosion {
                                effects: vec![
                                    RadiusEffect::EntityEffect(Effect::Health(HealthChange {
                                        amount: -500,
                                        cause: HealthSource::Explosion { owner: *owner },
                                    })),
                                    RadiusEffect::TerrainDestruction(4.0),
                                ],
                                radius: 12.0,
                                energy_regen: 0,
                            },
                            owner: *owner,
                            reagent: None,
                        });
                    }
                },
                Object::Firework { owner, reagent } => {
                    if vel.0.z < 0.0 {
                        server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::Suicide,
                        });
                        server_emitter.emit(ServerEvent::Explosion {
                            pos: pos.0,
                            explosion: Explosion {
                                effects: vec![
                                    RadiusEffect::EntityEffect(Effect::Health(HealthChange {
                                        amount: -50,
                                        cause: HealthSource::Explosion { owner: *owner },
                                    })),
                                    RadiusEffect::TerrainDestruction(4.0),
                                ],
                                radius: 12.0,
                                energy_regen: 0,
                            },
                            owner: *owner,
                            reagent: Some(*reagent),
                        });
                    }
                },
            }
        }
    }
}
