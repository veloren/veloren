use crate::{
    comp::{ActionState::*, CharacterState, ForceUpdate, HealthSource, Ori, Pos, Stats, Vel},
    state::{DeltaTime, Uid},
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use std::time::Duration;

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
        WriteStorage<'a, CharacterState>,
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
            mut character_states,
            mut stats,
            mut force_updates,
        ): Self::SystemData,
    ) {
        // Attacks
        for (entity, uid, pos, ori, mut character) in (
            &entities,
            &uids,
            &positions,
            &orientations,
            &mut character_states,
        )
            .join()
        {
            let mut todo_end = false;

            // Go through all other entities
            if let Attack { time_left, applied } = &mut character.action {
                if !*applied {
                    for (b, pos_b, mut vel_b, stat_b) in
                        (&entities, &positions, &mut velocities, &mut stats).join()
                    {
                        // Check if it is a hit
                        if entity != b
                            && !stat_b.is_dead
                            && pos.0.distance_squared(pos_b.0) < 50.0
                            && ori.0.angle_between(pos_b.0 - pos.0).to_degrees() < 90.0
                        {
                            // Deal damage
                            stat_b
                                .health
                                .change_by(-10, HealthSource::Attack { by: *uid }); // TODO: variable damage and weapon
                            vel_b.0 += (pos_b.0 - pos.0).normalized() * 2.0;
                            vel_b.0.z = 2.0;
                            let _ = force_updates.insert(b, ForceUpdate);
                        }
                    }
                    *applied = true;
                }

                if *time_left == Duration::default() {
                    todo_end = true;
                } else {
                    *time_left = time_left
                        .checked_sub(Duration::from_secs_f32(dt.0))
                        .unwrap_or_default();
                }
            }
            if todo_end {
                character.action = Wield {
                    time_left: Duration::default(),
                };
            }

            if let Wield { time_left } = &mut character.action {
                if *time_left != Duration::default() {
                    *time_left = time_left
                        .checked_sub(Duration::from_secs_f32(dt.0))
                        .unwrap_or_default();
                }
            }
        }
    }
}
