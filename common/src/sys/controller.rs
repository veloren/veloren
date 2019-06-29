use crate::{
    comp::{
        Animation, AnimationInfo, Attacking, Controller, Gliding, HealthSource, Jumping, MoveDir,
        OnGround, Respawning, Rolling, Stats, {ForceUpdate, Ori, Pos, Vel},
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
        ReadStorage<'a, OnGround>,
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
            on_grounds,
            mut move_dirs,
            mut jumpings,
            mut attackings,
            mut rollings,
            mut respawns,
            mut glidings,
        ): Self::SystemData,
    ) {
        for (entity, controller, stats, pos, vel, ori, on_ground) in (
            &entities,
            &controllers,
            &stats,
            &positions,
            &velocities,
            &orientations,
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

            // Move dir
            if rollings.get(entity).is_none() {
                move_dirs.insert(
                    entity,
                    MoveDir(if controller.move_dir.magnitude() > 1.0 {
                        controller.move_dir.normalized()
                    } else {
                        controller.move_dir
                    }),
                );
            }

            // Glide
            if controller.glide
                && on_ground.is_none()
                && attackings.get(entity).is_none()
                && rollings.get(entity).is_none()
            {
                glidings.insert(entity, Gliding);
            } else {
                glidings.remove(entity);
            }

            // Attack
            if controller.attack
                && attackings.get(entity).is_none()
                && glidings.get(entity).is_none()
                && rollings.get(entity).is_none()
            {
                attackings.insert(entity, Attacking::start());
            }

            // Jump
            if controller.jump && on_ground.is_some() && vel.0.z <= 0.0 {
                jumpings.insert(entity, Jumping);
            }

            // Roll
            if controller.roll
                && rollings.get(entity).is_none()
                && attackings.get(entity).is_none()
                && glidings.get(entity).is_none()
                && on_ground.is_some()
                && vel.0.magnitude_squared() > 25.0
            {
                rollings.insert(entity, Rolling::start());
            }
        }
    }
}
