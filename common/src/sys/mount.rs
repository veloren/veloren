use crate::{
    comp::{Controller, MountState, Mounting, Ori, Pos, Vel},
    sync::UidAllocator,
};
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entities, Join, Read, System, WriteStorage,
};
use vek::*;

/// This system is responsible for controlling mounts
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
    );

    fn run(
        &mut self,
        (
            uid_allocator,
            entities,
            mut controllers,
            mut mount_state,
            mut mountings,
            mut positions,
            mut velocities,
            mut orientations,
        ): Self::SystemData,
    ) {
        // Mounted entities.
        for (entity, mut mount_states) in (&entities, &mut mount_state.restrict_mut()).join() {
            match mount_states.get_unchecked() {
                MountState::Unmounted => {},
                MountState::MountedBy(mounter_uid) => {
                    // Note: currently controller events are not passed through since none of them
                    // are currently relevant to controlling the mounted entity
                    if let Some((inputs, mounter)) = uid_allocator
                        .retrieve_entity_internal(mounter_uid.id())
                        .and_then(|mounter| {
                            controllers
                                .get(mounter)
                                .map(|c| (c.inputs.clone(), mounter))
                        })
                    {
                        // TODO: consider joining on these? (remember we can use .maybe())
                        let pos = positions.get(entity).copied();
                        let ori = orientations.get(entity).copied();
                        let vel = velocities.get(entity).copied();
                        if let (Some(pos), Some(ori), Some(vel)) = (pos, ori, vel) {
                            let _ = positions.insert(mounter, Pos(pos.0 + Vec3::unit_z() * 1.0));
                            let _ = orientations.insert(mounter, ori);
                            let _ = velocities.insert(mounter, vel);
                        }
                        controllers.get_mut(entity).map(|controller| {
                            *controller = Controller {
                                inputs,
                                ..Default::default()
                            }
                        });
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
