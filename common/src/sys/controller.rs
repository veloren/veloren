use crate::{
    comp::{
        phys::{ForceUpdate, Ori, Pos, Vel},
        Attacking, Controller, Gliding, Jumping, MoveDir, OnGround, Respawning, Stats,
    },
    state::DeltaTime,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};

/// This system is responsible for validating controller inputs
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Controller>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, MoveDir>,
        WriteStorage<'a, OnGround>,
        WriteStorage<'a, Jumping>,
        WriteStorage<'a, Attacking>,
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
            positions,
            mut velocities,
            mut orientations,
            mut move_dirs,
            mut on_grounds,
            mut jumpings,
            mut attackings,
            mut respawns,
            mut glidings,
            force_updates,
        ): Self::SystemData,
    ) {
        for (entity, controller, stats, pos, mut vel, mut ori, on_ground, mut attacking, mut jumping, mut gliding) in (
            &entities,
            &controllers,
            &stats,
            &positions,
            &mut velocities,
            &mut orientations,
            on_grounds.maybe(),
            attackings.maybe(),
            jumpings.maybe(),
            glidings.maybe(),
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
            if controller.glide && on_ground.is_none() && attacking.is_none() {
                gliding = Some(&Gliding);
            } else {
                gliding = None
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
            if controller.attack && attacking.is_none() && gliding.is_none() {
                attacking = Some(&Attacking::start());
            }

            // Jump
            if on_ground.is_some() && controller.jump && vel.0.z <= 0.0 {
                jumping = Some(&Jumping);
            } else {
                jumping = None;
            }
        }
    }
}
