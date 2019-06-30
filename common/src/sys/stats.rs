use crate::{
    comp::{Dying, HealthSource, Stats},
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
        WriteStorage<'a, Stats>,
        WriteStorage<'a, Dying>,
    );

    fn run(&mut self, (entities, dt, mut stats, mut dyings): Self::SystemData) {
        for (entity, mut stat) in (&entities, &mut stats).join() {
            if stat.should_die() && !stat.is_dead {
                // TODO: Replace is_dead with client states
                if let Err(err) = dyings.insert(
                    entity,
                    Dying {
                        cause: match stat.health.last_change {
                            Some(change) => change.2,
                            None => {
                                warn!("Nothing caused an entity to die!");
                                HealthSource::Unknown
                            }
                        },
                    },
                ) {
                    warn!("Inserting Dying for an entity failed: {:?}", err);
                }
                stat.is_dead = true;
            }
            if let Some(change) = &mut stat.health.last_change {
                change.1 += dt.0 as f64;
            }
        }
    }
}
