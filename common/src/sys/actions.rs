// Library
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::{
    comp::{
        phys::{Dir, Pos, Vel},
        Animation, AnimationInfo, Attacking,
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
        WriteStorage<'a, Attacking>,
    );

    fn run(&mut self, (entities, dt, mut attackings): Self::SystemData) {
        for (entity, attacking) in (&entities, &mut attackings).join() {
            attacking.time += dt.0;
        }

        let finished_attack = (&entities, &mut attackings)
            .join()
            .filter(|(e, a)| {
                a.time > 0.5 // TODO: constant
            })
            .map(|(e, a)| e)
            .collect::<Vec<_>>();

        for entity in finished_attack {
            attackings.remove(entity);
        }
    }
}
