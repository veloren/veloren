// Library
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{phys::Pos, Stats, ActionState, Control},
    state::DeltaTime,
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Stats>,
    );

    fn run(&mut self, (entities, mut stats): Self::SystemData) {
        for (entity, mut stat) in (&entities, &mut stats).join() {
            if stat.hp.current == 0 {
                entities.delete(entity);
            }
        }
    }
}
