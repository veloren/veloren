// Library
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{
        phys::{Ori, Pos, Vel},
        Animation, AnimationInfo, Attacking, Rolling,
    },
    state::DeltaTime,
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        WriteStorage<'a, Attacking,>,
        WriteStorage<'a, Rolling,>, 


    );

    fn run(&mut self, (entities, dt, mut attacks, mut rolls): Self::SystemData) {
        for (entity, attack, roll) in (&entities, &mut attacks, &mut rolls).join() {
            attack.time += dt.0;
            roll.time += dt.0;
        }
        let finished_attacks = (&entities, &mut attacks)
            .join()
            .filter(|(e, a)| {
                a.time > 0.25 // TODO: constant
            })
            .map(|(e, a)| e)
            .collect::<Vec<_>>();

        for entity in finished_attacks {
            attacks.remove(entity);
        }
        let finished_rolls = (&entities, &mut rolls)
            .join()
            .filter(|(e, a)| {
                a.time > 0.25 // TODO: constant
            })
            .map(|(e, a)| e)
            .collect::<Vec<_>>();

        for entity in finished_rolls {
            rolls.remove(entity);
        }
    }
}
