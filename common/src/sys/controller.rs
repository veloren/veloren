use crate::{
    comp::{
        phys::{ForceUpdate, Ori, Pos, Vel},
        Animation, AnimationInfo, Attacking, Rolling, Crunning, Cidling, Controller, Gliding, HealthSource, Jumping, MoveDir,
        OnGround, Respawning, Stats,
    },
    state::{DeltaTime, Uid},
    terrain::TerrainMap,
    vol::{ReadVol, Vox},
};
use log::warn;
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

/// This system is responsible for validating controller inputs
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Controller>,
        ReadStorage<'a, Stats>,
        ReadExpect<'a, TerrainMap>,
        ReadStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, MoveDir>,
        WriteStorage<'a, OnGround>,
        WriteStorage<'a, Jumping>,
        WriteStorage<'a, Attacking>,
        WriteStorage<'a, Rolling>,
        WriteStorage<'a, Crunning>,
        WriteStorage<'a, Cidling>,
        WriteStorage<'a, Respawning>,
        WriteStorage<'a, Gliding>,
        WriteStorage<'a, ForceUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            dt,
            controllers,
            stats,
            terrain,
            positions,
            mut velocities,
            mut orientations,
            mut move_dirs,
            mut on_grounds,
            mut jumpings,
            mut attackings,
            mut rollings,
            mut crunnings,
            mut cidlings,
            mut respawns,
            mut glidings,
            mut force_updates,
        ): Self::SystemData,
    ) {
        for (entity, controller, stats, pos, mut vel, mut ori, on_ground) in (
            &entities,
            &controllers,
            &stats,
            &positions,
            &mut velocities,
            &mut orientations,
            on_grounds.maybe(),
        )
            .join()
        {
            if stats.is_dead {
                // Respawn
                if controller.respawn {
                    respawns.insert(entity, Respawning);
                }
                continue;
            }

            // Glide
            if controller.glide && on_ground.is_none() && attackings.get(entity).is_none() {
                glidings.insert(entity, Gliding);
            } else {
                glidings.remove(entity);
            }

            // Move dir
            move_dirs.insert(
                entity,
                MoveDir(if controller.move_dir.magnitude() > 1.0 {
                    controller.move_dir.normalized()
                } else {
                    controller.move_dir
                }),
            );

            // Attack
            if controller.attack
                && attackings.get(entity).is_none()
                && glidings.get(entity).is_none()
            {
                attackings.insert(entity, Attacking::start());
            }

            // Jump
            if on_grounds.get(entity).is_some() && controller.jump && vel.0.z <= 0.0 {
                jumpings.insert(entity, Jumping);
            } else {
                jumpings.remove(entity);
            }

            // Roll
            if on_grounds.get(entity).is_some() && controller.roll {
                rollings.insert(entity, Rolling::start());
            } else {
                rollings.remove(entity);
            }
        }
    }
}
