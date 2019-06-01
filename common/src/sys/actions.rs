// Library
use specs::{Entities, Join, Read, System, WriteStorage};

// Crate
use crate::{comp::Attacking, state::DeltaTime};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        WriteStorage<'a, Attacking>,
    );

    fn run(&mut self, (entities, dt, mut attacks): Self::SystemData) {
        for attack in (&mut attacks).join() {
            attack.time += dt.0;
        }

        let finished_attacks = (&entities, &mut attacks)
            .join()
            .filter(|(_e, a)| a.time > 0.25) // TODO: constant
            .map(|(e, _a)| e)
            .collect::<Vec<_>>();

        for entity in finished_attacks {
            attacks.remove(entity);
        }
    }
}
