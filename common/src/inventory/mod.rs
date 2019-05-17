//Library
use specs::{Component, VecStorage}

// Re-exports
use self::item;

pub struct Inventory {
    pub mut slots: Vec<Option<Item>>
}

impl Inventory {
    fn new() -> Inventory {
        Inventory {
            slots: Vec<Option<Item>> = 
                vec![None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None];
        }
    }

    fn get(&self, cell: u8) -> Option<Item> {
        self.slots.get(cell)
    }
}

impl Component for Inventory {
    type Storage = VecStorage<Self>;
}