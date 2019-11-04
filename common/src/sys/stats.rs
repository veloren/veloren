use crate::{
    comp::{HealthSource, Stats},
    event::{EventBus, ServerEvent, SfxEvent, SfxEventItem},
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
        Read<'a, EventBus<SfxEventItem>>,
        WriteStorage<'a, Stats>,
    );

    fn run(
        &mut self,
        (entities, dt, server_event_bus, audio_event_bus, mut stats): Self::SystemData,
    ) {
        let mut server_event_emitter = server_event_bus.emitter();

        // Mutates all stats every tick causing the server to resend this component for every entity every tick
        for (entity, mut stat) in (&entities, &mut stats).join() {
            if stat.should_die() && !stat.is_dead {
                server_event_emitter.emit(ServerEvent::Destroy {
                    entity,
                    cause: stat.health.last_change.1.cause,
                });

                stat.is_dead = true;
            }

            stat.health.last_change.0 += f64::from(dt.0);

            if stat.exp.current() >= stat.exp.maximum() {
                while stat.exp.current() >= stat.exp.maximum() {
                    stat.exp.change_by(-(stat.exp.maximum() as i64));
                    stat.exp.change_maximum_by(25);
                    stat.level.change_by(1);
                }

                audio_event_bus
                    .emitter()
                    .emit(SfxEventItem::at_player_position(SfxEvent::LevelUp));

                stat.update_max_hp();
                stat.health
                    .set_to(stat.health.maximum(), HealthSource::LevelUp)
            }
        }
    }
}
