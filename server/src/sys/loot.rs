use common::{
    comp::{group::GroupManager, loot_owner::LootOwnerKind, LootOwner},
    uid::UidAllocator,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{saveload::MarkerAllocator, Entities, Entity, Join, Read, WriteStorage};
use tracing::debug;

// This system manages loot that exists in the world
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, LootOwner>,
        Read<'a, UidAllocator>,
        Read<'a, GroupManager>,
    );

    const NAME: &'static str = "loot";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (entities, mut loot_owners, uid_allocator, group_manager): Self::SystemData,
    ) {
        // Find and remove expired loot ownership. Loot ownership is expired when either
        // the expiry time has passed, or the owner no longer exists
        let expired = (&entities, &loot_owners)
            .join()
            .filter(|(_, loot_owner)| {
                loot_owner.expired()
                    || match loot_owner.owner() {
                        LootOwnerKind::Player(uid) => uid_allocator
                            .retrieve_entity_internal(uid.into())
                            .map_or(true, |entity| !entities.is_alive(entity)),
                        LootOwnerKind::Group(group) => group_manager.group_info(group).is_none(),
                    }
            })
            .map(|(entity, _)| entity)
            .collect::<Vec<Entity>>();

        if !&expired.is_empty() {
            debug!("Removing {} expired loot ownerships", expired.iter().len());
        }

        for entity in expired {
            loot_owners.remove(entity);
        }
    }
}
