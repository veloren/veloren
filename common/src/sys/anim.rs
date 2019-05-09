// Library
use specs::{Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{phys::Pos, AnimationHistory, Control},
    state::DeltaTime,
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, DeltaTime>,
        WriteStorage<'a, AnimationHistory>,
    );

    fn run(&mut self, (dt, mut anim_history): Self::SystemData) {
        for (mut anim_history) in (&mut anim_history).join() {
            anim_history.time += dt.0 as f64;
        }
    }
}
