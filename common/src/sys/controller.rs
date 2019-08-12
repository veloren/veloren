use {
    crate::comp::{
        Ability, Attack, Body, Controller, Glide, Jump, MoveDir, PhysicsState, Respawn, Roll,
        Stats, Vel, Wield,
    },
    specs::{Entities, Join, ReadStorage, System, WriteStorage},
};

/// This system is responsible for validating controller inputs
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Controller>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, Ability<MoveDir>>,
        WriteStorage<'a, Ability<Jump>>,
        WriteStorage<'a, Ability<Attack>>,
        WriteStorage<'a, Ability<Wield>>,
        WriteStorage<'a, Ability<Roll>>,
        WriteStorage<'a, Ability<Respawn>>,
        WriteStorage<'a, Ability<Glide>>,
    );

    fn run(
        &mut self,
        (
            entities,
            controllers,
            stats,
            velocities,
            physics_states,
            mut move_dirs,
            mut jumps,
            mut attacks,
            mut wields,
            mut rolls,
            mut respawns,
            mut glides,
        ): Self::SystemData,
    ) {
        for (entity, controller, stats, vel, physics_state) in (
            &entities,
            &controllers,
            &stats,
            &velocities,
            &physics_states,
        )
            .join()
        {
            if stats.is_dead {
                // Respawn
                if controller.respawn {
                    //TODO
                }
                continue;
            }

            // Move dir
            if !rolls.get(entity).filter(|r| r.started()).is_some() {
                if let Some(move_dir) = move_dirs.get_mut(entity) {
                    move_dir.try_start();
                    move_dir.0 = if controller.move_dir.magnitude_squared() > 1.0 {
                        controller.move_dir.normalized()
                    } else {
                        controller.move_dir
                    };
                }
            }

            // Glide
            if controller.glide
                && !physics_state.on_ground
                && !attacks.get(entity).filter(|a| a.started()).is_some()
                && !rolls.get(entity).filter(|r| r.started()).is_some()
            {
                glides.get_mut(entity).map(|g| g.try_start());
            } else {
                glides.get_mut(entity).map(|g| g.stop());
            }

            // Combat
            if controller.attack
                && !glides.get(entity).filter(|g| g.started()).is_some()
                && !rolls.get(entity).filter(|r| r.started()).is_some()
            {
                let mut ready = false;

                if let Some(wield) = wields.get_mut(entity) {
                    if wield.applied {
                        // TODO: Adjust value
                        ready = true;
                    } else if !wield.started() {
                        wield.try_start();
                    }
                } else {
                    // No need to wield
                    ready = true;
                }

                if ready {
                    attacks.get_mut(entity).map(|a| a.try_start());
                }
            }

            // Roll
            if controller.roll
                && physics_state.on_ground
                && vel.0.magnitude_squared() > 0.2
                && !attacks.get(entity).filter(|a| a.started()).is_some()
                && !glides.get(entity).filter(|g| g.started()).is_some()
            {
                rolls.get_mut(entity).map(|roll| roll.try_start());
            }

            // Jump
            if controller.jump && physics_state.on_ground && vel.0.z <= 0.0 {
                jumps.get_mut(entity).map(|j| j.try_start());
            }
        }
    }
}
