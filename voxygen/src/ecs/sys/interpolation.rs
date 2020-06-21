use crate::ecs::comp::Interpolated;
use common::{
    comp::{Ori, Pos, Vel},
    state::DeltaTime,
    util::Dir,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use tracing::warn;
use vek::*;

/// This system will allow NPCs to modify their controller
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)] // TODO: Pending review in #587
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, Vel>,
        WriteStorage<'a, Interpolated>,
    );

    fn run(
        &mut self,
        (entities, dt, positions, orientations, velocities, mut interpolated): Self::SystemData,
    ) {
        // Update interpolated positions and orientations
        for (pos, ori, i, vel) in (&positions, &orientations, &mut interpolated, &velocities).join()
        {
            // Update interpolation values
            if i.pos.distance_squared(pos.0) < 64.0 * 64.0 {
                i.pos = Lerp::lerp(i.pos, pos.0 + vel.0 * 0.03, 10.0 * dt.0);
                i.ori = Dir::slerp(i.ori, ori.0, 5.0 * dt.0);
            } else {
                i.pos = pos.0;
                i.ori = ori.0;
            }
        }
        // Insert interpolation components for entities which don't have them
        for (entity, pos, ori) in (&entities, &positions, &orientations, !&interpolated)
            .join()
            .map(|(e, p, o, _)| (e, p.0, o.0))
            .collect::<Vec<_>>()
        {
            interpolated
                .insert(entity, Interpolated { pos, ori })
                .err()
                .map(|err| warn!("Error inserting Interpolated component: {}", err));
        }
        // Remove Interpolated component from entities which don't have a position or an
        // orientation or a velocity
        for entity in (&entities, !&positions, &interpolated)
            .join()
            .map(|(e, _, _)| e)
            .collect::<Vec<_>>()
        {
            interpolated.remove(entity);
        }
        for entity in (&entities, !&orientations, &interpolated)
            .join()
            .map(|(e, _, _)| e)
            .collect::<Vec<_>>()
        {
            interpolated.remove(entity);
        }
        for entity in (&entities, !&velocities, &interpolated)
            .join()
            .map(|(e, _, _)| e)
            .collect::<Vec<_>>()
        {
            interpolated.remove(entity);
        }
    }
}
