use std::sync::Arc;

use common::{
    comp::{self, pet::is_mountable},
    consts::{MAX_MOUNT_RANGE, MAX_SPRITE_MOUNT_RANGE},
    event::{MountEvent, MountVolumeEvent, UnmountEvent},
    link::Is,
    mounting::{Mounting, Rider, VolumeMounting, VolumeRider},
    rtsim::RtSimEntity,
    uid::IdMaps,
};
use plugin_api::Uid;
use specs::WorldExt;

use crate::{rtsim::RtSim, state_ext::StateExt, Server};

pub fn within_mounting_range(
    player_position: Option<&comp::Pos>,
    mount_position: Option<&comp::Pos>,
) -> bool {
    match (player_position, mount_position) {
        (Some(ppos), Some(ipos)) => ppos.0.distance_squared(ipos.0) < MAX_MOUNT_RANGE.powi(2),
        _ => false,
    }
}

pub fn handle_mount(server: &mut Server, MountEvent(rider, mount): MountEvent) {
    let state = server.state_mut();

    let within_range = {
        let positions = state.ecs().read_storage::<comp::Pos>();
        within_mounting_range(positions.get(rider), positions.get(mount))
    };

    if within_range {
        let uids = state.ecs().read_storage::<Uid>();
        if let (Some(rider_uid), Some(mount_uid)) =
            (uids.get(rider).copied(), uids.get(mount).copied())
        {
            let is_pet_of = |mount, rider_uid| {
                matches!(
                    state
                        .ecs()
                        .read_storage::<comp::Alignment>()
                        .get(mount),
                    Some(comp::Alignment::Owned(owner)) if *owner == rider_uid,
                )
            };

            let can_ride = state
                .ecs()
                .read_storage()
                .get(mount)
                .map_or(false, |mount_body| {
                    is_mountable(mount_body, state.ecs().read_storage().get(rider))
                });

            let is_stay = state
                .ecs()
                .read_storage::<comp::Agent>()
                .get(mount)
                .and_then(|x| x.stay_pos)
                .is_some();

            if (is_pet_of(mount, rider_uid) || is_pet_of(rider, mount_uid)) && can_ride && !is_stay
            {
                drop(uids);
                let _ = state.link(Mounting {
                    mount: mount_uid,
                    rider: rider_uid,
                });
            }
        }
    }
}

pub fn handle_mount_volume(
    server: &mut Server,
    MountVolumeEvent(rider, volume_pos): MountVolumeEvent,
) {
    let state = server.state_mut();

    let block_transform = volume_pos.get_block_and_transform(
        &state.terrain(),
        &state.ecs().read_resource(),
        |e| {
            state
                .read_storage()
                .get(e)
                .copied()
                .zip(state.read_storage().get(e).copied())
        },
        &state.read_storage(),
    );

    if let Some((mat, _, block)) = block_transform
    && let Some(mount_offset) = block.mount_offset() {
        let mount_pos = (mat * mount_offset.0.with_w(1.0)).xyz();
        let within_range = {
            let positions = state.ecs().read_storage::<comp::Pos>();
            positions.get(rider).map_or(false, |pos| pos.0.distance_squared(mount_pos) < MAX_SPRITE_MOUNT_RANGE.powi(2))
        };

        let maybe_uid = state.ecs().read_storage::<Uid>().get(rider).copied();

        if let Some(rider) = maybe_uid && within_range {
            let _link_successful = state.link(VolumeMounting {
                pos: volume_pos,
                block,
                rider,
            }).is_ok();
            #[cfg(feature = "worldgen")]
            if _link_successful {
                let uid_allocator = state.ecs().read_resource::<IdMaps>();
                if let Some(rider_entity) = uid_allocator.uid_entity(rider)
                    && let Some(rider_actor) = state.entity_as_actor(rider_entity)
                    && let Some(volume_pos) = volume_pos.try_map_entity(|uid| {
                        let entity = uid_allocator.uid_entity(uid)?;
                        state.read_storage::<RtSimEntity>().get(entity).map(|v| v.0)
                    }) {
                    state.ecs().write_resource::<RtSim>().hook_character_mount_volume(
                            &state.ecs().read_resource::<Arc<world::World>>(),
                            state.ecs().read_resource::<world::IndexOwned>().as_index_ref(),
                            volume_pos,
                            rider_actor,
                    );
                }
            }
        }
    }
}

pub fn handle_unmount(server: &mut Server, UnmountEvent(rider): UnmountEvent) {
    let state = server.state_mut();
    state.ecs().write_storage::<Is<Rider>>().remove(rider);
    state.ecs().write_storage::<Is<VolumeRider>>().remove(rider);
}
