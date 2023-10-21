use crate::{
    comp::{self, pet::is_mountable, ship::figuredata::VOXEL_COLLIDER_MANIFEST},
    link::{Is, Link, LinkHandle, Role},
    terrain::{Block, TerrainGrid},
    tether,
    uid::{IdMaps, Uid},
    vol::ReadVol,
};
use hashbrown::HashSet;
use serde::{Deserialize, Serialize};
use specs::{
    storage::GenericWriteStorage, Component, DenseVecStorage, Entities, Entity, Read, ReadExpect,
    ReadStorage, Write, WriteStorage,
};
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

#[derive(Debug)]
pub enum MountingError {
    NoSuchEntity,
    NotMountable,
}

impl Link for Mounting {
    type CreateData<'a> = (
        Read<'a, IdMaps>,
        WriteStorage<'a, Is<Mount>>,
        WriteStorage<'a, Is<Rider>>,
        ReadStorage<'a, Is<VolumeRider>>,
        ReadStorage<'a, Is<tether::Follower>>,
    );
    type DeleteData<'a> = (
        Read<'a, IdMaps>,
        WriteStorage<'a, Is<Mount>>,
        WriteStorage<'a, Is<Rider>>,
        WriteStorage<'a, comp::Pos>,
        WriteStorage<'a, comp::ForceUpdate>,
        ReadExpect<'a, TerrainGrid>,
    );
    type Error = MountingError;
    type PersistData<'a> = (
        Read<'a, IdMaps>,
        Entities<'a>,
        ReadStorage<'a, comp::Health>,
        ReadStorage<'a, comp::Body>,
        ReadStorage<'a, Is<Mount>>,
        ReadStorage<'a, Is<Rider>>,
        ReadStorage<'a, comp::CharacterState>,
    );

    fn create(
        this: &LinkHandle<Self>,
        (id_maps, is_mounts, is_riders, is_volume_rider, is_followers): &mut Self::CreateData<'_>,
    ) -> Result<(), Self::Error> {
        let entity = |uid: Uid| id_maps.uid_entity(uid);

        if this.mount == this.rider {
            // Forbid self-mounting
            Err(MountingError::NotMountable)
        } else if let Some((mount, rider)) = entity(this.mount).zip(entity(this.rider)) {
            // Ensure that neither mount or rider are already part of a mounting
            // relationship
            if !is_mounts.contains(mount)
                && !is_riders.contains(rider)
                && !is_followers.contains(rider)
                // TODO: Does this definitely prevent mount cycles?
                && (!is_mounts.contains(rider) || !is_riders.contains(mount))
                && !is_volume_rider.contains(rider)
            {
                let _ = is_mounts.insert(mount, this.make_role());
                let _ = is_riders.insert(rider, this.make_role());
                Ok(())
            } else {
                Err(MountingError::NotMountable)
            }
        } else {
            Err(MountingError::NoSuchEntity)
        }
    }

    fn persist(
        this: &LinkHandle<Self>,
        (id_maps, entities, healths, bodies, is_mounts, is_riders, character_states): &mut Self::PersistData<'_>,
    ) -> bool {
        let entity = |uid: Uid| id_maps.uid_entity(uid);

        if let Some((mount, rider)) = entity(this.mount).zip(entity(this.rider)) {
            let is_alive = |entity| {
                entities.is_alive(entity) && healths.get(entity).map_or(true, |h| !h.is_dead)
            };

            let is_in_ridable_state = character_states
                .get(mount)
                .map_or(false, |cs| !matches!(cs, comp::CharacterState::Roll(_)));

            // Ensure that both entities are alive and that they continue to be linked
            is_alive(mount)
                && is_alive(rider)
                && is_mounts.get(mount).is_some()
                && is_riders.get(rider).is_some()
                && bodies.get(mount).map_or(false, |mount_body| {
                    is_mountable(mount_body, bodies.get(rider))
                })
                && is_in_ridable_state
        } else {
            false
        }
    }

    fn delete(
        this: &LinkHandle<Self>,
        (id_maps, is_mounts, is_riders, positions, force_update, terrain): &mut Self::DeleteData<
            '_,
        >,
    ) {
        let entity = |uid: Uid| id_maps.uid_entity(uid);

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
                    .unwrap_or_else(|| terrain.find_ground(old_pos).map(|e| e as f32))
                    + Vec3::new(0.5, 0.5, 0.0);
                if let Some(force_update) = force_update.get_mut(rider) {
                    force_update.update();
                }
            });
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VolumeRider;

impl Role for VolumeRider {
    type Link = VolumeMounting;
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Volume<E> {
    Terrain,
    Entity(E),
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct VolumePos<E = Uid> {
    pub kind: Volume<E>,
    pub pos: Vec3<i32>,
}

impl<E> VolumePos<E> {
    pub fn terrain(block_pos: Vec3<i32>) -> Self {
        Self {
            kind: Volume::Terrain,
            pos: block_pos,
        }
    }

    pub fn entity(block_pos: Vec3<i32>, uid: E) -> Self {
        Self {
            kind: Volume::Entity(uid),
            pos: block_pos,
        }
    }

    pub fn try_map_entity<U>(self, f: impl FnOnce(E) -> Option<U>) -> Option<VolumePos<U>> {
        Some(VolumePos {
            pos: self.pos,
            kind: match self.kind {
                Volume::Terrain => Volume::Terrain,
                Volume::Entity(e) => Volume::Entity(f(e)?),
            },
        })
    }
}

impl VolumePos {
    /// Retrieves the block and matrix transformation for this `VolumeBlock`
    ///
    /// The transform is located in the blocks minimum position relative to the
    /// volume.
    pub fn get_block_and_transform(
        &self,
        terrain: &TerrainGrid,
        id_maps: &IdMaps,
        mut read_pos_and_ori: impl FnMut(Entity) -> Option<(comp::Pos, comp::Ori)>,
        colliders: &ReadStorage<comp::Collider>,
    ) -> Option<(Mat4<f32>, comp::Ori, Block)> {
        match self.kind {
            Volume::Terrain => Some((
                Mat4::translation_3d(self.pos.as_()),
                comp::Ori::default(),
                *terrain.get(self.pos).ok()?,
            )),
            Volume::Entity(uid) => id_maps.uid_entity(uid).and_then(|entity| {
                let collider = colliders.get(entity)?;
                let (pos, ori) = read_pos_and_ori(entity)?;

                let voxel_colliders_manifest = VOXEL_COLLIDER_MANIFEST.read();
                let voxel_collider = collider.get_vol(&voxel_colliders_manifest)?;

                let block = *voxel_collider.volume().get(self.pos).ok()?;

                let local_translation = voxel_collider.translation + self.pos.as_();

                let trans = Mat4::from(ori.to_quat()).translated_3d(pos.0)
                    * Mat4::<f32>::translation_3d(local_translation);

                Some((trans, ori, block))
            }),
        }
    }

    /// Get the block at this `VolumePos`.
    pub fn get_block(
        &self,
        terrain: &TerrainGrid,
        id_maps: &IdMaps,
        colliders: &ReadStorage<comp::Collider>,
    ) -> Option<Block> {
        match self.kind {
            Volume::Terrain => Some(*terrain.get(self.pos).ok()?),
            Volume::Entity(uid) => id_maps.uid_entity(uid).and_then(|entity| {
                let collider = colliders.get(entity)?;

                let voxel_colliders_manifest = VOXEL_COLLIDER_MANIFEST.read();
                let voxel_collider = collider.get_vol(&voxel_colliders_manifest)?;

                let block = *voxel_collider.volume().get(self.pos).ok()?;

                Some(block)
            }),
        }
    }
}

#[derive(Default)]
pub struct VolumeRiders {
    riders: HashSet<Vec3<i32>>,
}

impl Component for VolumeRiders {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VolumeMounting {
    pub pos: VolumePos,
    pub block: Block,
    pub rider: Uid,
}

impl Link for VolumeMounting {
    type CreateData<'a> = (
        Write<'a, VolumeRiders>,
        WriteStorage<'a, VolumeRiders>,
        WriteStorage<'a, Is<VolumeRider>>,
        ReadStorage<'a, Is<Rider>>,
        ReadExpect<'a, TerrainGrid>,
        Read<'a, IdMaps>,
        ReadStorage<'a, comp::Collider>,
    );
    type DeleteData<'a> = (
        Write<'a, VolumeRiders>,
        WriteStorage<'a, VolumeRiders>,
        WriteStorage<'a, Is<VolumeRider>>,
        Read<'a, IdMaps>,
    );
    type Error = MountingError;
    type PersistData<'a> = (
        Entities<'a>,
        ReadStorage<'a, comp::Health>,
        Read<'a, VolumeRiders>,
        ReadStorage<'a, VolumeRiders>,
        ReadStorage<'a, Is<VolumeRider>>,
        ReadExpect<'a, TerrainGrid>,
        Read<'a, IdMaps>,
        ReadStorage<'a, comp::Collider>,
    );

    fn create(
        this: &LinkHandle<Self>,
        (
            terrain_riders,
            volume_riders,
            is_volume_riders,
            is_riders,
            terrain_grid,
            id_maps,
            colliders,
        ): &mut Self::CreateData<'_>,
    ) -> Result<(), Self::Error> {
        let entity = |uid: Uid| id_maps.uid_entity(uid);

        let riders = match this.pos.kind {
            Volume::Terrain => &mut *terrain_riders,
            Volume::Entity(uid) => entity(uid)
                .and_then(|entity| volume_riders.get_mut_or_default(entity))
                .ok_or(MountingError::NoSuchEntity)?,
        };
        let rider = entity(this.rider).ok_or(MountingError::NoSuchEntity)?;

        if !riders.riders.contains(&this.pos.pos)
            && !is_volume_riders.contains(rider)
            && !is_volume_riders.contains(rider)
            && !is_riders.contains(rider)
        {
            let block = this
                .pos
                .get_block(terrain_grid, id_maps, colliders)
                .ok_or(MountingError::NoSuchEntity)?;

            if block == this.block {
                let _ = is_volume_riders.insert(rider, this.make_role());
                riders.riders.insert(this.pos.pos);
                Ok(())
            } else {
                Err(MountingError::NotMountable)
            }
        } else {
            Err(MountingError::NotMountable)
        }
    }

    fn persist(
        this: &LinkHandle<Self>,
        (
            entities,
            healths,
            terrain_riders,
            volume_riders,
            is_volume_riders,
            terrain_grid,
            id_maps,
            colliders,
        ): &mut Self::PersistData<'_>,
    ) -> bool {
        let entity = |uid: Uid| id_maps.uid_entity(uid);
        let is_alive =
            |entity| entities.is_alive(entity) && healths.get(entity).map_or(true, |h| !h.is_dead);
        let riders = match this.pos.kind {
            Volume::Terrain => &*terrain_riders,
            Volume::Entity(uid) => {
                let Some(riders) = entity(uid)
                    .filter(|entity| is_alive(*entity))
                    .and_then(|entity| volume_riders.get(entity))
                else {
                    return false;
                };
                riders
            },
        };

        let rider_exists = entity(this.rider).map_or(false, |rider| {
            is_volume_riders.contains(rider) && is_alive(rider)
        });
        let mount_spot_exists = riders.riders.contains(&this.pos.pos);

        let block_exists = this
            .pos
            .get_block(terrain_grid, id_maps, colliders)
            .map_or(false, |block| block == this.block);

        rider_exists && mount_spot_exists && block_exists
    }

    fn delete(
        this: &LinkHandle<Self>,
        (terrain_riders, volume_riders, is_rider, id_maps): &mut Self::DeleteData<'_>,
    ) {
        let entity = |uid: Uid| id_maps.uid_entity(uid);

        let riders = match this.pos.kind {
            Volume::Terrain => Some(&mut **terrain_riders),
            Volume::Entity(uid) => {
                entity(uid).and_then(|entity| volume_riders.get_mut_or_default(entity))
            },
        };

        if let Some(riders) = riders {
            riders.riders.remove(&this.pos.pos);
        }

        if let Some(entity) = entity(this.rider) {
            is_rider.remove(entity);
        }
    }
}
