//Re-Exports
pub mod item;

// Reexports
pub use item::{Debug, Item, Tool};

use specs::{Component, HashMapStorage, NullStorage};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<Item>>,
}

impl Inventory {
    pub fn slots(&self) -> &[Option<Item>] {
        &self.slots
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Adds a new item to the first empty slot of the inventory. Returns the item again if no free
    /// slot was found.
    pub fn push(&mut self, item: Item) -> Option<Item> {
        match self.slots.iter_mut().find(|slot| slot.is_none()) {
            Some(slot) => {
                *slot = Some(item);
                None
            }
            None => Some(item),
        }
    }

    /// Replaces an item in a specific slot of the inventory. Returns the old item or the same item again if that slot
    /// was not found.
    pub fn insert(&mut self, cell: usize, item: Item) -> Result<Option<Item>, Item> {
        match self.slots.get_mut(cell) {
            Some(slot) => {
                let old = slot.take();
                *slot = Some(item);
                Ok(old)
            }
            None => Err(item),
        }
    }

    pub fn is_full(&self) -> bool {
        self.slots.iter().all(|slot| slot.is_some())
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
            slots: vec![None; 24],
        };

        inventory.push(Item::Debug(Debug::Boost));
        inventory.push(Item::Tool {
            kind: Tool::Bow,
            power: 10,
        });
        inventory.push(Item::Tool {
            kind: Tool::Daggers,
            power: 10,
        });
        inventory.push(Item::Tool {
            kind: Tool::Sword,
            power: 10,
        });
        inventory.push(Item::Tool {
            kind: Tool::Axe,
            power: 10,
        });
        inventory.push(Item::Tool {
            kind: Tool::Hammer,
            power: 10,
        });

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
