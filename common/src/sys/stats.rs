use crate::{
    comp::{HealthSource, Stats},
    event::{EventBus, ServerEvent},
    state::DeltaTime,
};
use specs::{Entities, Join, Read, System, WriteStorage};

/// This system kills players
/// and handles players levelling up
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, EventBus<ServerEvent>>,
        WriteStorage<'a, Stats>,
    );

    fn run(&mut self, (entities, dt, server_event_bus, mut stats): Self::SystemData) {
        let mut server_event_emitter = server_event_bus.emitter();

        // Increment last change timer
        stats.set_event_emission(false); // avoid unnecessary syncing
        for stat in (&mut stats).join() {
            stat.health.last_change.0 += f64::from(dt.0);
        }
        stats.set_event_emission(true);

        // Mutates all stats every tick causing the server to resend this component for every entity every tick
        for (entity, mut stats) in (&entities, &mut stats.restrict_mut()).join() {
            let (set_dead, level_up) = {
                let stat = stats.get_unchecked();
                (
                    stat.should_die() && !stat.is_dead,
                    stat.exp.current() >= stat.exp.maximum(),
                )
            };

            if set_dead {
                let stat = stats.get_mut_unchecked();
                server_event_emitter.emit(ServerEvent::Destroy {
                    entity,
                    cause: stat.health.last_change.1.cause,
                });

                stat.is_dead = true;
            }

            if level_up {
                let stat = stats.get_mut_unchecked();
                while stat.exp.current() >= stat.exp.maximum() {
                    stat.exp.change_by(-(stat.exp.maximum() as i64));
                    stat.exp.change_maximum_by(25);
                    stat.level.change_by(1);
                }

                stat.update_max_hp();
                stat.health
                    .set_to(stat.health.maximum(), HealthSource::LevelUp)
            }
        }
    }
}
