use std::cmp::Ordering;

use crate::{
    comp::inventory::{
        item::ItemDefinitionIdOwned, slot::InvSlotId, trade_pricing::TradePricing, Inventory,
    },
    terrain::BiomeKind,
    uid::Uid,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use tracing::{trace, warn};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradePhase {
    Mutate,
    Review,
    Complete,
}

/// Clients submit `TradeAction` to the server, which adds the Uid of the
/// player out-of-band (i.e. without trusting the client to say who it's
/// accepting on behalf of)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeAction {
    AddItem {
        item: InvSlotId,
        quantity: u32,
        ours: bool,
    },
    RemoveItem {
        item: InvSlotId,
        quantity: u32,
        ours: bool,
    },
    /// Accept needs the phase indicator to avoid progressing too far in the
    /// trade if there's latency and a player presses the accept button
    /// multiple times
    Accept(TradePhase),
    Decline,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
/// swap items in their inventory during a trade, they could mutate the trade,
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingTrade {
    /// `parties[0]` is the entity that initiated the trade, parties[1] is the
    /// other entity that's being traded with
    pub parties: [Uid; 2],
    /// `offers[i]` represents the items and quantities of the party i's items
    /// being offered
    pub offers: [HashMap<InvSlotId, u32>; 2],
    /// The current phase of the trade
    pub phase: TradePhase,
    /// `accept_flags` indicate that which parties wish to proceed to the next
    /// phase of the trade
    pub accept_flags: [bool; 2],
}

impl TradePhase {
    fn next(self) -> TradePhase {
        match self {
            TradePhase::Mutate => TradePhase::Review,
            TradePhase::Review => TradePhase::Complete,
            TradePhase::Complete => TradePhase::Complete,
        }
    }
}

impl TradeAction {
    pub fn item(item: InvSlotId, delta: i32, ours: bool) -> Option<Self> {
        match delta.cmp(&0) {
            Ordering::Equal => None,
            Ordering::Less => Some(TradeAction::RemoveItem {
                item,
                ours,
                quantity: -delta as u32,
            }),
            Ordering::Greater => Some(TradeAction::AddItem {
                item,
                ours,
                quantity: delta as u32,
            }),
        }
    }
}

impl PendingTrade {
    pub fn new(party: Uid, counterparty: Uid) -> PendingTrade {
        PendingTrade {
            parties: [party, counterparty],
            offers: [HashMap::new(), HashMap::new()],
            phase: TradePhase::Mutate,
            accept_flags: [false, false],
        }
    }

    pub fn phase(&self) -> TradePhase { self.phase }

    pub fn should_commit(&self) -> bool { matches!(self.phase, TradePhase::Complete) }

    pub fn which_party(&self, party: Uid) -> Option<usize> {
        self.parties
            .iter()
            .enumerate()
            .find(|(_, x)| **x == party)
            .map(|(i, _)| i)
    }

    pub fn is_empty_trade(&self) -> bool { self.offers[0].is_empty() && self.offers[1].is_empty() }

    /// Invariants:
    /// - A party is never shown as offering more of an item than they own
    /// - Offers with a quantity of zero get removed from the trade
    /// - Modifications can only happen in phase 1
    /// - Whenever a trade is modified, both accept flags get reset
    /// - Accept flags only get set for the current phase
    pub fn process_trade_action(
        &mut self,
        mut who: usize,
        action: TradeAction,
        inventories: &[&Inventory],
    ) {
        use TradeAction::*;
        match action {
            AddItem {
                item,
                quantity: delta,
                ours,
            } => {
                if self.phase() == TradePhase::Mutate && delta > 0 {
                    if !ours {
                        who = 1 - who;
                    }
                    let total = self.offers[who].entry(item).or_insert(0);
                    let owned_quantity =
                        inventories[who].get(item).map(|i| i.amount()).unwrap_or(0);
                    *total = total.saturating_add(delta).min(owned_quantity);
                    self.accept_flags = [false, false];
                }
            },
            RemoveItem {
                item,
                quantity: delta,
                ours,
            } => {
                if self.phase() == TradePhase::Mutate {
                    if !ours {
                        who = 1 - who;
                    }
                    self.offers[who]
                        .entry(item)
                        .and_replace_entry_with(|_, mut total| {
                            total = total.saturating_sub(delta);
                            if total > 0 { Some(total) } else { None }
                        });
                    self.accept_flags = [false, false];
                }
            },
            Accept(phase) => {
                if self.phase == phase && !self.is_empty_trade() {
                    self.accept_flags[who] = true;
                }
                if self.accept_flags[0] && self.accept_flags[1] {
                    self.phase = self.phase.next();
                    self.accept_flags = [false, false];
                }
            },
            Decline => {},
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TradeId(usize);

pub struct Trades {
    pub next_id: TradeId,
    pub trades: HashMap<TradeId, PendingTrade>,
    pub entity_trades: HashMap<Uid, TradeId>,
}

impl Trades {
    pub fn begin_trade(&mut self, party: Uid, counterparty: Uid) -> TradeId {
        let id = self.next_id;
        self.next_id = TradeId(id.0.wrapping_add(1));
        self.trades
            .insert(id, PendingTrade::new(party, counterparty));
        self.entity_trades.insert(party, id);
        self.entity_trades.insert(counterparty, id);
        id
    }

    pub fn process_trade_action<'a, F: Fn(Uid) -> Option<&'a Inventory>>(
        &mut self,
        id: TradeId,
        who: Uid,
        action: TradeAction,
        get_inventory: F,
    ) {
        trace!("for trade id {:?}, message {:?}", id, action);
        if let Some(trade) = self.trades.get_mut(&id) {
            if let Some(party) = trade.which_party(who) {
                let mut inventories = Vec::new();
                for party in trade.parties.iter() {
                    match get_inventory(*party) {
                        Some(inventory) => inventories.push(inventory),
                        None => return,
                    }
                }
                trade.process_trade_action(party, action, &inventories);
            } else {
                warn!(
                    "An entity who is not a party to trade {:?} tried to modify it",
                    id
                );
            }
        } else {
            warn!("Attempt to modify nonexistent trade id {:?}", id);
        }
    }

    pub fn decline_trade(&mut self, id: TradeId, who: Uid) -> Option<Uid> {
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
                        "An entity who is not a party to trade {:?} tried to decline it",
                        id
                    );
                    // put it back
                    self.trades.insert(id, trade);
                },
            }
        } else {
            warn!("Attempt to decline nonexistent trade id {:?}", id);
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
        self.in_trade_with_property(uid, |trade| trade.phase() != TradePhase::Mutate)
    }

    pub fn in_mutable_trade(&self, uid: &Uid) -> bool {
        self.in_trade_with_property(uid, |trade| trade.phase() == TradePhase::Mutate)
    }

    pub fn implicit_mutation_occurred(&mut self, uid: &Uid) {
        if let Some(trade_id) = self.entity_trades.get(uid) {
            self.trades
                .get_mut(trade_id)
                .map(|trade| trade.accept_flags = [false, false]);
        }
    }
}

impl Default for Trades {
    fn default() -> Trades {
        Trades {
            next_id: TradeId(0),
            trades: HashMap::new(),
            entity_trades: HashMap::new(),
        }
    }
}

// we need this declaration in common for Merchant loadout creation, it is not
// directly related to trade between entities, but between sites (more abstract)
// economical information
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize, EnumIter)]
pub enum Good {
    Territory(BiomeKind),
    Flour,
    Meat,
    Terrain(BiomeKind),
    Transportation,
    Food,
    Wood,
    Stone,
    Tools, // weapons, farming tools
    Armor,
    Ingredients, // raw material for Armor+Tools+Potions
    Potions,
    Coin, // exchange material across sites
    RoadSecurity,
}

impl Default for Good {
    fn default() -> Self {
        Good::Terrain(BiomeKind::Void) // Arbitrary
    }
}

impl Good {
    /// The discounting factor applied when selling goods back to a merchant
    pub fn trade_margin(&self) -> f32 {
        match self {
            Good::Tools | Good::Armor => 0.5,
            Good::Food | Good::Potions | Good::Ingredients => 0.75,
            Good::Coin => 1.0,
            // Certain abstract goods (like Territory) shouldn't be attached to concrete items;
            // give a sale price of 0 if the player is trying to sell a concrete item that somehow
            // has one of these categories
            _ => 0.0,
        }
    }
}

// ideally this would be a real Id<Site> but that is from the world crate
pub type SiteId = u64;

#[derive(Clone, Debug)]
pub struct SiteInformation {
    pub id: SiteId,
    pub unconsumed_stock: HashMap<Good, f32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SitePrices {
    pub values: HashMap<Good, f32>,
}

impl SitePrices {
    pub fn balance(
        &self,
        offers: &[HashMap<InvSlotId, u32>; 2],
        inventories: &[Option<ReducedInventory>; 2],
        who: usize,
        reduce: bool,
    ) -> f32 {
        offers[who]
            .iter()
            .map(|(slot, amount)| {
                inventories[who]
                    .as_ref()
                    .and_then(|ri| {
                        ri.inventory.get(slot).map(|item| {
                            if let Some(vec) = TradePricing::get_materials(&item.name.as_ref()) {
                                vec.iter()
                                    .map(|(amount2, material)| {
                                        self.values.get(material).copied().unwrap_or_default()
                                            * *amount2
                                            * (if reduce { material.trade_margin() } else { 1.0 })
                                    })
                                    .sum::<f32>()
                                    * (*amount as f32)
                            } else {
                                0.0
                            }
                        })
                    })
                    .unwrap_or_default()
            })
            .sum()
    }
}

#[derive(Clone, Debug)]
pub struct ReducedInventoryItem {
    pub name: ItemDefinitionIdOwned,
    pub amount: u32,
}

#[derive(Clone, Debug, Default)]
pub struct ReducedInventory {
    pub inventory: HashMap<InvSlotId, ReducedInventoryItem>,
}

impl ReducedInventory {
    pub fn from(inventory: &Inventory) -> Self {
        let items = inventory
            .slots_with_id()
            .filter(|(_, it)| it.is_some())
            .map(|(sl, it)| {
                (sl, ReducedInventoryItem {
                    name: it.as_ref().unwrap().item_definition_id().to_owned(),
                    amount: it.as_ref().unwrap().amount(),
                })
            })
            .collect();
        Self { inventory: items }
    }
}
