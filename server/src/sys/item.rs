use std::collections::HashMap;

use common::{
    comp,
    event::{DeleteEvent, EventBus},
    resources::ProgramTime,
    CachedSpatialGrid,
};
use common_ecs::{Origin, Phase, System};
use specs::{Entities, Entity, Join, LendJoin, Read, ReadStorage, WriteStorage};

const MAX_ITEM_MERGE_DIST: f32 = 2.0;
const CHECKS_PER_SECOND: f64 = 10.0; // Start by checking an item 10 times every second

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, comp::PickupItem>,
        ReadStorage<'a, comp::Pos>,
        ReadStorage<'a, comp::LootOwner>,
        Read<'a, CachedSpatialGrid>,
        Read<'a, ProgramTime>,
        Read<'a, EventBus<DeleteEvent>>,
    );

    const NAME: &'static str = "item";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut common_ecs::Job<Self>,
        (
            entities,
            mut items,
            positions,
            loot_owners,
            spatial_grid,
            program_time,
            delete_bus,
        ): Self::SystemData,
    ) {
        // Contains items that have been checked for merge, or that were merged into
        // another one
        let mut merged = HashMap::new();
        // Contains merges that will be performed (from, into)
        let mut merges = Vec::new();
        // Delete events are emitted when this is dropped
        let mut delete_emitter = delete_bus.emitter();

        for (entity, item, pos, loot_owner) in
            (&entities, &items, &positions, loot_owners.maybe()).join()
        {
            // Do not process items that are already being merged
            if merged.contains_key(&entity) {
                continue;
            }

            // Exponentially back of the frequency at which items are checked for merge
            if program_time.0 < item.next_merge_check().0 {
                continue;
            }

            // We do not want to allow merging this item if it isn't already being
            // merged into another
            merged.insert(entity, true);

            for (source_entity, _) in get_nearby_mergeable_items(
                item,
                pos,
                loot_owner,
                (&entities, &items, &positions, &loot_owners, &spatial_grid),
            ) {
                // Prevent merging an item multiple times, we cannot
                // do this in the above filter since we mutate `merged` below
                if merged.contains_key(&source_entity) {
                    continue;
                }

                // Do not merge items multiple times
                merged.insert(source_entity, false);
                // Defer the merge
                merges.push((source_entity, entity));
            }
        }

        for (source, target) in merges {
            let source_item = items
                .remove(source)
                .expect("We know this entity must have an item.");
            let mut target_item = items
                .get_mut(target)
                .expect("We know this entity must have an item.");

            if let Err(item) = target_item.try_merge(source_item) {
                // We re-insert the item, should be unreachable since we already checked whether
                // the items were mergeable in the above loop
                items
                    .insert(source, item)
                    .expect("PickupItem was removed from this entity earlier");
            } else {
                // If the merging was successfull, we remove the old item entity from the ECS
                delete_emitter.emit(DeleteEvent(source));
            }
        }

        for updated in merged
            .into_iter()
            .filter_map(|(entity, is_merge_parent)| is_merge_parent.then_some(entity))
        {
            if let Some(mut item) = items.get_mut(updated) {
                item.next_merge_check_mut().0 +=
                    (program_time.0 - item.created().0).max(1.0 / CHECKS_PER_SECOND);
            }
        }
    }
}

pub fn get_nearby_mergeable_items<'a>(
    item: &'a comp::PickupItem,
    pos: &'a comp::Pos,
    loot_owner: Option<&'a comp::LootOwner>,
    (entities, items, positions, loot_owners, spatial_grid): (
        &'a Entities<'a>,
        // We do not actually need write access here, but currently all callers of this function
        // have a WriteStorage<Item> in scope which we cannot *downcast* into a ReadStorage
        &'a WriteStorage<'a, comp::PickupItem>,
        &'a ReadStorage<'a, comp::Pos>,
        &'a ReadStorage<'a, comp::LootOwner>,
        &'a CachedSpatialGrid,
    ),
) -> impl Iterator<Item = (Entity, f32)> + 'a {
    // Get nearby items
    spatial_grid
        .0
        .in_circle_aabr(pos.0.xy(), MAX_ITEM_MERGE_DIST)
        // Filter out any unrelated entities
        .flat_map(move |entity| {
            (entities, items, positions, loot_owners.maybe())
                .lend_join()
                .get(entity, entities)
                .and_then(|(entity, item, other_position, loot_owner)| {
                    let distance_sqrd = other_position.0.distance_squared(pos.0);
                    if distance_sqrd < MAX_ITEM_MERGE_DIST.powi(2) {
                        Some((entity, item, distance_sqrd, loot_owner))
                    } else {
                        None
                    }
                })
        })
        // Filter by "mergeability"
        .filter_map(move |(entity, other_item, distance, other_loot_owner)| {
            (other_loot_owner.map(|owner| owner.owner()) == loot_owner.map(|owner| owner.owner())
                && item.can_merge(other_item)).then_some((entity, distance))
        })
}
