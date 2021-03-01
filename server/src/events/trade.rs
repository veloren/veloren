use crate::Server;
use common::{
    comp::inventory::{item::MaterialStatManifest, Inventory},
    trade::{PendingTrade, TradeAction, TradeId, TradeResult, Trades},
};
use common_net::{
    msg::ServerGeneral,
    sync::{Uid, WorldSyncExt},
};
use hashbrown::hash_map::Entry;
use specs::{world::WorldExt, Entity as EcsEntity};
use std::cmp::Ordering;
use tracing::{error, trace};

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
                    server.notify_client(e, ServerGeneral::FinishedTrade(TradeResult::Declined))
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
                let msg = if entry.get().should_commit() {
                    let result = commit_trade(server.state.ecs(), entry.get());
                    entry.remove();
                    ServerGeneral::FinishedTrade(result)
                } else {
                    ServerGeneral::UpdatePendingTrade(trade_id, entry.get().clone())
                };
                // send the updated state to both parties
                for party in parties.iter() {
                    server
                        .state
                        .ecs()
                        .entity_from_uid(party.0)
                        .map(|e| server.notify_client(e, msg.clone()));
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
    let mut inventories = ecs.write_component::<Inventory>();
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
