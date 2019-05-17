//Library
use specs::{Component, VecStorage};

//Re-Exports
use super::item::Item;

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

    fn get(&self, cell: u8) -> Option<Item> {
        self.slots.get_mut(cell)
    }

    fn insert(&mut self, cell: u8, item: Item) -> Option<Item> {
        self.slots
          .get_mut(cell)
          .and_then(|cell| cell.replace(item))
    }

    fn remove(&mut self, cell: u8, item: Item) -> Option<Item> {
        self.slots
          .get_mut(cell)
          .and_then(|cell| cell.take(item))
    }
}

impl Component for Inventory {
    type Storage = VecStorage<Self>;
}