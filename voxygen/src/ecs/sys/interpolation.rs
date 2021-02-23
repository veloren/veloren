use crate::ecs::comp::Interpolated;
use common::{
    comp::{object, Body, Ori, Pos, Vel},
    resources::DeltaTime,
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
        ReadStorage<'a, Body>,
        WriteStorage<'a, Interpolated>,
    );

    fn run(
        &mut self,
        (entities, dt, positions, orientations, velocities, bodies, mut interpolated): Self::SystemData,
    ) {
        // Update interpolated positions and orientations
        for (pos, ori, i, body, vel) in (
            &positions,
            &orientations,
            &mut interpolated,
            &bodies,
            &velocities,
        )
            .join()
        {
            // Update interpolation values
            if i.pos.distance_squared(pos.0) < 64.0 * 64.0 {
                i.pos = Lerp::lerp(i.pos, pos.0 + vel.0 * 0.03, 10.0 * dt.0);
                i.ori = Ori::slerp(i.ori, *ori, base_ori_interp(body) * dt.0);
            } else {
                i.pos = pos.0;
                i.ori = *ori;
            }
        }
        // Insert interpolation components for entities which don't have them
        for (entity, pos, ori) in (&entities, &positions, &orientations, !&interpolated)
            .join()
            .map(|(e, p, o, _)| (e, p.0, *o))
            .collect::<Vec<_>>()
        {
            interpolated
                .insert(entity, Interpolated { pos, ori })
                .err()
                .map(|e| warn!(?e, "Error inserting Interpolated component"));
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

#[allow(clippy::collapsible_match)]
fn base_ori_interp(body: &Body) -> f32 {
    match body {
        Body::Object(object) => match object {
            object::Body::Crossbow => 100.0,
            _ => 10.0,
        },
        _ => 10.0,
    }
}
