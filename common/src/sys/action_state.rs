// Library
use specs::{Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{phys::Pos, ActionState, Control},
    state::DeltaTime,
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (Read<'a, DeltaTime>, WriteStorage<'a, ActionState>);

    fn run(&mut self, (dt, mut action_state): Self::SystemData) {
        for (mut action_state) in (&mut action_state).join() {
            action_state.time += dt.0 as f64;
            if action_state.attack_started {
                action_state.attack_started = false;
                println!("POW");
            }
        }
    }
}
