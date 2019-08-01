use crate::{
    comp::{Attacking, ForceUpdate, HealthSource, Ori, Pos, Stats, Vel, Wielding},
    state::{DeltaTime, Uid},
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};

/// This system is responsible for handling accepted inputs like moving or attacking
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Uid>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Ori>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Attacking>,
        WriteStorage<'a, Wielding>,
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
            mut attackings,
            mut wieldings,
            mut stats,
            mut force_updates,
        ): Self::SystemData,
    ) {
        // Attacks
        (&entities, &uids, &positions, &orientations, &mut attackings)
            .join()
            .filter_map(|(entity, uid, pos, ori, mut attacking)| {
                if !attacking.applied {
                    // Go through all other entities
                    for (b, pos_b, mut vel_b, stat_b) in
                        (&entities, &positions, &mut velocities, &mut stats).join()
                    {
                        // Check if it is a hit
                        if entity != b
                            && !stat_b.is_dead
                            && pos.0.distance_squared(pos_b.0) < 50.0
                            && ori.0.angle_between(pos_b.0 - pos.0).to_degrees() < 70.0
                        {
                            // Deal damage
                            stat_b
                                .health
                                .change_by(-10, HealthSource::Attack { by: *uid }); // TODO: variable damage and weapon
                            vel_b.0.z = 7.0;
                            let _ = force_updates.insert(b, ForceUpdate);
                        }
                    }
                    attacking.applied = true;
                }

                if attacking.time > 0.5 {
                    Some(entity)
                } else {
                    attacking.time += dt.0;

                    None
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|e| {
                attackings.remove(e);
            });
        {
            // Wields
            for wielding in (&mut wieldings).join() {
                if !wielding.applied && wielding.time > 0.3 {
                    wielding.applied = true;
                } else {
                    wielding.time += dt.0;
                }
            }
        }
    }
}
