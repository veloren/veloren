use common::comp::{
    self,
    inventory::item::{item_key::ItemKey, Item},
};
use serde::{Deserialize, Serialize};

use super::HudInfo;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SlotContents {
    Inventory(u64, ItemKey),
    Ability(usize),
}

#[derive(Clone, Default)]
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

    pub fn get(&self, slot: Slot) -> Option<SlotContents> { self.slots[slot as usize].clone() }

    pub fn swap(&mut self, a: Slot, b: Slot) { self.slots.swap(a as usize, b as usize); }

    pub fn clear_slot(&mut self, slot: Slot) { self.slots[slot as usize] = None; }

    #[allow(clippy::only_used_in_recursion)] // false positive
    pub fn add_inventory_link(&mut self, slot: Slot, item: &Item) {
        self.slots[slot as usize] = Some(SlotContents::Inventory(
            item.item_hash(),
            ItemKey::from(item),
        ));
    }

    // TODO: remove pending UI
    // Adds ability slots if missing and should be present
    // Removes ability slots if not there and shouldn't be present
    pub fn maintain_abilities(&mut self, client: &client::Client, info: &HudInfo) {
        use specs::WorldExt;
        if let Some(active_abilities) = client
            .state()
            .ecs()
            .read_storage::<comp::ActiveAbilities>()
            .get(info.viewpoint_entity)
        {
            use common::comp::ability::AuxiliaryAbility;
            for ((i, ability), hotbar_slot) in active_abilities
                .auxiliary_set(
                    client.inventories().get(info.viewpoint_entity),
                    client
                        .state()
                        .read_storage::<comp::SkillSet>()
                        .get(info.viewpoint_entity),
                )
                .iter()
                .enumerate()
                .zip(self.slots.iter_mut())
            {
                if matches!(ability, AuxiliaryAbility::Empty) {
                    if matches!(hotbar_slot, Some(SlotContents::Ability(_))) {
                        // If ability is empty but hotbar shows an ability, clear it
                        *hotbar_slot = None;
                    }
                } else {
                    // If an ability is not empty show it on the hotbar
                    *hotbar_slot = Some(SlotContents::Ability(i));
                }
            }
        } else {
            self.slots
                .iter_mut()
                .filter(|slot| matches!(slot, Some(SlotContents::Ability(_))))
                .for_each(|slot| *slot = None)
        }
    }
}
