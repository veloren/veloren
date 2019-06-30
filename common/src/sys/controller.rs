use crate::{
    comp::{
        ActionState, Animation, AnimationInfo, Attacking, Controller, Gliding, HealthSource,
        Jumping, MoveDir, OnGround, Respawning, Rolling, Stats, {ForceUpdate, Ori, Pos, Vel},
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
        ReadStorage<'a, Vel>,
        ReadStorage<'a, Ori>,
        WriteStorage<'a, ActionState>,
        WriteStorage<'a, MoveDir>,
        WriteStorage<'a, Jumping>,
        WriteStorage<'a, Attacking>,
        WriteStorage<'a, Rolling>,
        WriteStorage<'a, Respawning>,
        WriteStorage<'a, Gliding>,
    );

    fn run(
        &mut self,
        (
            entities,
            dt,
            controllers,
            stats,
            positions,
            velocities,
            orientations,
            mut action_states,
            mut move_dirs,
            mut jumpings,
            mut attackings,
            mut rollings,
            mut respawns,
            mut glidings,
        ): Self::SystemData,
    ) {
        for (entity, controller, stats, pos, vel, ori, mut a) in (
            &entities,
            &controllers,
            &stats,
            &positions,
            &velocities,
            &orientations,
            // Although this is changed, it is only kept for this system
            // as it will be replaced in the action state system
            &mut action_states,
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

            // Move dir
            if !a.rolling {
                move_dirs.insert(
                    entity,
                    MoveDir(if controller.move_dir.magnitude_squared() > 1.0 {
                        controller.move_dir.normalized()
                    } else {
                        controller.move_dir
                    }),
                );
            }

            // Glide
            if controller.glide && !a.on_ground && !a.attacking && !a.rolling {
                glidings.insert(entity, Gliding);
                a.gliding = true;
            } else {
                glidings.remove(entity);
                a.gliding = false;
            }

            // Attack
            if controller.attack && !a.attacking && !a.gliding && !a.rolling {
                attackings.insert(entity, Attacking::start());
                a.attacking = true;
            }

            // Roll
            if controller.roll
                && !a.rolling
                && a.on_ground
                && a.moving
                && !a.attacking
                && !a.gliding
            {
                rollings.insert(entity, Rolling::start());
                a.rolling = true;
            }

            // Jump
            if controller.jump && a.on_ground && vel.0.z <= 0.0 {
                jumpings.insert(entity, Jumping);
                a.on_ground = false;
            }
        }
    }
}
