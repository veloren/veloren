use common::comp::slot::InvSlotId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq)]
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
    Inventory(InvSlotId),
    Ability(usize),
}

#[derive(Clone, Copy, Default)]
pub struct State {
    pub slots: [Option<SlotContents>; 10],
    inputs: [bool; 10],
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

    pub fn add_inventory_link(&mut self, slot: Slot, inventory_pos: InvSlotId) {
        self.slots[slot as usize] = Some(SlotContents::Inventory(inventory_pos));
    }

    // TODO: remove pending UI
    // Adds ability slots if missing and should be present
    // Removes ability slots if not there and shouldn't be present
    pub fn maintain_abilities(&mut self, client: &client::Client) {
        use specs::WorldExt;
        if let Some(ability_pool) = client
            .state()
            .ecs()
            .read_storage::<common::comp::AbilityPool>()
            .get(client.entity())
        {
            use common::comp::Ability;
            for (i, ability) in ability_pool.abilities.iter().enumerate() {
                if matches!(ability, Ability::Empty) {
                    self.slots
                        .iter_mut()
                        .filter(|s| matches!(s, Some(SlotContents::Ability(index)) if *index == i))
                        .for_each(|s| *s = None)
                } else if let Some(slot) = self
                    .slots
                    .iter_mut()
                    .find(|s| !matches!(s, Some(SlotContents::Ability(index)) if *index != i))
                {
                    *slot = Some(SlotContents::Ability(i));
                }
            }
        }
    }
}
