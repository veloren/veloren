use crate::{
    comp::{EntityEvent, Events, HealthSource, Stats},
    state::DeltaTime,
};
use log::warn;
use specs::{Entities, Join, Read, System, WriteStorage};

/// This system kills players
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Events>,
        WriteStorage<'a, Stats>,
    );

    fn run(&mut self, (entities, mut events, mut stats): Self::SystemData) {
        for (entity, mut events) in (&entities, &mut events).join() {
            for event in events.drain(..) {
                match event {
                    EntityEvent::HitGround { vel } => {
                        if let Some(stat) = stats.get_mut(entity) {
                            let falldmg = (vel.z / 1.5 + 6.0) as i32;
                            if falldmg < 0 {
                                stat.health.change_by(falldmg, HealthSource::World);
                            }
                        }
                    }
                }
            }
        }
    }
}
