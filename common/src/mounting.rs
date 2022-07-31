use crate::{
    comp,
    comp::{pet::is_mountable, Body},
    link::{Is, Link, LinkHandle, Role},
    terrain::TerrainGrid,
    uid::{Uid, UidAllocator},
};
use serde::{Deserialize, Serialize};
use specs::{saveload::MarkerAllocator, Entities, Read, ReadExpect, ReadStorage, WriteStorage};
use vek::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct Rider;

impl Role for Rider {
    type Link = Mounting;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Mount;

impl Role for Mount {
    type Link = Mounting;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Mounting {
    pub mount: Uid,
    pub rider: Uid,
}

pub enum MountingError {
    NoSuchEntity,
    NotMountable,
}

impl Link for Mounting {
    type CreateData<'a> = (
        Read<'a, UidAllocator>,
        WriteStorage<'a, Is<Mount>>,
        WriteStorage<'a, Is<Rider>>,
        WriteStorage<'a, Body>,
    );
    type DeleteData<'a> = (
        Read<'a, UidAllocator>,
        WriteStorage<'a, Is<Mount>>,
        WriteStorage<'a, Is<Rider>>,
        WriteStorage<'a, comp::Pos>,
        WriteStorage<'a, comp::ForceUpdate>,
        ReadExpect<'a, TerrainGrid>,
    );
    type Error = MountingError;
    type PersistData<'a> = (
        Read<'a, UidAllocator>,
        Entities<'a>,
        ReadStorage<'a, comp::Health>,
        ReadStorage<'a, Is<Mount>>,
        ReadStorage<'a, Is<Rider>>,
    );

    fn create(
        this: &LinkHandle<Self>,
        (uid_allocator, mut is_mounts, mut is_riders, body): Self::CreateData<'_>,
    ) -> Result<(), Self::Error> {
        let entity = |uid: Uid| uid_allocator.retrieve_entity_internal(uid.into());

        if this.mount == this.rider {
            // Forbid self-mounting
            Err(MountingError::NotMountable)
        } else if let Some((mount, rider)) = entity(this.mount).zip(entity(this.rider)) {
            if let Some(mount_body) = body.get(mount) {
                if is_mountable(mount_body, body.get(rider)) {
                    let can_mount_with =
                        |entity| is_mounts.get(entity).is_none() && is_riders.get(entity).is_none();

                    // Ensure that neither mount or rider are already part of a mounting
                    // relationship
                    if can_mount_with(mount) && can_mount_with(rider) {
                        let _ = is_mounts.insert(mount, this.make_role());
                        let _ = is_riders.insert(rider, this.make_role());
                        Ok(())
                    } else {
                        Err(MountingError::NotMountable)
                    }
                } else {
                    Err(MountingError::NotMountable)
                }
            } else {
                Err(MountingError::NotMountable)
            }
        } else {
            Err(MountingError::NoSuchEntity)
        }
    }

    fn persist(
        this: &LinkHandle<Self>,
        (uid_allocator, entities, healths, is_mounts, is_riders): Self::PersistData<'_>,
    ) -> bool {
        let entity = |uid: Uid| uid_allocator.retrieve_entity_internal(uid.into());

        if let Some((mount, rider)) = entity(this.mount).zip(entity(this.rider)) {
            let is_alive = |entity| {
                entities.is_alive(entity) && healths.get(entity).map_or(true, |h| !h.is_dead)
            };

            // Ensure that both entities are alive and that they continue to be linked
            is_alive(mount)
                && is_alive(rider)
                && is_mounts.get(mount).is_some()
                && is_riders.get(rider).is_some()
        } else {
            false
        }
    }

    fn delete(
        this: &LinkHandle<Self>,
        (uid_allocator, mut is_mounts, mut is_riders, mut positions, mut force_update, terrain): Self::DeleteData<'_>,
    ) {
        let entity = |uid: Uid| uid_allocator.retrieve_entity_internal(uid.into());

        let mount = entity(this.mount);
        let rider = entity(this.rider);

        // Delete link components
        mount.map(|mount| is_mounts.remove(mount));
        rider.map(|rider| is_riders.remove(rider));

        // Try to move the rider to a safe place when dismounting
        let safe_pos = rider
            .and_then(|rider| positions.get(rider).copied())
            .filter(|rider_pos| terrain.is_space(rider_pos.0.map(|e| e.floor() as i32)))
            .or_else(|| {
                mount
                    .and_then(|mount| positions.get(mount).copied())
                    .filter(|mount_pos| {
                        terrain.is_space(
                            (mount_pos.0 + Vec3::unit_z() * 0.1).map(|e| e.floor() as i32),
                        )
                    })
            });
        rider
            .and_then(|rider| Some(rider).zip(positions.get_mut(rider)))
            .map(|(rider, pos)| {
                let old_pos = pos.0.map(|e| e.floor() as i32);
                pos.0 = safe_pos
                    .map(|p| p.0.map(|e| e.floor()))
                    .unwrap_or_else(|| terrain.find_space(old_pos).map(|e| e as f32))
                    + Vec3::new(0.5, 0.5, 0.0);
                if let Some(force_update) = force_update.get_mut(rider) {
                    force_update.update();
                }
            });
    }
}
