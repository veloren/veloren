use common::{
    comp::{Body, Controller, InputKind, Ori, Pos, Vel},
    link::Is,
    mounting::Mount,
    uid::UidAllocator,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entities, Join, Read, ReadStorage, WriteStorage,
};
use vek::*;

/// This system is responsible for controlling mounts
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, UidAllocator>,
        Entities<'a>,
        WriteStorage<'a, Controller>,
        ReadStorage<'a, Is<Mount>>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        ReadStorage<'a, Body>,
    );

    const NAME: &'static str = "mount";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            uid_allocator,
            entities,
            mut controllers,
            is_mounts,
            mut positions,
            mut velocities,
            mut orientations,
            bodies,
        ): Self::SystemData,
    ) {
        // For each mount...
        for (entity, is_mount, body) in (&entities, &is_mounts, bodies.maybe()).join() {
            // ...find the rider...
            let Some((inputs, queued_inputs, rider)) = uid_allocator
                .retrieve_entity_internal(is_mount.rider.id())
                .and_then(|rider| {
                    controllers
                        .get_mut(rider)
                        .map(|c| {
                            let queued_inputs = c.queued_inputs
                                // TODO: Formalise ways to pass inputs to mounts
                                .drain_filter(|i, _| matches!(i, InputKind::Jump | InputKind::Fly | InputKind::Roll))
                                .collect();
                            (c.inputs.clone(), queued_inputs, rider)
                        })
                })
            else { continue };

            // ...apply the mount's position/ori/velocity to the rider...
            let pos = positions.get(entity).copied();
            let ori = orientations.get(entity).copied();
            let vel = velocities.get(entity).copied();
            if let (Some(pos), Some(ori), Some(vel)) = (pos, ori, vel) {
                let mounter_body = bodies.get(rider);
                let mounting_offset = body.map_or(Vec3::unit_z(), Body::mount_offset)
                    + mounter_body.map_or(Vec3::zero(), Body::rider_offset);
                let _ = positions.insert(rider, Pos(pos.0 + ori.to_quat() * mounting_offset));
                let _ = orientations.insert(rider, ori);
                let _ = velocities.insert(rider, vel);
            }
            // ...and apply the rider's inputs to the mount's controller.
            if let Some(controller) = controllers.get_mut(entity) {
                *controller = Controller {
                    inputs,
                    queued_inputs,
                    ..Default::default()
                }
            }
        }
    }
}
