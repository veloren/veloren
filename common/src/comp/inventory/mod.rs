pub mod item;

// Reexports
pub use item::{Consumable, DebugKind, Item, ItemKind, SwordKind, ToolData, ToolKind};

use crate::assets;
use specs::{Component, FlaggedStorage, HashMapStorage};
use specs_idvs::IDVStorage;
use std::ops::Not;

// The limit on distance between the entity and a collectible (squared)
pub const MAX_PICKUP_RANGE_SQR: f32 = 64.0;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<Item>>,
}

/// Errors which the methods on `Inventory` produce
#[derive(Debug)]
pub enum Error {
    /// The inventory is full and items could not be added. The extra items have
    /// been returned.
    Full(Vec<Item>),
}

impl Inventory {
    pub fn slots(&self) -> &[Option<Item>] { &self.slots }

    pub fn len(&self) -> usize { self.slots.len() }

    /// Adds a new item to the first fitting group of the inventory or starts a
    /// new group. Returns the item again if no space was found.
    pub fn push(&mut self, item: Item) -> Option<Item> {
        match item.kind {
            ItemKind::Tool(_) | ItemKind::Armor { .. } => self.add_to_first_empty(item),
            ItemKind::Utility {
                kind: item_kind,
                amount: new_amount,
            } => {
                for slot in &mut self.slots {
                    if slot
                        .as_ref()
                        .map(|s| s.name() == item.name())
                        .unwrap_or(false)
                        && slot
                            .as_ref()
                            .map(|s| s.description() == item.description())
                            .unwrap_or(false)
                    {
                        if let Some(Item {
                            kind: ItemKind::Utility { kind, amount },
                            ..
                        }) = slot
                        {
                            if item_kind == *kind {
                                *amount += new_amount;
                                return None;
                            }
                        }
                    }
                }
                // It didn't work
                self.add_to_first_empty(item)
            },
            ItemKind::Consumable {
                kind: item_kind,
                amount: new_amount,
                ..
            } => {
                for slot in &mut self.slots {
                    if slot
                        .as_ref()
                        .map(|s| s.name() == item.name())
                        .unwrap_or(false)
                        && slot
                            .as_ref()
                            .map(|s| s.description() == item.description())
                            .unwrap_or(false)
                    {
                        if let Some(Item {
                            kind: ItemKind::Consumable { kind, amount, .. },
                            ..
                        }) = slot
                        {
                            if item_kind == *kind {
                                *amount += new_amount;
                                return None;
                            }
                        }
                    }
                }
                // It didn't work
                self.add_to_first_empty(item)
            },
            ItemKind::Ingredient {
                kind: item_kind,
                amount: new_amount,
            } => {
                for slot in &mut self.slots {
                    if slot
                        .as_ref()
                        .map(|s| s.name() == item.name())
                        .unwrap_or(false)
                        && slot
                            .as_ref()
                            .map(|s| s.description() == item.description())
                            .unwrap_or(false)
                    {
                        if let Some(Item {
                            kind: ItemKind::Ingredient { kind, amount },
                            ..
                        }) = slot
                        {
                            if item_kind == *kind {
                                *amount += new_amount;
                                return None;
                            }
                        }
                    }
                }
                // It didn't work
                self.add_to_first_empty(item)
            },
        }
    }

    /// Adds a new item to the first empty slot of the inventory. Returns the
    /// item again if no free slot was found.
    fn add_to_first_empty(&mut self, item: Item) -> Option<Item> {
        match self.slots.iter_mut().find(|slot| slot.is_none()) {
            Some(slot) => {
                *slot = Some(item);
                None
            },
            None => Some(item),
        }
    }

    /// Add a series of items to inventory, returning any which do not fit as an
    /// error.
    pub fn push_all<I: Iterator<Item = Item>>(&mut self, mut items: I) -> Result<(), Error> {
        // Vec doesn't allocate for zero elements so this should be cheap
        let mut leftovers = Vec::new();
        let mut slots = self.slots.iter_mut();
        for item in &mut items {
            if let Some(slot) = slots.find(|slot| slot.is_none()) {
                slot.replace(item);
            } else {
                leftovers.push(item);
            }
        }
        if leftovers.len() > 0 {
            Err(Error::Full(leftovers))
        } else {
            Ok(())
        }
    }

    /// Add a series of items to an inventory without giving duplicates.
    /// (n * m complexity)
    ///
    /// Error if inventory cannot contain the items (is full), returning the
    /// un-added items. This is a lazy inefficient implementation, as it
    /// iterates over the inventory more times than necessary (n^2) and with
    /// the proper structure wouldn't need to iterate at all, but because
    /// this should be fairly cold code, clarity has been favored over
    /// efficiency.
    pub fn push_all_unique<I: Iterator<Item = Item>>(&mut self, mut items: I) -> Result<(), Error> {
        let mut leftovers = Vec::new();
        for item in &mut items {
            if self.contains(&item).not() {
                self.push(item).map(|overflow| leftovers.push(overflow));
            } // else drop item if it was already in
        }
        if leftovers.len() > 0 {
            Err(Error::Full(leftovers))
        } else {
            Ok(())
        }
    }

    /// Replaces an item in a specific slot of the inventory. Returns the old
    /// item or the same item again if that slot was not found.
    pub fn insert(&mut self, cell: usize, item: Item) -> Result<Option<Item>, Item> {
        match self.slots.get_mut(cell) {
            Some(slot) => {
                let old = slot.take();
                *slot = Some(item);
                Ok(old)
            },
            None => Err(item),
        }
    }

    pub fn is_full(&self) -> bool { self.slots.iter().all(|slot| slot.is_some()) }

    /// O(n) count the number of items in this inventory.
    pub fn count(&self) -> usize { self.slots.iter().filter_map(|slot| slot.as_ref()).count() }

    /// O(n) check if an item is in this inventory.
    pub fn contains(&self, item: &Item) -> bool {
        self.slots.iter().any(|slot| slot.as_ref() == Some(item))
    }

    /// Get content of a slot
    pub fn get(&self, cell: usize) -> Option<&Item> {
        self.slots.get(cell).and_then(Option::as_ref)
    }

    /// Swap the items inside of two slots
    pub fn swap_slots(&mut self, a: usize, b: usize) {
        if a.max(b) < self.slots.len() {
            self.slots.swap(a, b);
        }
    }

    /// Remove an item from the slot
    pub fn remove(&mut self, cell: usize) -> Option<Item> {
        self.slots.get_mut(cell).and_then(|item| item.take())
    }

    /// Remove just one item from the slot
    pub fn take(&mut self, cell: usize) -> Option<Item> {
        if let Some(Some(item)) = self.slots.get_mut(cell) {
            let mut return_item = item.clone();
            match &mut item.kind {
                ItemKind::Tool(_) | ItemKind::Armor { .. } => self.remove(cell),
                ItemKind::Utility { kind, amount } => {
                    if *amount <= 1 {
                        self.remove(cell)
                    } else {
                        *amount -= 1;
                        return_item.kind = ItemKind::Utility {
                            kind: *kind,
                            amount: 1,
                        };
                        Some(return_item)
                    }
                },
                ItemKind::Consumable {
                    kind,
                    amount,
                    effect,
                } => {
                    if *amount <= 1 {
                        self.remove(cell)
                    } else {
                        *amount -= 1;
                        return_item.kind = ItemKind::Consumable {
                            kind: *kind,
                            effect: *effect,
                            amount: 1,
                        };
                        Some(return_item)
                    }
                },
                ItemKind::Ingredient { kind, amount } => {
                    if *amount <= 1 {
                        self.remove(cell)
                    } else {
                        *amount -= 1;
                        return_item.kind = ItemKind::Ingredient {
                            kind: *kind,
                            amount: 1,
                        };
                        Some(return_item)
                    }
                },
            }
        } else {
            None
        }
    }
}

impl Default for Inventory {
    fn default() -> Inventory {
        let mut inventory = Inventory {
            slots: vec![None; 25],
        };
        inventory.push(assets::load_expect_cloned("common.items.cheese"));
        inventory.push(assets::load_expect_cloned("common.items.apple"));
        inventory
    }
}

impl Component for Inventory {
    type Storage = HashMapStorage<Self>;
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum InventoryUpdateEvent {
    Init,
    Used,
    Consumed(Consumable),
    Gave,
    Given,
    Swapped,
    Dropped,
    Collected,
    CollectFailed,
    Possession,
    Debug,
}

impl Default for InventoryUpdateEvent {
    fn default() -> Self { Self::Init }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct InventoryUpdate {
    event: InventoryUpdateEvent,
}

impl InventoryUpdate {
    pub fn new(event: InventoryUpdateEvent) -> Self { Self { event } }

    pub fn event(&self) -> InventoryUpdateEvent { self.event }
}

impl Component for InventoryUpdate {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

#[cfg(test)] mod test;
