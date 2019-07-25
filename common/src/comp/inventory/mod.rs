//Re-Exports
pub mod item;

// Reexports
pub use self::item::Item;

use specs::{Component, NullStorage, HashMapStorage};
use specs_idvs::IDVStorage;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<Item>>,
}

impl Inventory {
    // Get info about an item slot
    pub fn get(&self, cell: usize) -> Option<Item> {
        self.slots.get(cell).cloned().flatten()
    }

    // Insert an item to a slot if its empty
    pub fn swap(&mut self, cell: usize, item: Item) -> Option<Item> {
        //TODO: Check if a slot is empty first.
        self.slots.get_mut(cell).and_then(|cell| cell.replace(item))
    }

    // Remove an item from the slot
    pub fn remove(&mut self, cell: usize) -> Option<Item> {
        self.slots.get_mut(cell).and_then(|item| item.take())
    }
}

impl Default for Inventory {
    fn default() -> Inventory {
        Inventory {
            slots: vec![None; 24],
        }
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
