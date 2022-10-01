use crate::Server;
use common::{
    comp::{
        agent::{Agent, AgentEvent},
        inventory::{
            item::{tool::AbilityMap, ItemDefinitionIdOwned, MaterialStatManifest},
            Inventory,
        },
    },
    trade::{PendingTrade, ReducedInventory, TradeAction, TradeId, TradeResult, Trades},
};
use common_net::{
    msg::ServerGeneral,
    sync::{Uid, WorldSyncExt},
};
use hashbrown::{hash_map::Entry, HashMap};
use specs::{world::WorldExt, Entity as EcsEntity};
use std::cmp::Ordering;
use tracing::{error, trace};
use world::IndexOwned;

fn notify_agent_simple(
    mut agents: specs::WriteStorage<Agent>,
    entity: EcsEntity,
    event: AgentEvent,
) {
    if let Some(agent) = agents.get_mut(entity) {
        agent.inbox.push_back(event);
    }
}

fn notify_agent_prices(
    mut agents: specs::WriteStorage<Agent>,
    index: &IndexOwned,
    entity: EcsEntity,
    event: AgentEvent,
) {
    if let Some((site_id, agent)) = agents.get_mut(entity).map(|a| (a.behavior.trade_site(), a)) {
        if let AgentEvent::UpdatePendingTrade(boxval) = event {
            // Prefer using this Agent's price data, but use the counterparty's price
            // data if we don't have price data
            let prices = site_id
                .and_then(|site_id| index.get_site_prices(site_id))
                .unwrap_or(boxval.2);
            // Box<(tid, pend, _, inventories)>) = event {
            agent
                .inbox
                .push_back(AgentEvent::UpdatePendingTrade(Box::new((
                    boxval.0, boxval.1, prices, boxval.3,
                ))));
        }
    }
}

/// Invoked when the trade UI is up, handling item changes, accepts, etc
pub(super) fn handle_process_trade_action(
    server: &mut Server,
    entity: EcsEntity,
    trade_id: TradeId,
    action: TradeAction,
) {
    if let Some(uid) = server.state.ecs().uid_from_entity(entity) {
        let mut trades = server.state.ecs().write_resource::<Trades>();
        if let TradeAction::Decline = action {
            let to_notify = trades.decline_trade(trade_id, uid);
            to_notify
                .and_then(|u| server.state.ecs().entity_from_uid(u.0))
                .map(|e| {
                    server.notify_client(e, ServerGeneral::FinishedTrade(TradeResult::Declined));
                    notify_agent_simple(
                        server.state.ecs().write_storage::<Agent>(),
                        e,
                        AgentEvent::FinishedTrade(TradeResult::Declined),
                    );
                });
        } else {
            {
                let ecs = server.state.ecs();
                let inventories = ecs.read_component::<Inventory>();
                let get_inventory = |uid: Uid| {
                    if let Some(entity) = ecs.entity_from_uid(uid.0) {
                        inventories.get(entity)
                    } else {
                        None
                    }
                };
                trades.process_trade_action(trade_id, uid, action, get_inventory);
            }
            if let Entry::Occupied(entry) = trades.trades.entry(trade_id) {
                let parties = entry.get().parties;
                if entry.get().should_commit() {
                    let result = commit_trade(server.state.ecs(), entry.get());
                    entry.remove();
                    for party in parties.iter() {
                        if let Some(e) = server.state.ecs().entity_from_uid(party.0) {
                            server.notify_client(e, ServerGeneral::FinishedTrade(result.clone()));
                            notify_agent_simple(
                                server.state.ecs().write_storage::<Agent>(),
                                e,
                                AgentEvent::FinishedTrade(result.clone()),
                            );
                        }
                        trades.entity_trades.remove_entry(party);
                    }
                } else {
                    let mut entities: [Option<specs::Entity>; 2] = [None, None];
                    let mut inventories: [Option<ReducedInventory>; 2] = [None, None];
                    let mut prices = None;
                    let agents = server.state.ecs().read_storage::<Agent>();
                    // sadly there is no map and collect on arrays
                    for i in 0..2 {
                        // parties.len()) {
                        entities[i] = server.state.ecs().entity_from_uid(parties[i].0);
                        if let Some(e) = entities[i] {
                            inventories[i] = server
                                .state
                                .ecs()
                                .read_component::<Inventory>()
                                .get(e)
                                .map(ReducedInventory::from);
                            // Get price info from the first Agent in the trade (currently, an
                            // Agent will never initiate a trade with another agent though)
                            #[cfg(feature = "worldgen")]
                            {
                                prices = prices.or_else(|| {
                                    agents
                                        .get(e)
                                        .and_then(|a| a.behavior.trade_site())
                                        .and_then(|id| server.index.get_site_prices(id))
                                });
                            }
                        }
                    }
                    drop(agents);
                    for party in entities.iter() {
                        if let Some(e) = *party {
                            server.notify_client(
                                e,
                                ServerGeneral::UpdatePendingTrade(
                                    trade_id,
                                    entry.get().clone(),
                                    prices.clone(),
                                ),
                            );
                            #[cfg(feature = "worldgen")]
                            notify_agent_prices(
                                server.state.ecs().write_storage::<Agent>(),
                                &server.index,
                                e,
                                AgentEvent::UpdatePendingTrade(Box::new((
                                    trade_id,
                                    entry.get().clone(),
                                    prices.clone().unwrap_or_default(),
                                    inventories.clone(),
                                ))),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Cancel all trades registered for a given UID.
///
/// Note: This doesn't send any notification to the provided entity (only other
/// participants in the trade). It is assumed that the supplied entity either no
/// longer exists or is awareof this cancellation through other means (e.g.
/// client getting ExitInGameSuccess message knows that it should clear any
/// trades).
pub(crate) fn cancel_trades_for(state: &mut common_state::State, entity: EcsEntity) {
    let ecs = state.ecs();
    if let Some(uid) = ecs.uid_from_entity(entity) {
        let mut trades = ecs.write_resource::<Trades>();

        let active_trade = match trades.entity_trades.get(&uid) {
            Some(n) => *n,
            None => return,
        };

        let to_notify = trades.decline_trade(active_trade, uid);
        to_notify.and_then(|u| ecs.entity_from_uid(u.0)).map(|e| {
            if let Some(c) = ecs.read_storage::<crate::Client>().get(e) {
                c.send_fallible(ServerGeneral::FinishedTrade(TradeResult::Declined));
            }
            notify_agent_simple(
                ecs.write_storage::<Agent>(),
                e,
                AgentEvent::FinishedTrade(TradeResult::Declined),
            );
        });
    }
}

/// Commit a trade that both parties have agreed to, modifying their respective
/// inventories
fn commit_trade(ecs: &specs::World, trade: &PendingTrade) -> TradeResult {
    let mut entities = Vec::new();
    for party in trade.parties.iter() {
        match ecs.entity_from_uid(party.0) {
            Some(entity) => entities.push(entity),
            None => return TradeResult::Declined,
        }
    }
    let mut inventories = ecs.write_storage::<Inventory>();
    for entity in entities.iter() {
        if inventories.get_mut(*entity).is_none() {
            return TradeResult::Declined;
        }
    }
    let invmsg = "inventories.get_mut(entities[who]).is_none() should have returned already";
    trace!("committing trade: {:?}", trade);
    // Compute the net change in slots of each player during the trade, to detect
    // out-of-space-ness before transferring any items
    let mut delta_slots: [i64; 2] = [0, 0];

    // local struct used to calculate delta_slots for stackable items.
    // Uses u128 as an intermediate value to prevent overflow.
    #[derive(Default)]
    struct ItemQuantities {
        full_stacks: u64,
        quantity_sold: u128,
        freed_capacity: u128,
        unusable_capacity: u128,
    }

    // local struct used to calculate delta_slots for stackable items
    struct TradeQuantities {
        max_stack_size: u32,
        trade_quantities: [ItemQuantities; 2],
    }

    impl TradeQuantities {
        fn new(max_stack_size: u32) -> Self {
            Self {
                max_stack_size,
                trade_quantities: [ItemQuantities::default(), ItemQuantities::default()],
            }
        }
    }

    // Hashmap to compute merged stackable stacks, including overflow checks
    let mut stackable_items: HashMap<ItemDefinitionIdOwned, TradeQuantities> = HashMap::new();
    for who in [0, 1].iter().cloned() {
        for (slot, quantity) in trade.offers[who].iter() {
            let inventory = inventories.get_mut(entities[who]).expect(invmsg);
            let item = match inventory.get(*slot) {
                Some(item) => item,
                None => {
                    error!(
                        "PendingTrade invariant violation in trade {:?}: slots offered in a trade \
                         should be non-empty",
                        trade
                    );
                    return TradeResult::Declined;
                },
            };

            // assuming the quantity is never 0
            match item.amount().cmp(quantity) {
                Ordering::Less => {
                    error!(
                        "PendingTrade invariant violation in trade {:?}: party {} offered more of \
                         an item than they have",
                        trade, who
                    );
                    return TradeResult::Declined;
                },
                Ordering::Equal => {
                    if item.is_stackable() {
                        // Marks a full stack to remove. Can no longer accept items from the other
                        // party, and therefore adds the remaining capacity it holds to
                        // `unusable_capacity`.
                        let TradeQuantities {
                            max_stack_size,
                            trade_quantities,
                        } = stackable_items
                            .entry(item.item_definition_id().to_owned())
                            .or_insert_with(|| TradeQuantities::new(item.max_amount()));

                        trade_quantities[who].full_stacks += 1;
                        trade_quantities[who].quantity_sold += *quantity as u128;
                        trade_quantities[who].unusable_capacity +=
                            *max_stack_size as u128 - item.amount() as u128;
                    } else {
                        delta_slots[who] -= 1; // exact, removes the whole stack
                        delta_slots[1 - who] += 1; // item is not stackable, so the stacks won't merge
                    }
                },
                Ordering::Greater => {
                    if item.is_stackable() {
                        // Marks a partial stack to remove, able to accepts items from the other
                        // party, and therefore adds the additional capacity freed after the item
                        // exchange to `freed_capacity`.
                        let TradeQuantities {
                            max_stack_size: _,
                            trade_quantities,
                        } = stackable_items
                            .entry(item.item_definition_id().to_owned())
                            .or_insert_with(|| TradeQuantities::new(item.max_amount()));

                        trade_quantities[who].quantity_sold += *quantity as u128;
                        trade_quantities[who].freed_capacity += *quantity as u128;
                    } else {
                        // unreachable in theory
                        error!(
                            "Inventory invariant violation in trade {:?}: party {} has a stack \
                             larger than 1 of an unstackable item",
                            trade, who
                        );
                        return TradeResult::Declined;
                    }
                },
            }
        }
    }
    // at this point delta_slots only contains the slot variations for unstackable
    // items. The following loops calculates capacity for stackable items and
    // computes the final delta_slots in consequence.

    for (
        item_id,
        TradeQuantities {
            max_stack_size,
            trade_quantities,
        },
    ) in stackable_items.iter()
    {
        for who in [0, 1].iter().cloned() {
            // removes all exchanged full stacks.
            delta_slots[who] -= trade_quantities[who].full_stacks as i64;

            // calculates the available item capacity in the other party's inventory,
            // substracting the unusable space calculated previously,
            // and adding the capacity freed by the trade.
            let other_party_capacity = inventories
                .get_mut(entities[1 - who])
                .expect(invmsg)
                .slots()
                .flatten()
                .filter_map(|it| {
                    if it.item_definition_id() == item_id.as_ref() {
                        Some(*max_stack_size as u128 - it.amount() as u128)
                    } else {
                        None
                    }
                })
                .sum::<u128>()
                - trade_quantities[1 - who].unusable_capacity
                + trade_quantities[1 - who].freed_capacity;

            // checks if the capacity in remaining partial stacks of the other party is
            // enough to contain everything, creates more stacks otherwise
            if other_party_capacity < trade_quantities[who].quantity_sold {
                let surplus = trade_quantities[who].quantity_sold - other_party_capacity;
                // the total amount of exchanged slots can never exceed the max inventory size
                // (around 4 * 2^32 slots), so the cast to i64 should be safe
                delta_slots[1 - who] += (surplus / *max_stack_size as u128) as i64 + 1;
            }
        }
    }

    trace!("delta_slots: {:?}", delta_slots);
    for who in [0, 1].iter().cloned() {
        // Inventories should never exceed 2^{63} slots, so the usize -> i64
        // conversions here should be safe
        let inv = inventories.get_mut(entities[who]).expect(invmsg);
        if inv.populated_slots() as i64 + delta_slots[who] > inv.capacity() as i64 {
            return TradeResult::NotEnoughSpace;
        }
    }

    let mut items = [Vec::new(), Vec::new()];
    let ability_map = ecs.read_resource::<AbilityMap>();
    let msm = ecs.read_resource::<MaterialStatManifest>();
    for who in [0, 1].iter().cloned() {
        for (slot, quantity) in trade.offers[who].iter() {
            // Take the items one by one, to benefit from Inventory's stack handling
            for _ in 0..*quantity {
                inventories
                    .get_mut(entities[who])
                    .expect(invmsg)
                    .take(*slot, &ability_map, &msm)
                    .map(|item| items[who].push(item));
            }
        }
    }

    for who in [0, 1].iter().cloned() {
        if let Err(leftovers) = inventories
            .get_mut(entities[1 - who])
            .expect(invmsg)
            .push_all(items[who].drain(..))
        {
            // This should only happen if the arithmetic above for delta_slots says there's
            // enough space and there isn't (i.e. underapproximates)
            panic!(
                "Not enough space for all the items, leftovers are {:?}",
                leftovers
            );
        }
    }

    TradeResult::Completed
}

#[cfg(test)]
mod tests {
    use hashbrown::HashMap;

    use super::*;
    use common::{comp::slot::InvSlotId, uid::UidAllocator};

    use specs::{Builder, World};

    // Creates a specs World containing two Entities which have Inventory
    // components. Left over inventory size is determined by input. Mapping to the
    // returned Entities. Any input over the maximum default inventory size will
    // result in maximum left over space.
    fn create_mock_trading_world(
        player_inv_size: usize,
        merchant_inv_size: usize,
    ) -> (World, EcsEntity, EcsEntity) {
        let mut mockworld = World::new();
        mockworld.insert(UidAllocator::new());
        mockworld.insert(MaterialStatManifest::load().cloned());
        mockworld.insert(AbilityMap::load().cloned());
        mockworld.register::<Inventory>();
        mockworld.register::<Uid>();

        let player: EcsEntity = mockworld
            .create_entity()
            .with(Inventory::with_empty())
            .build();

        let merchant: EcsEntity = mockworld
            .create_entity()
            .with(Inventory::with_empty())
            .build();

        {
            use specs::saveload::MarkerAllocator;
            let mut uids = mockworld.write_component::<Uid>();
            let mut uid_allocator = mockworld.write_resource::<UidAllocator>();
            uids.insert(player, uid_allocator.allocate(player, None))
                .expect("inserting player uid failed");
            uids.insert(merchant, uid_allocator.allocate(merchant, None))
                .expect("inserting merchant uid failed");
        }

        let invmsg = "inventories.get_mut().is_none() should have returned already";
        let capmsg = "There should be enough space here";
        let mut inventories = mockworld.write_component::<Inventory>();
        let mut playerinv = inventories.get_mut(player).expect(invmsg);
        if player_inv_size < playerinv.capacity() {
            for _ in player_inv_size..playerinv.capacity() {
                playerinv
                    .push(common::comp::Item::new_from_asset_expect(
                        "common.items.npc_armor.pants.plate_red",
                    ))
                    .expect(capmsg);
            }
        }

        let mut merchantinv = inventories.get_mut(merchant).expect(invmsg);
        if merchant_inv_size < merchantinv.capacity() {
            for _ in merchant_inv_size..merchantinv.capacity() {
                merchantinv
                    .push(common::comp::Item::new_from_asset_expect(
                        "common.items.armor.cloth_purple.foot",
                    ))
                    .expect(capmsg);
            }
        }
        drop(inventories);

        (mockworld, player, merchant)
    }

    fn prepare_merchant_inventory(mockworld: &World, merchant: EcsEntity) {
        let mut inventories = mockworld.write_component::<Inventory>();
        let invmsg = "inventories.get_mut().is_none() should have returned already";
        let capmsg = "There should be enough space here";
        let mut merchantinv = inventories.get_mut(merchant).expect(invmsg);
        for _ in 0..10 {
            merchantinv
                .push(common::comp::Item::new_from_asset_expect(
                    "common.items.consumable.potion_minor",
                ))
                .expect(capmsg);
            merchantinv
                .push(common::comp::Item::new_from_asset_expect(
                    "common.items.food.meat.fish_cooked",
                ))
                .expect(capmsg);
        }
        drop(inventories);
    }

    #[test]
    fn commit_trade_with_stackable_item_test() {
        use common::{assets::AssetExt, comp::item::ItemDef};
        use std::sync::Arc;

        let (mockworld, player, merchant) = create_mock_trading_world(1, 20);

        prepare_merchant_inventory(&mockworld, merchant);

        let invmsg = "inventories.get_mut().is_none() should have returned already";
        let capmsg = "There should be enough space here";
        let mut inventories = mockworld.write_component::<Inventory>();

        let mut playerinv = inventories.get_mut(player).expect(invmsg);
        playerinv
            .push(common::comp::Item::new_from_asset_expect(
                "common.items.consumable.potion_minor",
            ))
            .expect(capmsg);

        let potion_asset = "common.items.consumable.potion_minor";

        let potion = common::comp::Item::new_from_asset_expect(potion_asset);
        let potion_def = Arc::<ItemDef>::load_expect_cloned(potion_asset);

        let merchantinv = inventories.get_mut(merchant).expect(invmsg);

        let potioninvid = merchantinv
            .get_slot_of_item(&potion)
            .expect("expected get_slot_of_item to return");

        let playerid = mockworld
            .uid_from_entity(player)
            .expect("mockworld.uid_from_entity(player) should have returned");
        let merchantid = mockworld
            .uid_from_entity(merchant)
            .expect("mockworld.uid_from_entity(player) should have returned");

        let playeroffers: HashMap<InvSlotId, u32> = HashMap::new();
        let mut merchantoffers: HashMap<InvSlotId, u32> = HashMap::new();
        merchantoffers.insert(potioninvid, 1);

        let trade = PendingTrade {
            parties: [playerid, merchantid],
            accept_flags: [true, true],
            offers: [playeroffers, merchantoffers],
            phase: common::trade::TradePhase::Review,
        };

        drop(inventories);

        let traderes = commit_trade(&mockworld, &trade);
        assert_eq!(traderes, TradeResult::Completed);

        let mut inventories = mockworld.write_component::<Inventory>();
        let playerinv = inventories.get_mut(player).expect(invmsg);
        let potioncount = playerinv.item_count(&potion_def);
        assert_eq!(potioncount, 2);
    }

    #[test]
    fn commit_trade_with_full_inventory_test() {
        let (mockworld, player, merchant) = create_mock_trading_world(1, 20);

        prepare_merchant_inventory(&mockworld, merchant);

        let invmsg = "inventories.get_mut().is_none() should have returned already";
        let capmsg = "There should be enough space here";
        let mut inventories = mockworld.write_component::<Inventory>();

        let mut playerinv = inventories.get_mut(player).expect(invmsg);
        playerinv
            .push(common::comp::Item::new_from_asset_expect(
                "common.items.consumable.potion_minor",
            ))
            .expect(capmsg);

        let fish = common::comp::Item::new_from_asset_expect("common.items.food.meat.fish_cooked");
        let merchantinv = inventories.get_mut(merchant).expect(invmsg);

        let fishinvid = merchantinv
            .get_slot_of_item(&fish)
            .expect("expected get_slot_of_item to return");

        let playerid = mockworld
            .uid_from_entity(player)
            .expect("mockworld.uid_from_entity(player) should have returned");
        let merchantid = mockworld
            .uid_from_entity(merchant)
            .expect("mockworld.uid_from_entity(player) should have returned");

        let playeroffers: HashMap<InvSlotId, u32> = HashMap::new();
        let mut merchantoffers: HashMap<InvSlotId, u32> = HashMap::new();
        merchantoffers.insert(fishinvid, 1);
        let trade = PendingTrade {
            parties: [playerid, merchantid],
            accept_flags: [true, true],
            offers: [playeroffers, merchantoffers],
            phase: common::trade::TradePhase::Review,
        };

        drop(inventories);

        let traderes = commit_trade(&mockworld, &trade);
        assert_eq!(traderes, TradeResult::NotEnoughSpace);
    }

    #[test]
    fn commit_trade_with_empty_inventory_test() {
        let (mockworld, player, merchant) = create_mock_trading_world(20, 20);

        prepare_merchant_inventory(&mockworld, merchant);

        let invmsg = "inventories.get_mut().is_none() should have returned already";
        let mut inventories = mockworld.write_component::<Inventory>();

        let fish = common::comp::Item::new_from_asset_expect("common.items.food.meat.fish_cooked");
        let merchantinv = inventories.get_mut(merchant).expect(invmsg);

        let fishinvid = merchantinv
            .get_slot_of_item(&fish)
            .expect("expected get_slot_of_item to return");

        let playerid = mockworld
            .uid_from_entity(player)
            .expect("mockworld.uid_from_entity(player) should have returned");
        let merchantid = mockworld
            .uid_from_entity(merchant)
            .expect("mockworld.uid_from_entity(player) should have returned");

        let playeroffers: HashMap<InvSlotId, u32> = HashMap::new();
        let mut merchantoffers: HashMap<InvSlotId, u32> = HashMap::new();
        merchantoffers.insert(fishinvid, 1);
        let trade = PendingTrade {
            parties: [playerid, merchantid],
            accept_flags: [true, true],
            offers: [playeroffers, merchantoffers],
            phase: common::trade::TradePhase::Review,
        };

        drop(inventories);

        let traderes = commit_trade(&mockworld, &trade);
        assert_eq!(traderes, TradeResult::Completed);
    }

    #[test]
    fn commit_trade_with_both_full_inventories_test() {
        let (mockworld, player, merchant) = create_mock_trading_world(2, 2);

        prepare_merchant_inventory(&mockworld, merchant);

        let invmsg = "inventories.get_mut().is_none() should have returned already";
        let capmsg = "There should be enough space here";
        let mut inventories = mockworld.write_component::<Inventory>();

        let fish = common::comp::Item::new_from_asset_expect("common.items.food.meat.fish_cooked");
        let merchantinv = inventories.get_mut(merchant).expect(invmsg);
        let fishinvid = merchantinv
            .get_slot_of_item(&fish)
            .expect("expected get_slot_of_item to return");

        let potion =
            common::comp::Item::new_from_asset_expect("common.items.consumable.potion_minor");
        let mut playerinv = inventories.get_mut(player).expect(invmsg);
        playerinv
            .push(common::comp::Item::new_from_asset_expect(
                "common.items.consumable.potion_minor",
            ))
            .expect(capmsg);
        let potioninvid = playerinv
            .get_slot_of_item(&potion)
            .expect("expected get_slot_of_item to return");

        let playerid = mockworld
            .uid_from_entity(player)
            .expect("mockworld.uid_from_entity(player) should have returned");
        let merchantid = mockworld
            .uid_from_entity(merchant)
            .expect("mockworld.uid_from_entity(player) should have returned");

        let mut playeroffers: HashMap<InvSlotId, u32> = HashMap::new();
        playeroffers.insert(potioninvid, 1);

        let mut merchantoffers: HashMap<InvSlotId, u32> = HashMap::new();
        merchantoffers.insert(fishinvid, 10);
        let trade = PendingTrade {
            parties: [playerid, merchantid],
            accept_flags: [true, true],
            offers: [playeroffers, merchantoffers],
            phase: common::trade::TradePhase::Review,
        };

        drop(inventories);

        let traderes = commit_trade(&mockworld, &trade);
        assert_eq!(traderes, TradeResult::Completed);
    }

    #[test]
    fn commit_trade_with_overflow() {
        let (mockworld, player, merchant) = create_mock_trading_world(2, 20);

        prepare_merchant_inventory(&mockworld, merchant);

        let invmsg = "inventories.get_mut().is_none() should have returned already";
        let capmsg = "There should be enough space here";
        let mut inventories = mockworld.write_component::<Inventory>();

        let mut playerinv = inventories.get_mut(player).expect(invmsg);
        let mut potion =
            common::comp::Item::new_from_asset_expect("common.items.consumable.potion_minor");
        potion
            .set_amount(potion.max_amount() - 2)
            .expect("Should be below the max amount");
        playerinv.push(potion).expect(capmsg);

        let potion =
            common::comp::Item::new_from_asset_expect("common.items.consumable.potion_minor");
        let merchantinv = inventories.get_mut(merchant).expect(invmsg);

        let potioninvid = merchantinv
            .get_slot_of_item(&potion)
            .expect("expected get_slot_of_item to return");

        let playerid = mockworld
            .uid_from_entity(player)
            .expect("mockworld.uid_from_entity(player) should have returned");
        let merchantid = mockworld
            .uid_from_entity(merchant)
            .expect("mockworld.uid_from_entity(player) should have returned");

        let playeroffers: HashMap<InvSlotId, u32> = HashMap::new();
        let mut merchantoffers: HashMap<InvSlotId, u32> = HashMap::new();
        merchantoffers.insert(potioninvid, 10);
        let trade = PendingTrade {
            parties: [playerid, merchantid],
            accept_flags: [true, true],
            offers: [playeroffers, merchantoffers],
            phase: common::trade::TradePhase::Review,
        };

        drop(inventories);

        let traderes = commit_trade(&mockworld, &trade);
        assert_eq!(traderes, TradeResult::Completed);

        let mut inventories = mockworld.write_component::<Inventory>();
        let mut playerinv = inventories.get_mut(player).expect(invmsg);

        let slot1 = playerinv
            .get_slot_of_item(&potion)
            .expect("There should be a slot here");
        let item1 = playerinv
            .remove(slot1)
            .expect("The slot should not be empty");

        let slot2 = playerinv
            .get_slot_of_item(&potion)
            .expect("There should be a slot here");
        let item2 = playerinv
            .remove(slot2)
            .expect("The slot should not be empty");

        assert_eq!(item1.amount(), potion.max_amount());
        assert_eq!(item2.amount(), 8);
    }

    #[test]
    fn commit_trade_with_inventory_overflow_failure() {
        let (mockworld, player, merchant) = create_mock_trading_world(2, 20);

        prepare_merchant_inventory(&mockworld, merchant);

        let invmsg = "inventories.get_mut().is_none() should have returned already";
        let capmsg = "There should be enough space here";
        let mut inventories = mockworld.write_component::<Inventory>();

        let mut playerinv = inventories.get_mut(player).expect(invmsg);
        let mut potion =
            common::comp::Item::new_from_asset_expect("common.items.consumable.potion_minor");
        potion
            .set_amount(potion.max_amount() - 2)
            .expect("Should be below the max amount");
        playerinv.push(potion).expect(capmsg);
        let mut potion =
            common::comp::Item::new_from_asset_expect("common.items.consumable.potion_minor");
        potion
            .set_amount(potion.max_amount() - 2)
            .expect("Should be below the max amount");
        playerinv.push(potion).expect(capmsg);

        let potion =
            common::comp::Item::new_from_asset_expect("common.items.consumable.potion_minor");
        let merchantinv = inventories.get_mut(merchant).expect(invmsg);

        let potioninvid = merchantinv
            .get_slot_of_item(&potion)
            .expect("expected get_slot_of_item to return");

        let playerid = mockworld
            .uid_from_entity(player)
            .expect("mockworld.uid_from_entity(player) should have returned");
        let merchantid = mockworld
            .uid_from_entity(merchant)
            .expect("mockworld.uid_from_entity(player) should have returned");

        let playeroffers: HashMap<InvSlotId, u32> = HashMap::new();
        let mut merchantoffers: HashMap<InvSlotId, u32> = HashMap::new();
        merchantoffers.insert(potioninvid, 5);
        let trade = PendingTrade {
            parties: [playerid, merchantid],
            accept_flags: [true, true],
            offers: [playeroffers, merchantoffers],
            phase: common::trade::TradePhase::Review,
        };

        drop(inventories);

        let traderes = commit_trade(&mockworld, &trade);
        assert_eq!(traderes, TradeResult::NotEnoughSpace);
    }

    #[test]
    fn commit_trade_with_inventory_overflow_success() {
        let (mockworld, player, merchant) = create_mock_trading_world(2, 20);

        prepare_merchant_inventory(&mockworld, merchant);

        let invmsg = "inventories.get_mut().is_none() should have returned already";
        let capmsg = "There should be enough space here";
        let mut inventories = mockworld.write_component::<Inventory>();

        let mut playerinv = inventories.get_mut(player).expect(invmsg);
        let mut potion =
            common::comp::Item::new_from_asset_expect("common.items.consumable.potion_minor");
        potion
            .set_amount(potion.max_amount() - 2)
            .expect("Should be below the max amount");
        playerinv.push(potion).expect(capmsg);
        let mut potion =
            common::comp::Item::new_from_asset_expect("common.items.consumable.potion_minor");
        potion
            .set_amount(potion.max_amount() - 2)
            .expect("Should be below the max amount");
        playerinv.push(potion).expect(capmsg);

        let potion =
            common::comp::Item::new_from_asset_expect("common.items.consumable.potion_minor");
        let merchantinv = inventories.get_mut(merchant).expect(invmsg);

        let potioninvid = merchantinv
            .get_slot_of_item(&potion)
            .expect("expected get_slot_of_item to return");

        let playerid = mockworld
            .uid_from_entity(player)
            .expect("mockworld.uid_from_entity(player) should have returned");
        let merchantid = mockworld
            .uid_from_entity(merchant)
            .expect("mockworld.uid_from_entity(player) should have returned");

        let playeroffers: HashMap<InvSlotId, u32> = HashMap::new();
        let mut merchantoffers: HashMap<InvSlotId, u32> = HashMap::new();
        merchantoffers.insert(potioninvid, 4);
        let trade = PendingTrade {
            parties: [playerid, merchantid],
            accept_flags: [true, true],
            offers: [playeroffers, merchantoffers],
            phase: common::trade::TradePhase::Review,
        };

        drop(inventories);

        let traderes = commit_trade(&mockworld, &trade);
        assert_eq!(traderes, TradeResult::Completed);
    }
}
