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
        Read<'a, DeltaTime>,
        WriteStorage<'a, ActionState>,
        ReadStorage<'a, Pos>,
        WriteStorage<'a, Stats>,
    );

    fn run(&mut self, (entities, dt, mut action_states, positions, mut stats): Self::SystemData) {
        for (a, mut action_state_a, pos_a) in (&entities, &mut action_states, &positions).join() {
            if action_state_a.attack_started {
                action_state_a.attack_started = false;
                for (b, pos_b, stat_b) in (&entities, &positions, &mut stats).join() {
                    if a == b {
                        continue;
                    }
                    if pos_a.0.distance_squared(pos_b.0) < 50.0 {
                        stat_b.hp.change_by(-1000, 0.0);
                    }
                }
            }
            action_state_a.time += dt.0 as f64;
        }
    }
}
