// Library
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{
        phys::{Dir, ForceUpdate, Pos, Vel},
        Action, Actions, Control, Stats,
    },
    state::{DeltaTime, Time},
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, Time>,
        Read<'a, DeltaTime>,
        WriteStorage<'a, Actions>,
        ReadStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        ReadStorage<'a, Dir>,
        WriteStorage<'a, Stats>,
        WriteStorage<'a, ForceUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            time,
            dt,
            mut actions,
            positions,
            mut velocities,
            directions,
            mut stats,
            mut force_updates,
        ): Self::SystemData,
    ) {
        for (a, actions_a, pos_a, dir_a) in
            (&entities, &mut actions, &positions, &directions).join()
        {
            for event in actions_a.0.drain(..) {
                match event {
                    Action::Attack => {
                        for (b, pos_b, stat_b, vel_b) in
                            (&entities, &positions, &mut stats, &mut velocities).join()
                        {
                            if a == b {
                                continue;
                            }
                            if a != b
                                && pos_a.0.distance_squared(pos_b.0) < 50.0
                                && dir_a.0.angle_between_degrees(pos_b.0 - pos_a.0) < 70.0
                            {
                                stat_b.hp.change_by(-10); // TODO: variable damage
                                vel_b.0 += (pos_b.0 - pos_a.0).normalized() * 20.0;
                                vel_b.0.z = 20.0;
                                force_updates.insert(b, ForceUpdate);
                            }
                        }
                    }
                }
            }
        }
    }
}
