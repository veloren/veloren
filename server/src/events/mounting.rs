#[cfg(feature = "worldgen")] use std::sync::Arc;

use common::{
    comp::{self, pet::is_mountable},
    consts::{MAX_MOUNT_RANGE, MAX_SPRITE_MOUNT_RANGE},
    event::MountEvent,
    link::Is,
    mounting::{Mounting, Rider, VolumeMounting, VolumePos, VolumeRider},
    uid::Uid,
};
#[cfg(feature = "worldgen")]
use common::{rtsim::RtSimEntity, uid::IdMaps};
use specs::{Entity as EcsEntity, WorldExt};
use vek::Vec3;

#[cfg(feature = "worldgen")]
use crate::rtsim::RtSim;
use crate::{Server, state_ext::StateExt};

pub fn within_mounting_range(
    player_position: Option<&comp::Pos>,
    mount_position: Option<&comp::Pos>,
) -> bool {
    match (player_position, mount_position) {
        (Some(ppos), Some(ipos)) => ppos.0.distance_squared(ipos.0) < MAX_MOUNT_RANGE.powi(2),
        _ => false,
    }
}

pub fn handle_mount(server: &mut Server, event: MountEvent) {
    match event {
        MountEvent::MountEntity(rider, mount) => handle_mount_entity(server, rider, mount),
        MountEvent::MountVolume(rider, mount) => handle_mount_volume(server, rider, mount),
        MountEvent::Unmount(rider) => handle_unmount(server, rider),
    }
}

fn handle_mount_entity(server: &mut Server, rider: EcsEntity, mount: EcsEntity) {
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
                .zip(state.ecs().read_storage().get(mount))
                .is_some_and(|(mount_body, mount_mass)| {
                    is_mountable(
                        mount_body,
                        mount_mass,
                        state.ecs().read_storage().get(rider),
                        state.ecs().read_storage().get(rider),
                    )
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

fn handle_mount_volume(server: &mut Server, rider: EcsEntity, volume_pos: VolumePos) {
    let state = server.state_mut();

    let mount_mat = volume_pos.get_mount_mat(
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

    if let Some((mat, block)) = mount_mat {
        let mount_pos = mat.mul_point(Vec3::zero());
        let within_range = {
            let positions = state.ecs().read_storage::<comp::Pos>();
            positions.get(rider).is_some_and(|pos| {
                pos.0.distance_squared(mount_pos) < MAX_SPRITE_MOUNT_RANGE.powi(2)
            })
        };

        let maybe_uid = state.ecs().read_storage::<Uid>().get(rider).copied();

        if let Some(rider_uid) = maybe_uid
            && within_range
        {
            let _link_successful = state
                .link(VolumeMounting {
                    pos: volume_pos,
                    block,
                    rider: rider_uid,
                })
                .is_ok();
            #[cfg(feature = "worldgen")]
            if _link_successful {
                let uid_allocator = state.ecs().read_resource::<IdMaps>();
                if let Some(rider_entity) = uid_allocator.uid_entity(rider_uid)
                    && let Some(rider_actor) = state.entity_as_actor(rider_entity)
                    && let Some(volume_pos) = volume_pos.try_map_entity(|uid| {
                        let entity = uid_allocator.uid_entity(uid)?;
                        state.read_storage::<RtSimEntity>().get(entity).copied()
                    })
                {
                    state
                        .ecs()
                        .write_resource::<RtSim>()
                        .hook_character_mount_volume(
                            &state.ecs().read_resource::<Arc<world::World>>(),
                            state
                                .ecs()
                                .read_resource::<world::IndexOwned>()
                                .as_index_ref(),
                            volume_pos,
                            rider_actor,
                        );
                }
            }
        }
    }
}

fn handle_unmount(server: &mut Server, rider: EcsEntity) {
    let state = server.state_mut();
    state.ecs().write_storage::<Is<Rider>>().remove(rider);
    state.ecs().write_storage::<Is<VolumeRider>>().remove(rider);
}
