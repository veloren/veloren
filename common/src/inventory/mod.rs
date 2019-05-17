//Library
use specs::{Component, VecStorage};

//Re-Exports
pub mod item;

use item::Item;

pub struct Inventory {
    pub slots: Vec<Option<Item>>
}

impl Inventory {
    fn new() -> Inventory {
        Inventory {
            slots: Vec<Option<Item>> = 
                vec![None; 24];
        }
    }

    // Get info about an item slot
    fn get(&self, cell: usize) -> Option<Item> {
        self.slots.get_mut(cell)
    }

    // Insert an item to a slot if its empty
    fn insert(&mut self, cell: usize, item: Item) -> Option<Item> {
        self.slots
          .get_mut(cell)
          .and_then(|cell| cell.replace(item))
    }

    // Remove an item from the slot
    fn remove(&mut self, cell: usize, item: Item) -> Option<Item> {
        self.slots
          .get_mut(cell)
          .and_then(|cell| cell.take(item))
    }
}

impl Component for Inventory {
    type Storage = VecStorage<Self>;
}