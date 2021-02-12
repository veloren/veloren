use crate::{
    comp::inventory::{slot::InvSlotId, Inventory},
    uid::Uid,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use tracing::{trace, warn};

/// Clients submit `TradeActionMsg` to the server, which adds the Uid of the
/// player out-of-band (i.e. without trusting the client to say who it's
/// accepting on behalf of)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TradeActionMsg {
    AddItem { item: InvSlotId, quantity: u32 },
    RemoveItem { item: InvSlotId, quantity: u32 },
    Phase1Accept,
    Phase2Accept,

    Decline,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TradeResult {
    Completed,
    Declined,
    NotEnoughSpace,
}

/// Items are not removed from the inventory during a PendingTrade: all the
/// items are moved atomically (if there's space and both parties agree) upon
/// completion
///
/// Since this stores `InvSlotId`s (i.e. references into inventories) instead of
/// items themselves, there aren't any duplication/loss risks from things like
/// dropped connections or declines, since the server doesn't have to move items
/// from a trade back into a player's inventory.
///
/// On the flip side, since they are references to *slots*, if a player could
/// swaps items in their inventory during a trade, they could mutate the trade,
/// enabling them to remove an item from the trade even after receiving the
/// counterparty's phase2 accept. To prevent this, we disallow all
/// forms of inventory manipulation in `server::events::inventory_manip` if
/// there's a pending trade that's past phase1 (in phase1, the trade should be
/// mutable anyway).
///
/// Inventory manipulation in phase1 may be beneficial to trade (e.g. splitting
/// a stack of items, once that's implemented), but should reset both phase1
/// accept flags to make the changes more visible.
///
/// Another edge case prevented by using `InvSlotId`s is that it disallows
/// trading currently-equipped items (since `EquipSlot`s are disjoint from
/// `InvSlotId`s), which avoids the issues associated with trading equipped bags
/// that may still have contents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingTrade {
    /// `parties[0]` is the entity that initiated the trade, parties[1] is the
    /// other entity that's being traded with
    pub parties: [Uid; 2],
    /// `offers[i]` represents the items and quantities of the party i's items
    /// being offered
    pub offers: [HashMap<InvSlotId, u32>; 2],
    /// phase1_accepts indicate that the parties wish to proceed to review
    pub phase1_accepts: [bool; 2],
    /// phase2_accepts indicate that the parties have reviewed the trade and
    /// wish to commit it
    pub phase2_accepts: [bool; 2],
}

impl PendingTrade {
    pub fn new(party: Uid, counterparty: Uid) -> PendingTrade {
        PendingTrade {
            parties: [party, counterparty],
            offers: [HashMap::new(), HashMap::new()],
            phase1_accepts: [false, false],
            phase2_accepts: [false, false],
        }
    }

    pub fn in_phase1(&self) -> bool { !self.phase1_accepts[0] || !self.phase1_accepts[1] }

    pub fn in_phase2(&self) -> bool {
        (self.phase1_accepts[0] && self.phase1_accepts[1])
            && (!self.phase2_accepts[0] || !self.phase2_accepts[1])
    }

    pub fn should_commit(&self) -> bool {
        self.phase1_accepts[0]
            && self.phase1_accepts[1]
            && self.phase2_accepts[0]
            && self.phase2_accepts[1]
    }

    pub fn which_party(&self, party: Uid) -> Option<usize> {
        self.parties
            .iter()
            .enumerate()
            .find(|(_, x)| **x == party)
            .map(|(i, _)| i)
    }

    /// Invariants:
    /// - A party is never shown as offering more of an item than they own
    /// - Offers with a quantity of zero get removed from the trade
    /// - Modifications can only happen in phase 1
    /// - Whenever a trade is modified, both accept flags get reset
    /// - Accept flags only get set for the current phase
    pub fn process_msg(&mut self, who: usize, msg: TradeActionMsg, inventory: &Inventory) {
        use TradeActionMsg::*;
        match msg {
            AddItem {
                item,
                quantity: delta,
            } => {
                if self.in_phase1() && delta > 0 {
                    let total = self.offers[who].entry(item).or_insert(0);
                    let owned_quantity = inventory.get(item).map(|i| i.amount()).unwrap_or(0);
                    *total = total.saturating_add(delta).min(owned_quantity);
                    self.phase1_accepts = [false, false];
                }
            },
            RemoveItem {
                item,
                quantity: delta,
            } => {
                if self.in_phase1() {
                    self.offers[who]
                        .entry(item)
                        .and_replace_entry_with(|_, mut total| {
                            total = total.saturating_sub(delta);
                            if total > 0 { Some(total) } else { None }
                        });
                    self.phase1_accepts = [false, false];
                }
            },
            Phase1Accept => {
                if self.in_phase1() {
                    self.phase1_accepts[who] = true;
                }
            },
            Phase2Accept => {
                if self.in_phase2() {
                    self.phase2_accepts[who] = true;
                }
            },
            Decline => {},
        }
    }
}

pub struct Trades {
    pub next_id: usize,
    pub trades: HashMap<usize, PendingTrade>,
    pub entity_trades: HashMap<Uid, usize>,
}

impl Trades {
    pub fn begin_trade(&mut self, party: Uid, counterparty: Uid) -> usize {
        let id = self.next_id;
        self.next_id = id.wrapping_add(1);
        self.trades
            .insert(id, PendingTrade::new(party, counterparty));
        self.entity_trades.insert(party, id);
        self.entity_trades.insert(counterparty, id);
        id
    }

    pub fn process_trade_action(
        &mut self,
        id: usize,
        who: Uid,
        msg: TradeActionMsg,
        inventory: &Inventory,
    ) {
        trace!("for trade id {}, message {:?}", id, msg);
        if let Some(trade) = self.trades.get_mut(&id) {
            if let Some(party) = trade.which_party(who) {
                trade.process_msg(party, msg, inventory);
            } else {
                warn!(
                    "An entity who is not a party to trade {} tried to modify it",
                    id
                );
            }
        } else {
            warn!("Attempt to modify nonexistent trade id {}", id);
        }
    }

    pub fn decline_trade(&mut self, id: usize, who: Uid) -> Option<Uid> {
        let mut to_notify = None;
        if let Some(trade) = self.trades.remove(&id) {
            match trade.which_party(who) {
                Some(i) => {
                    self.entity_trades.remove(&trade.parties[0]);
                    self.entity_trades.remove(&trade.parties[1]);
                    // let the other person know the trade was declined
                    to_notify = Some(trade.parties[1 - i])
                },
                None => {
                    warn!(
                        "An entity who is not a party to trade {} tried to decline it",
                        id
                    );
                    // put it back
                    self.trades.insert(id, trade);
                },
            }
        } else {
            warn!("Attempt to decline nonexistent trade id {}", id);
        }
        to_notify
    }

    /// See the doc comment on `common::trade::PendingTrade` for the
    /// significance of these checks
    pub fn in_trade_with_property<F: FnOnce(&PendingTrade) -> bool>(
        &self,
        uid: &Uid,
        f: F,
    ) -> bool {
        self.entity_trades
            .get(uid)
            .and_then(|trade_id| self.trades.get(trade_id))
            .map(f)
            // if any of the option lookups failed, we're not in any trade
            .unwrap_or(false)
    }

    pub fn in_immutable_trade(&self, uid: &Uid) -> bool {
        self.in_trade_with_property(uid, |trade| !trade.in_phase1())
    }

    pub fn in_mutable_trade(&self, uid: &Uid) -> bool {
        self.in_trade_with_property(uid, |trade| trade.in_phase1())
    }

    pub fn implicit_mutation_occurred(&mut self, uid: &Uid) {
        if let Some(trade_id) = self.entity_trades.get(uid) {
            self.trades
                .get_mut(trade_id)
                .map(|trade| trade.phase1_accepts = [false, false]);
        }
    }
}

impl Default for Trades {
    fn default() -> Trades {
        Trades {
            next_id: 0,
            trades: HashMap::new(),
            entity_trades: HashMap::new(),
        }
    }
}
