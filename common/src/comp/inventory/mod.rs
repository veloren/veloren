pub mod item;

// Reexports
pub use item::{Debug, Item, ItemKind, SwordKind, ToolData, ToolKind};

use crate::assets;
use specs::{Component, HashMapStorage, NullStorage};
use std::ops::Not;

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

    /// Adds a new item to the first empty slot of the inventory. Returns the
    /// item again if no free slot was found.
    pub fn push(&mut self, item: Item) -> Option<Item> {
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

// ForceUpdate
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct InventoryUpdate;

impl Component for InventoryUpdate {
    type Storage = NullStorage<Self>;
}

#[cfg(test)] mod test;
