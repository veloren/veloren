use crate::comp::{
    ActionState, Attacking, Body, Controller, Gliding, Jumping, MoveDir, Respawning, Rolling,
    Stats, Vel, Wielding,
};
use specs::{Entities, Join, ReadStorage, System, WriteStorage};

/// This system is responsible for validating controller inputs
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Controller>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Vel>,
        WriteStorage<'a, ActionState>,
        WriteStorage<'a, MoveDir>,
        WriteStorage<'a, Jumping>,
        WriteStorage<'a, Attacking>,
        WriteStorage<'a, Wielding>,
        WriteStorage<'a, Rolling>,
        WriteStorage<'a, Respawning>,
        WriteStorage<'a, Gliding>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut controllers,
            stats,
            bodies,
            velocities,
            mut action_states,
            mut move_dirs,
            mut jumpings,
            mut attackings,
            mut wieldings,
            mut rollings,
            mut respawns,
            mut glidings,
        ): Self::SystemData,
    ) {
        for (entity, controller, stats, body, vel, mut a) in (
            &entities,
            &mut controllers,
            &stats,
            &bodies,
            &velocities,
            // Although this is changed, it is only kept for this system
            // as it will be replaced in the action state system
            &mut action_states,
        )
            .join()
        {
            if stats.is_dead {
                // Respawn
                if controller.respawn {
                    let _ = respawns.insert(entity, Respawning);
                }
                continue;
            }

            // Move dir
            if !a.rolling {
                let _ = move_dirs.insert(
                    entity,
                    MoveDir(if controller.move_dir.magnitude_squared() > 1.0 {
                        controller.move_dir.normalized()
                    } else {
                        controller.move_dir
                    }),
                );
            }

            // Glide
            if controller.glide && !a.on_ground && !a.attacking && !a.rolling && body.is_humanoid()
            {
                let _ = glidings.insert(entity, Gliding);
                a.gliding = true;
            } else {
                let _ = glidings.remove(entity);
                a.gliding = false;
            }

            // Wield
            if controller.attack && !a.wielding && !a.gliding && !a.rolling {
                let _ = wieldings.insert(entity, Wielding::start());
                a.wielding = true;
            }

            // Attack
            if controller.attack
                && !a.attacking
                && wieldings.get(entity).map(|w| w.applied).unwrap_or(false)
                && !a.gliding
                && !a.rolling
            {
                let _ = attackings.insert(entity, Attacking::start());
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
                let _ = rollings.insert(entity, Rolling::start());
                a.rolling = true;
            }

            // Jump
            if controller.jump && a.on_ground && vel.0.z <= 0.0 {
                let _ = jumpings.insert(entity, Jumping);
                a.on_ground = false;
            }

            // Reset the controller ready for the next tick
            *controller = Controller::default();
        }
    }
}
