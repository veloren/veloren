// Library
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{phys::Pos, Animation, AnimationInfo, Control, Stats},
    state::DeltaTime,
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (Read<'a, DeltaTime>, WriteStorage<'a, AnimationInfo>);

    fn run(&mut self, (dt, mut animation_infos): Self::SystemData) {
        for (mut animation_info) in (&mut animation_infos).join() {
            animation_info.time += dt.0 as f64;
            &animation_info.time;
        }
    }
}
