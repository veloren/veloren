use specs::{Component, VecStorage};

//Re-Exports
pub mod item;

use item::Item;
use std::mem::swap;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<Item>>,
}

impl Inventory {
    pub fn new() -> Inventory {
        Inventory {
            slots: vec![None; 24],
        }
    }

    // Get info about an item slot
    pub fn get(&self, cell: usize) -> Option<Option<Item>> {
        self.slots.get(cell).cloned()
    }

    // Insert an item to a slot if its empty
    pub fn swap(&mut self, cell: usize, item: Item) -> Option<Item> {
        //TODO: Check if a slot is empty first.
        self.slots.get_mut(cell).and_then(|cell| cell.replace(item))
    }

    // Remove an item from the slot
    pub fn remove(&mut self, cell: usize, item: Item) -> Option<Item> {
        let mut tmp_item = Some(item);

        if let Some(old_item) = self.slots.get_mut(cell) {
            swap(old_item, &mut tmp_item);
        }

        tmp_item
    }
}

impl Component for Inventory {
    type Storage = VecStorage<Self>;
}
