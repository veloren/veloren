use crate::{comp::{Attacking, Rolling, Cidling}, state::DeltaTime};
use specs::{Entities, Join, Read, System, WriteStorage};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        WriteStorage<'a, Attacking>,
        WriteStorage<'a, Rolling>,
        WriteStorage<'a, Cidling>,

    );

    fn run(&mut self, (entities, dt, mut attacks, mut rolls, mut cidles): Self::SystemData) {
        for (entity, attack) in (&entities, &mut attacks).join() {
            attack.time += dt.0;
        }
        let finished_attacks = (&entities, &mut attacks)
            .join()
            .filter(|(_, a)| {
                a.time > 0.25 // TODO: constant
            })
            .map(|(e, _)| e)

            .collect::<Vec<_>>();

        for entity in finished_attacks {
            attacks.remove(entity);
        }
        for (entity, roll) in (&entities, &mut rolls).join() {
            roll.time += dt.0;
        }
        let finished_rolls = (&entities, &mut rolls)
            .join()
            .filter(|(_, a)| {
                a.time > 0.8 // TODO: constant
            })
            .map(|(e, _)| e)
            .collect::<Vec<_>>();

        for entity in finished_rolls {
            rolls.remove(entity);
        }
        for (entity, cidle) in (&entities, &mut cidles).join() {
            cidle.time += dt.0;
        }
        let finished_cidles = (&entities, &mut cidles)
            .join()
            .filter(|(_, a)| {
                a.time > 5.0 // TODO: constant
            })
            .map(|(e, _)| e)
            .collect::<Vec<_>>();

        for entity in finished_cidles {
            cidles.remove(entity);
        }
    }
}
