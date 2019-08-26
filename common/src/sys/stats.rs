use crate::{
    comp::{HealthSource, Stats},
    event::{EventBus, ServerEvent},
    state::DeltaTime,
};
use log::warn;
use specs::{Entities, Join, Read, System, WriteStorage};

/// This system kills players
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, EventBus<ServerEvent>>,
        WriteStorage<'a, Stats>,
    );

    fn run(&mut self, (entities, dt, event_bus, mut stats): Self::SystemData) {
        let mut event_emitter = event_bus.emitter();

        for (entity, mut stat) in (&entities, &mut stats).join() {
            if stat.should_die() && !stat.is_dead {
                event_emitter.emit(ServerEvent::Die {
                    entity,
                    cause: match stat.health.last_change {
                        Some(change) => change.2,
                        None => {
                            warn!("Nothing caused an entity to die!");
                            HealthSource::Unknown
                        }
                    },
                });

                stat.is_dead = true;
            }

            if let Some(change) = &mut stat.health.last_change {
                change.1 += f64::from(dt.0);
            }

            if stat.exp.current() >= stat.exp.maximum() {
                stat.exp.change_by(-stat.exp.maximum());
                stat.exp.change_maximum_by(25.0);
                stat.level.change_by(1);
                stat.health.set_maximum(stat.health.maximum() + 10);
                stat.health
                    .set_to(stat.health.maximum(), HealthSource::LevelUp)
            }
        }
    }
}
