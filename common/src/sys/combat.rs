use crate::{
    comp::{Ability, Attack, ForceUpdate, HealthSource, Ori, Pos, Stats, Vel, Wield},
    state::{DeltaTime, Uid},
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};

/// This system is responsible for handling attacks.
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Uid>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Ori>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ability<Wield>>,
        WriteStorage<'a, Ability<Attack>>,
        WriteStorage<'a, Stats>,
        WriteStorage<'a, ForceUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            uids,
            dt,
            positions,
            orientations,
            mut velocities,
            mut wields,
            mut attacks,
            mut stats,
            mut force_updates,
        ): Self::SystemData,
    ) {
        // Attack
        for (entity, uid, pos, ori, mut attack) in
            (&entities, &uids, &positions, &orientations, &mut attacks).join()
        {
            // TODO: Currently we hit right after pressing attack. This might change when we
            // have better combat animations and weapons.
            if attack.started() && !attack.applied {
                // Go through all other entities
                for (b, pos_b, mut vel_b, stat_b) in
                    (&entities, &positions, &mut velocities, &mut stats).join()
                {
                    // Check if it is a hit
                    if entity != b
                            && !stat_b.is_dead
                            && pos.0.distance_squared(pos_b.0) < 50.0
                            // TODO: Aiming
                            && ori.0.angle_between(pos_b.0 - pos.0).to_degrees() < 90.0
                    {
                        // Deal damage
                        stat_b
                            .health
                            .change_by(-10, HealthSource::Attack { by: *uid }); // TODO: variable damage and weapon

                        // Knockback
                        vel_b.0 += (pos_b.0 - pos.0).normalized() * 2.0;
                        vel_b.0.z = 2.0;
                        if let Some(force_update) = force_updates.get_mut(b) {
                            force_update.0 = true;
                        }
                    }
                }
                attack.applied = true;
            }

            // TODO: Make this weapon dependent
            if attack.time > 0.5 {
                attack.stop()
            } else if attack.started() {
                attack.time += dt.0;
            }
        }

        // Wields
        for wield in (&mut wields).join() {
            // TODO: Make this weapon dependent
            if wield.started() {
                if !wield.applied && wield.time > 0.3 {
                    wield.applied = true;
                } else {
                    wield.time += dt.0;
                }
            }
        }
    }
}
