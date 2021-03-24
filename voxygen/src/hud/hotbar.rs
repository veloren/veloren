use crate::hud::slots::EquipSlot;
use common::comp::{
    item::{tool::Hands, ItemKind},
    slot::InvSlotId,
    Inventory,
};
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
    Ability3,
    Ability4,
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

    pub fn add_inventory_link(&mut self, slot: Slot, inventory_pos: InvSlotId) {
        self.slots[slot as usize] = Some(SlotContents::Inventory(inventory_pos));
    }

    // TODO: remove
    // Adds ability3 slot if it is missing and should be present
    // Removes if it is there and shouldn't be present
    #[allow(clippy::unnested_or_patterns)] // TODO: Pending review in #587
    pub fn maintain_ability3(&mut self, client: &client::Client) {
        use specs::WorldExt;
        let inventories = client.state().ecs().read_storage::<Inventory>();
        let inventory = inventories.get(client.entity());
        let stats = client.state().ecs().read_storage::<common::comp::Stats>();
        let stat = stats.get(client.entity());

        let hands =
            |equip_slot| match inventory.and_then(|i| i.equipped(equip_slot).map(|i| i.kind())) {
                Some(ItemKind::Tool(tool)) => Some(tool.hands),
                _ => None,
            };

        let equip_slot = match (hands(EquipSlot::Mainhand), hands(EquipSlot::Offhand)) {
            (Some(_), _) => Some(EquipSlot::Mainhand),
            (_, Some(_)) => Some(EquipSlot::Offhand),
            _ => None,
        };

        let should_be_present = if let (Some(inventory), Some(stat), Some(equip_slot)) =
            (inventory, stat, equip_slot)
        {
            inventory.equipped(equip_slot).map_or(false, |i| {
                i.item_config_expect()
                    .abilities
                    .abilities
                    .get(0)
                    .as_ref()
                    .map_or(false, |(s, _)| {
                        s.map_or(true, |s| stat.skill_set.has_skill(s))
                    })
            })
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

    pub fn maintain_ability4(&mut self, client: &client::Client) {
        use specs::WorldExt;
        let inventories = client.state().ecs().read_storage::<Inventory>();
        let inventory = inventories.get(client.entity());
        let stats = client.state().ecs().read_storage::<common::comp::Stats>();
        let stat = stats.get(client.entity());
        let should_be_present = if let (Some(inventory), Some(stat)) = (inventory, stat) {
            let hands = |equip_slot| match inventory.equipped(equip_slot).map(|i| i.kind()) {
                Some(ItemKind::Tool(tool)) => Some(tool.hands),
                _ => None,
            };

            let active_tool_hands = hands(EquipSlot::Mainhand);
            let second_tool_hands = hands(EquipSlot::Offhand);

            let (equip_slot, skill_index) = match (active_tool_hands, second_tool_hands) {
                (Some(Hands::Two), _) => (Some(EquipSlot::Mainhand), 1),
                (Some(_), Some(Hands::One)) => (Some(EquipSlot::Offhand), 0),
                (Some(Hands::One), _) => (Some(EquipSlot::Mainhand), 1),
                (None, Some(_)) => (Some(EquipSlot::Offhand), 1),
                (_, _) => (None, 0),
            };

            if let Some(equip_slot) = equip_slot {
                inventory.equipped(equip_slot).map_or(false, |i| {
                    i.item_config_expect()
                        .abilities
                        .abilities
                        .get(skill_index)
                        .as_ref()
                        .map_or(false, |(s, _)| {
                            s.map_or(true, |s| stat.skill_set.has_skill(s))
                        })
                })
            } else {
                false
            }
        } else {
            false
        };

        if should_be_present {
            if !self
                .slots
                .iter()
                .any(|s| matches!(s, Some(SlotContents::Ability4)))
            {
                self.slots[1] = Some(SlotContents::Ability4);
            }
        } else {
            self.slots
                .iter_mut()
                .filter(|s| matches!(s, Some(SlotContents::Ability4)))
                .for_each(|s| *s = None)
        }
    }
}
