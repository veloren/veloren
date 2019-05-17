//Library
use specs::{Component, VecStorage};

//Re-Exports
pub mod item;

use item::Item;

 #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<Item>>
}

impl Inventory {
    pub fn new() -> Inventory {
        Inventory {
            slots: vec![None; 24],
        }
    }

    // Get info about an item slot
    pub fn get(&self, cell: usize) -> Option<Item> {
        self.slots.get(cell).cloned().flatten()
    }

    // Insert an item to a slot if its empty
    pub fn insert(&mut self, cell: usize, item: Item) -> Option<Item> {
        self.slots
          .get_mut(cell)
          .and_then(|cell| cell.replace(item))
    }

    // Remove an item from the slot
    pub fn remove(&mut self, cell: usize, item: Item) -> Option<Item> {
        self.slots
          .get_mut(cell)
          .and_then(|cell| item.take().cloned())
    }
}

impl Component for Inventory {
    type Storage = VecStorage<Self>;
}