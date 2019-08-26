//Re-Exports
pub mod item;

// Reexports
pub use self::item::{Item, Tool};

use specs::{Component, HashMapStorage, NullStorage};
use specs_idvs::IDVStorage;

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

    pub fn insert(&mut self, item: Item) -> Option<Item> {
        match self.slots.iter_mut().find(|slot| slot.is_none()) {
            Some(slot) => {
                *slot = Some(item);
                None
            }
            None => Some(item),
        }
    }

    // Get info about an item slot
    pub fn get(&self, cell: usize) -> Option<Item> {
        self.slots.get(cell).cloned().flatten()
    }

    // Insert an item to a slot if its empty
    pub fn swap(&mut self, cell: usize, item: Item) -> Option<Item> {
        //TODO: Check if a slot is empty first.
        self.slots.get_mut(cell).and_then(|cell| cell.replace(item))
    }

    pub fn swap_slots(&mut self, a: usize, b: usize) {
        if a.max(b) < self.slots.len() {
            self.slots.swap(a, b);
        }
    }

    // Remove an item from the slot
    pub fn remove(&mut self, cell: usize) -> Option<Item> {
        self.slots.get_mut(cell).and_then(|item| item.take())
    }
}

impl Default for Inventory {
    fn default() -> Inventory {
        let mut inventory = Inventory {
            slots: vec![None; 24],
        };

        inventory.insert(Item::Tool {
            kind: Tool::Daggers,
            power: 10,
        });
        inventory.insert(Item::Tool {
            kind: Tool::Sword,
            power: 10,
        });
        inventory.insert(Item::Tool {
            kind: Tool::Axe,
            power: 10,
        });
        inventory.insert(Item::Tool {
            kind: Tool::Hammer,
            power: 10,
        });
        inventory.insert(Item::Tool {
            kind: Tool::Bow,
            power: 10,
        });
        for _ in 0..10 {
            inventory.insert(Item::default());
        }

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
