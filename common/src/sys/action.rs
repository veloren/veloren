// Library
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{phys::Pos, Action, Actions, Control, Stats},
    state::DeltaTime,
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        WriteStorage<'a, Actions>,
        ReadStorage<'a, Pos>,
        WriteStorage<'a, Stats>,
    );

    fn run(&mut self, (entities, dt, mut actions, positions, mut stats): Self::SystemData) {
        for (a, mut actions_a, pos_a) in (&entities, &mut actions, &positions).join() {
            for event in actions_a.0.drain(..) {
                match event {
                    Action::Attack => {
                        for (b, pos_b, stat_b) in (&entities, &positions, &mut stats).join() {
                            if a == b {
                                continue;
                            }
                            if pos_a.0.distance_squared(pos_b.0) < 50.0 {
                                &mut stat_b.hp.change_by(-60, 0.0); // TODO: variable damage and current time
                                &stat_b.hp;
                            }
                        }
                    }
                }
            }
        }
    }
}
