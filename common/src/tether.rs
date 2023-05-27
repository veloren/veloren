use crate::{
    comp,
    link::{Is, Link, LinkHandle, Role},
    mounting::{Rider, VolumeRider},
    uid::{Uid, UidAllocator},
};
use serde::{Deserialize, Serialize};
use specs::{saveload::MarkerAllocator, Entities, Read, ReadStorage, WriteStorage};
use vek::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct Leader;

impl Role for Leader {
    type Link = Tethered;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Follower;

impl Role for Follower {
    type Link = Tethered;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tethered {
    pub leader: Uid,
    pub follower: Uid,
    pub tether_length: f32,
}

#[derive(Debug)]
pub enum TetherError {
    NoSuchEntity,
    NotTetherable,
}

impl Link for Tethered {
    type CreateData<'a> = (
        Read<'a, UidAllocator>,
        WriteStorage<'a, Is<Leader>>,
        WriteStorage<'a, Is<Follower>>,
        ReadStorage<'a, Is<Rider>>,
        ReadStorage<'a, Is<VolumeRider>>,
    );
    type DeleteData<'a> = (
        Read<'a, UidAllocator>,
        WriteStorage<'a, Is<Leader>>,
        WriteStorage<'a, Is<Follower>>,
    );
    type Error = TetherError;
    type PersistData<'a> = (
        Read<'a, UidAllocator>,
        Entities<'a>,
        ReadStorage<'a, comp::Health>,
        ReadStorage<'a, Is<Leader>>,
        ReadStorage<'a, Is<Follower>>,
    );

    fn create(
        this: &LinkHandle<Self>,
        (
            uid_allocator,
            is_leaders,
            is_followers,
            is_riders,
            is_volume_rider,
        ): &mut Self::CreateData<'_>,
    ) -> Result<(), Self::Error> {
        let entity = |uid: Uid| uid_allocator.retrieve_entity_internal(uid.into());

        if this.leader == this.follower {
            // Forbid self-tethering
            Err(TetherError::NotTetherable)
        } else if let Some((leader, follower)) = entity(this.leader).zip(entity(this.follower)) {
            // Ensure that neither leader or follower are already part of a conflicting
            // relationship
            if !is_riders.contains(follower)
                && !is_volume_rider.contains(follower)
                && !is_followers.contains(follower)
                // TODO: Does this definitely prevent tether cycles?
                && (!is_leaders.contains(follower) || !is_followers.contains(leader))
            {
                let _ = is_leaders.insert(leader, this.make_role());
                let _ = is_followers.insert(follower, this.make_role());
                Ok(())
            } else {
                Err(TetherError::NotTetherable)
            }
        } else {
            Err(TetherError::NoSuchEntity)
        }
    }

    fn persist(
        this: &LinkHandle<Self>,
        (uid_allocator, entities, healths, is_leaders, is_followers): &mut Self::PersistData<'_>,
    ) -> bool {
        let entity = |uid: Uid| uid_allocator.retrieve_entity_internal(uid.into());

        if let Some((leader, follower)) = entity(this.leader).zip(entity(this.follower)) {
            let is_alive = |entity| {
                entities.is_alive(entity) && healths.get(entity).map_or(true, |h| !h.is_dead)
            };

            // Ensure that both entities are alive and that they continue to be linked
            is_alive(leader)
                && is_alive(follower)
                && is_leaders.get(leader).is_some()
                && is_followers.get(follower).is_some()
        } else {
            false
        }
    }

    fn delete(
        this: &LinkHandle<Self>,
        (uid_allocator, is_leaders, is_followers): &mut Self::DeleteData<'_>,
    ) {
        let entity = |uid: Uid| uid_allocator.retrieve_entity_internal(uid.into());

        let leader = entity(this.leader);
        let follower = entity(this.follower);

        // Delete link components
        leader.map(|leader| is_leaders.remove(leader));
        follower.map(|follower| is_followers.remove(follower));
    }
}
