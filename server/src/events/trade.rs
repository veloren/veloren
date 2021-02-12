use crate::{events::group_manip::handle_invite, Server};
use common::{
    comp::{group::InviteKind, inventory::Inventory},
    trade::{PendingTrade, TradeActionMsg, TradeResult, Trades},
};
use common_net::{msg::ServerGeneral, sync::WorldSyncExt};
use specs::{world::WorldExt, Entity as EcsEntity};
use std::cmp::Ordering;
use tracing::{error, trace, warn};

/// Invoked when pressing the trade button near an entity, triggering the invite
/// UI flow
pub fn handle_initiate_trade(server: &mut Server, interactor: EcsEntity, counterparty: EcsEntity) {
    if let Some(uid) = server.state_mut().ecs().uid_from_entity(counterparty) {
        handle_invite(server, interactor, uid, InviteKind::Trade);
    } else {
        warn!("Entity tried to trade with an entity that lacks an uid");
    }
}

/// Invoked when the trade UI is up, handling item changes, accepts, etc
pub fn handle_process_trade_action(
    server: &mut Server,
    entity: EcsEntity,
    trade_id: usize,
    msg: TradeActionMsg,
) {
    if let Some(uid) = server.state.ecs().uid_from_entity(entity) {
        let mut trades = server.state.ecs().write_resource::<Trades>();
        if let TradeActionMsg::Decline = msg {
            let to_notify = trades.decline_trade(trade_id, uid);
            to_notify
                .and_then(|u| server.state.ecs().entity_from_uid(u.0))
                .map(|e| {
                    server.notify_client(e, ServerGeneral::FinishedTrade(TradeResult::Declined))
                });
        } else {
            if let Some(inv) = server.state.ecs().read_component::<Inventory>().get(entity) {
                trades.process_trade_action(trade_id, uid, msg, inv);
            }
            if let Some(trade) = trades.trades.get(&trade_id) {
                let mut msg = ServerGeneral::UpdatePendingTrade(trade_id, trade.clone());
                if trade.should_commit() {
                    let result = commit_trade(server.state.ecs(), trade);
                    msg = ServerGeneral::FinishedTrade(result);
                }
                // send the updated state to both parties
                for party in trade.parties.iter() {
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
    let mut entities = vec![];
    for who in [0, 1].iter().cloned() {
        match ecs.entity_from_uid(trade.parties[who].0) {
            Some(entity) => entities.push(entity),
            None => return TradeResult::Declined,
        }
    }
    let mut inventories = ecs.write_component::<Inventory>();
    for who in [0, 1].iter().cloned() {
        if inventories.get_mut(entities[who]).is_none() {
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
                    // no change to delta_slots[who], since they have leftovers
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
    let mut items = [vec![], vec![]];
    for who in [0, 1].iter().cloned() {
        for (slot, quantity) in trade.offers[who].iter() {
            // take the items one by one, to benefit from Inventory's stack handling
            for _ in 0..*quantity {
                inventories
                    .get_mut(entities[who])
                    .expect(invmsg)
                    .take(*slot)
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
            // this should only happen if the arithmetic above for delta_slots says there's
            // enough space and there isn't (i.e. underapproximates)
            error!(
                "Not enough space for all the items, destroying leftovers {:?}",
                leftovers
            );
        }
    }
    TradeResult::Completed
}
