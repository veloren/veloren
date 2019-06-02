use crate::{
    comp::{Dying, Stats},
    state::DeltaTime,
};
use specs::{Entities, Join, Read, System, WriteStorage};

// Basic ECS AI agent system
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
                dyings
                    .insert(
                        entity,
                        Dying {
                            cause: stat
                                .hp
                                .last_change
                                .expect("Nothing caused the entity to die")
                                .2, // Safe because damage is necessary for death
                        },
                    )
                    .expect("Inserting dying for an entity failed!");
                stat.is_dead = true;
            }
            if let Some(change) = &mut stat.hp.last_change {
                change.1 += dt.0 as f64;
            }
        }
    }
}
