// Library
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{Control, Dying, Stats},
    state::DeltaTime,
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Stats>,
        WriteStorage<'a, Dying>,
    );

    fn run(&mut self, (entities, stats, mut dyings): Self::SystemData) {
        for (entity, stat) in (&entities, &stats).join() {
            if stat.hp.current == 0 {
                dyings.insert(entity, Dying);
            }
        }
    }
}
