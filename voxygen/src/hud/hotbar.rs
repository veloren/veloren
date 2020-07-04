use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq)]
pub enum Slot {
    One = 0,
    Two = 1,
    Three = 2,
    Four = 3,
    Five = 4,
    Six = 5,
    Seven = 6,
    Eight = 7,
    Nine = 8,
    Ten = 9,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum SlotContents {
    Inventory(usize),
    Ability3,
}

#[derive(Clone, Debug)]
pub struct State {
    pub slots: [Option<SlotContents>; 10],
    inputs: [bool; 10],
}

impl Default for State {
    fn default() -> Self {
        Self {
            slots: [None; 10],
            inputs: [false; 10],
        }
    }
}

impl State {
    pub fn new(slots: [Option<SlotContents>; 10]) -> Self {
        Self {
            slots,
            inputs: [false; 10],
        }
    }

    /// Returns true if the button was just pressed
    pub fn process_input(&mut self, slot: Slot, state: bool) -> bool {
        let slot = slot as usize;
        let just_pressed = !self.inputs[slot] && state;
        self.inputs[slot] = state;
        just_pressed
    }

    pub fn get(&self, slot: Slot) -> Option<SlotContents> { self.slots[slot as usize] }

    pub fn swap(&mut self, a: Slot, b: Slot) { self.slots.swap(a as usize, b as usize); }

    pub fn clear_slot(&mut self, slot: Slot) { self.slots[slot as usize] = None; }

    pub fn add_inventory_link(&mut self, slot: Slot, inventory_index: usize) {
        self.slots[slot as usize] = Some(SlotContents::Inventory(inventory_index));
    }

    // TODO: remove
    // Adds ability3 slot if it is missing and should be present
    // Removes if it is there and shouldn't be present
    #[allow(clippy::unnested_or_patterns)] // TODO: Pending review in #587
    pub fn maintain_ability3(&mut self, client: &client::Client) {
        use specs::WorldExt;
        let loadouts = client.state().ecs().read_storage::<common::comp::Loadout>();
        let loadout = loadouts.get(client.entity());
        let should_be_present = if let Some(loadout) = loadout {
            loadout
                .active_item
                .as_ref()
                .map(|i| &i.item.kind)
                .filter(|kind| {
                    use common::comp::item::{
                        tool::{DebugKind, StaffKind, Tool, ToolKind},
                        ItemKind,
                    };
                    matches!(
                        kind,
                        ItemKind::Tool(Tool {
                            kind: ToolKind::Staff(StaffKind::BasicStaff),
                            ..
                        }) | ItemKind::Tool(Tool {
                            kind: ToolKind::Debug(DebugKind::Boost),
                            ..
                        })
                    )
                })
                .is_some()
        } else {
            false
        };

        if should_be_present {
            if !self
                .slots
                .iter()
                .any(|s| matches!(s, Some(SlotContents::Ability3)))
            {
                self.slots[0] = Some(SlotContents::Ability3);
            }
        } else {
            self.slots
                .iter_mut()
                .filter(|s| matches!(s, Some(SlotContents::Ability3)))
                .for_each(|s| *s = None)
        }
    }
}
