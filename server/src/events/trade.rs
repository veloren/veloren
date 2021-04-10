use crate::Server;
use common::{
    comp::{
        agent::{Agent, AgentEvent},
        inventory::{item::MaterialStatManifest, Inventory},
    },
    trade::{PendingTrade, ReducedInventory, TradeAction, TradeId, TradeResult, Trades},
};
use common_net::{
    msg::ServerGeneral,
    sync::{Uid, WorldSyncExt},
};
use hashbrown::hash_map::Entry;
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
        agent.inbox.push_front(event);
    }
}

fn notify_agent_prices(
    mut agents: specs::WriteStorage<Agent>,
    index: &IndexOwned,
    entity: EcsEntity,
    event: AgentEvent,
) {
    if let Some((Some(site_id), agent)) = agents.get_mut(entity).map(|a| (a.behavior.trade_site, a))
    {
        let prices = index.get_site_prices(site_id);
        if let AgentEvent::UpdatePendingTrade(boxval) = event {
            // Box<(tid, pend, _, inventories)>) = event {
            agent
                .inbox
                .push_front(AgentEvent::UpdatePendingTrade(Box::new((
                    // Prefer using this Agent's price data, but use the counterparty's price
                    // data if we don't have price data
                    boxval.0,
                    boxval.1,
                    prices.unwrap_or(boxval.2),
                    boxval.3,
                ))));
        }
    }
}

/// Invoked when the trade UI is up, handling item changes, accepts, etc
pub fn handle_process_trade_action(
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
                                .map(|i| ReducedInventory::from(i));
                            // Get price info from the first Agent in the trade (currently, an
                            // Agent will never initiate a trade with another agent though)
                            prices = prices.or_else(|| {
                                agents
                                    .get(e)
                                    .and_then(|a| a.behavior.trade_site)
                                    .and_then(|id| server.index.get_site_prices(id))
                            });
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
    let mut delta_slots: [isize; 2] = [0, 0];
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
            match item.amount().cmp(&quantity) {
                Ordering::Less => {
                    error!(
                        "PendingTrade invariant violation in trade {:?}: party {} offered more of \
                         an item than they have",
                        trade, who
                    );
                    return TradeResult::Declined;
                },
                Ordering::Equal => {
                    delta_slots[who] -= 1; // exact, removes the whole stack
                    delta_slots[1 - who] += 1; // overapproximation, assumes the stack won't merge
                },
                Ordering::Greater => {
                    // No change to delta_slots[who], since they have leftovers
                    delta_slots[1 - who] += 1; // overapproximation, assumes the stack won't merge
                },
            }
        }
    }
    trace!("delta_slots: {:?}", delta_slots);
    for who in [0, 1].iter().cloned() {
        // Inventories should never exceed 2^{63} slots, so the usize -> isize
        // conversions here should be safe
        let inv = inventories.get_mut(entities[who]).expect(invmsg);
        if inv.populated_slots() as isize + delta_slots[who] > inv.capacity() as isize {
            return TradeResult::NotEnoughSpace;
        }
    }
    let mut items = [Vec::new(), Vec::new()];
    let msm = ecs.read_resource::<MaterialStatManifest>();
    for who in [0, 1].iter().cloned() {
        for (slot, quantity) in trade.offers[who].iter() {
            // Take the items one by one, to benefit from Inventory's stack handling
            for _ in 0..*quantity {
                inventories
                    .get_mut(entities[who])
                    .expect(invmsg)
                    .take(*slot, &msm)
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
