// Library
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{phys::Pos, ActionState, ActionEvent, Control, Stats},
    state::DeltaTime,
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, DeltaTime>,
        WriteStorage<'a, ActionState>,
    );

    fn run(&mut self, (dt, mut action_states): Self::SystemData) {
        for (dt, mut animation) in (dt, &mut animation).join() {
            animation.time += dt.0 as f64;
        }
    }
}
