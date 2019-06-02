use crate::{comp::AnimationInfo, state::DeltaTime};
use specs::{Join, Read, System, WriteStorage};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (Read<'a, DeltaTime>, WriteStorage<'a, AnimationInfo>);

    fn run(&mut self, (dt, mut animation_infos): Self::SystemData) {
        for mut animation_info in (&mut animation_infos).join() {
            animation_info.time += dt.0 as f64;
        }
    }
}
