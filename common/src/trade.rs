use crate::{comp::inventory::slot::InvSlotId, uid::Uid};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use tracing::warn;

/// Clients submit `TradeActionMsg` to the server, which adds the Uid of the
/// player out-of-band (i.e. without trusting the client to say who it's
/// accepting on behalf of)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TradeActionMsg {
    AddItem { item: InvSlotId, quantity: usize },
    RemoveItem { item: InvSlotId, quantity: usize },
    Phase1Accept,
    Phase2Accept,
}

/// Items are not removed from the inventory during a PendingTrade: all the
/// items are moved atomically (if there's space and both parties agree) upon
/// completion
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingTrade {
    /// `parties[0]` is the entity that initiated the trade, parties[1] is the
    /// other entity that's being traded with
    pub parties: [Uid; 2],
    /// `offers[i]` represents the items and quantities of the party i's items
    /// being offered
    pub offers: [HashMap<InvSlotId, usize>; 2],
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
        (self.phase1_accepts[0] && self.phase1_accepts[1]) && (!self.phase2_accepts[0] || !self.phase2_accepts[1])
    }

    pub fn should_commit(&self) -> bool {
        self.phase1_accepts[0] && self.phase1_accepts[1] && self.phase2_accepts[0] && self.phase2_accepts[1]
    }

    pub fn which_party(&self, party: Uid) -> Option<usize> {
        self.parties
            .iter()
            .enumerate()
            .find(|(_, x)| **x == party)
            .map(|(i, _)| i)
    }

    pub fn process_msg(&mut self, who: usize, msg: TradeActionMsg) {
        use TradeActionMsg::*;
        match msg {
            AddItem { item, quantity } => {
                if self.in_phase1() {
                    let total = self.offers[who].entry(item).or_insert(0);
                    *total = total.saturating_add(quantity);
                }
            },
            RemoveItem { item, quantity } => {
                if self.in_phase1() {
                    let total = self.offers[who].entry(item).or_insert(0);
                    *total = total.saturating_sub(quantity);
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
        }
    }
}

pub struct Trades {
    pub next_id: usize,
    pub trades: HashMap<usize, PendingTrade>,
}

impl Trades {
    pub fn begin_trade(&mut self, party: Uid, counterparty: Uid) -> usize {
        let id = self.next_id;
        self.next_id = id.wrapping_add(1);
        self.trades
            .insert(id, PendingTrade::new(party, counterparty));
        id
    }

    pub fn process_trade_action(&mut self, id: usize, who: Uid, msg: TradeActionMsg) {
        if let Some(trade) = self.trades.get_mut(&id) {
            if let Some(party) = trade.which_party(who) {
                trade.process_msg(party, msg);
            } else {
                warn!("An entity who is not a party to trade {} tried to modify it", id);
            }
        } else {
            warn!("Attempt to modify nonexistent trade id {}", id);
        }
    }

    pub fn decline_trade(&mut self, id: usize, who: Uid) {
        if let Some(trade) = self.trades.remove(&id) {
            if let None = trade.which_party(who) {
                warn!("An entity who is not a party to trade {} tried to decline it", id);
                // put it back
                self.trades.insert(id, trade);
            }
        } else {
            warn!("Attempt to decline nonexistent trade id {}", id);
        }
    }
}

impl Default for Trades {
    fn default() -> Trades {
        Trades {
            next_id: 0,
            trades: HashMap::new(),
        }
    }
}
