use common::{
    comp::{group::GroupManager, loot_owner::LootOwnerKind, LootOwner},
    uid::IdMaps,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Entities, Entity, Join, Read, WriteStorage};
use tracing::debug;

// This system manages loot that exists in the world
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, LootOwner>,
        Read<'a, IdMaps>,
        Read<'a, GroupManager>,
    );

    const NAME: &'static str = "loot";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (entities, mut loot_owners, id_maps, group_manager): Self::SystemData,
    ) {
        // Find and remove expired loot ownership. Loot ownership is expired when either
        // the expiry time has passed, or the owner no longer exists
        let expired = (&entities, &loot_owners)
            .join()
            .filter(|(_, loot_owner)| {
                loot_owner.expired()
                    || match loot_owner.owner() {
                        LootOwnerKind::Player(uid) => id_maps
                            .uid_entity(uid)
                            .map_or(true, |entity| !entities.is_alive(entity)),
                        LootOwnerKind::Group(group) => {
                            // Special alignment groups (NPC and ENEMY) aren't tracked by the group
                            // manager, check them separately here
                            !group.is_special() && group_manager.group_info(group).is_none()
                        },
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
