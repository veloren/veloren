// Library
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{
        phys::{Dir, Pos, Vel},
        Actions, Animation, AnimationInfo,
    },
    state::DeltaTime,
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (Entities<'a>, Read<'a, DeltaTime>, WriteStorage<'a, Actions>);

    fn run(&mut self, (entities, dt, mut actions): Self::SystemData) {
        for (entity, mut action) in (&entities, &mut actions).join() {
            let should_end = action.attack_time.as_mut().map_or(false, |mut time| {
                *time += dt.0;
                *time > 0.5 // TODO: constant
            });

            if should_end {
                action.attack_time = None;
            }
        }
    }
}
