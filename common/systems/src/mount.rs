use common::{
    comp::{Body, Controller, MountState, Mounting, Ori, Pos, Vel},
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
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Read<'a, UidAllocator>,
        Entities<'a>,
        WriteStorage<'a, Controller>,
        WriteStorage<'a, MountState>,
        WriteStorage<'a, Mounting>,
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
            mut mount_state,
            mut mountings,
            mut positions,
            mut velocities,
            mut orientations,
            bodies,
        ): Self::SystemData,
    ) {
        // Mounted entities.
        for (entity, mut mount_states, body) in
            (&entities, &mut mount_state.restrict_mut(), bodies.maybe()).join()
        {
            match mount_states.get_unchecked() {
                MountState::Unmounted => {},
                MountState::MountedBy(mounter_uid) => {
                    // Note: currently controller events are not passed through since none of them
                    // are currently relevant to controlling the mounted entity
                    if let Some((inputs, queued_inputs, mounter)) = uid_allocator
                        .retrieve_entity_internal(mounter_uid.id())
                        .and_then(|mounter| {
                            controllers
                                .get(mounter)
                                .map(|c| (c.inputs.clone(), c.queued_inputs.clone(), mounter))
                        })
                    {
                        // TODO: consider joining on these? (remember we can use .maybe())
                        let pos = positions.get(entity).copied();
                        let ori = orientations.get(entity).copied();
                        let vel = velocities.get(entity).copied();
                        if let (Some(pos), Some(ori), Some(vel)) = (pos, ori, vel) {
                            let mounting_offset =
                                body.map_or(Vec3::unit_z(), Body::mounting_offset);
                            let _ = positions.insert(mounter, Pos(pos.0 + mounting_offset));
                            let _ = orientations.insert(mounter, ori);
                            let _ = velocities.insert(mounter, vel);
                        }
                        if let Some(controller) = controllers.get_mut(entity) {
                            *controller = Controller {
                                inputs,
                                queued_inputs,
                                ..Default::default()
                            }
                        }
                    } else {
                        *(mount_states.get_mut_unchecked()) = MountState::Unmounted;
                    }
                },
            }
        }

        let mut to_unmount = Vec::new();
        for (entity, Mounting(mountee_uid)) in (&entities, &mountings).join() {
            if uid_allocator
                .retrieve_entity_internal(mountee_uid.id())
                .filter(|mountee| entities.is_alive(*mountee))
                .is_none()
            {
                to_unmount.push(entity);
            }
        }
        for entity in to_unmount {
            mountings.remove(entity);
        }
    }
}
