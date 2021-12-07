use crate::ecs::comp::Interpolated;
use common::{
    comp::{object, Body, Ori, Pos, Vel},
    resources::DeltaTime,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Entities, Join, Read, ReadStorage, WriteStorage};
use tracing::warn;
use vek::*;

/// This system will allow NPCs to modify their controller
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, Body>,
        WriteStorage<'a, Interpolated>,
    );

    const NAME: &'static str = "interpolation";
    const ORIGIN: Origin = Origin::Frontend("voxygen");
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
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
            // Update interpolation values, but don't interpolate far things or objects
            if i.pos.distance_squared(pos.0) < 64.0 * 64.0 && !matches!(body, Body::Object(_)) {
                // Note, these values are specifically tuned for smoother motion with high
                // network latency or low network sampling rate and for smooth
                // block hopping (which is instantaneous)
                const POS_LERP_RATE_FACTOR: f32 = 10.0;
                i.pos = Lerp::lerp(i.pos, pos.0 + vel.0 * 0.03, POS_LERP_RATE_FACTOR * dt.0);
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

fn base_ori_interp(body: &Body) -> f32 {
    match body {
        Body::Object(object) => match object {
            object::Body::Crossbow | object::Body::HaniwaSentry => 100.0,
            _ => 10.0,
        },
        _ => 10.0,
    }
}
