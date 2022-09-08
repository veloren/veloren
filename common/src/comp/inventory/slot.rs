use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, convert::TryFrom};

use crate::comp::inventory::{
    item::{armor, armor::ArmorKind, tool, ItemKind},
    loadout::LoadoutSlotId,
};

#[derive(Debug, PartialEq, Eq)]
pub enum SlotError {
    InventoryFull,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Slot {
    Inventory(InvSlotId),
    Equip(EquipSlot),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvSlotId {
    // The index of the loadout item that provides this inventory slot. 0 represents
    // built-in inventory slots
    loadout_idx: u16,
    // The index of the slot within its container
    slot_idx: u16,
}

impl InvSlotId {
    pub const fn new(loadout_idx: u16, slot_idx: u16) -> Self {
        Self {
            loadout_idx,
            slot_idx,
        }
    }

    pub fn idx(&self) -> u32 { (u32::from(self.loadout_idx) << 16) | u32::from(self.slot_idx) }

    pub fn loadout_idx(&self) -> usize { usize::from(self.loadout_idx) }

    pub fn slot_idx(&self) -> usize { usize::from(self.slot_idx) }
}

impl From<LoadoutSlotId> for InvSlotId {
    fn from(loadout_slot_id: LoadoutSlotId) -> Self {
        Self {
            loadout_idx: u16::try_from(loadout_slot_id.loadout_idx + 1).unwrap(),
            slot_idx: u16::try_from(loadout_slot_id.slot_idx).unwrap(),
        }
    }
}

impl PartialOrd for InvSlotId {
    fn partial_cmp(&self, other: &InvSlotId) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl Ord for InvSlotId {
    fn cmp(&self, other: &InvSlotId) -> Ordering { self.idx().cmp(&other.idx()) }
}

pub(super) enum SlotId {
    Inventory(usize),
    Loadout(LoadoutSlotId),
}

impl From<InvSlotId> for SlotId {
    fn from(inv_slot_id: InvSlotId) -> Self {
        match inv_slot_id.loadout_idx {
            0 => SlotId::Inventory(inv_slot_id.slot_idx()),
            _ => SlotId::Loadout(LoadoutSlotId {
                loadout_idx: inv_slot_id.loadout_idx() - 1,
                slot_idx: inv_slot_id.slot_idx(),
            }),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum EquipSlot {
    Armor(ArmorSlot),
    ActiveMainhand,
    ActiveOffhand,
    InactiveMainhand,
    InactiveOffhand,
    Lantern,
    Glider,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum ArmorSlot {
    Head,
    Neck,
    Shoulders,
    Chest,
    Hands,
    Ring1,
    Ring2,
    Back,
    Belt,
    Legs,
    Feet,
    Tabard,
    Bag1,
    Bag2,
    Bag3,
    Bag4,
}

impl Slot {
    pub fn can_hold(self, item_kind: &ItemKind) -> bool {
        match (self, item_kind) {
            (Self::Inventory(_), _) => true,
            (Self::Equip(slot), item_kind) => slot.can_hold(item_kind),
        }
    }
}

impl EquipSlot {
    pub fn can_hold(self, item_kind: &ItemKind) -> bool {
        match (self, item_kind) {
            (Self::Armor(slot), ItemKind::Armor(armor::Armor { kind, .. })) => slot.can_hold(kind),
            (Self::ActiveMainhand, ItemKind::Tool(_)) => true,
            (Self::ActiveOffhand, ItemKind::Tool(tool)) => matches!(tool.hands, tool::Hands::One),
            (Self::InactiveMainhand, ItemKind::Tool(_)) => true,
            (Self::InactiveOffhand, ItemKind::Tool(tool)) => matches!(tool.hands, tool::Hands::One),
            (Self::Lantern, ItemKind::Lantern(_)) => true,
            (Self::Glider, ItemKind::Glider) => true,
            _ => false,
        }
    }
}

impl ArmorSlot {
    fn can_hold(self, armor: &ArmorKind) -> bool {
        matches!(
            (self, armor),
            (Self::Head, ArmorKind::Head)
                | (Self::Neck, ArmorKind::Neck)
                | (Self::Shoulders, ArmorKind::Shoulder)
                | (Self::Chest, ArmorKind::Chest)
                | (Self::Hands, ArmorKind::Hand)
                | (Self::Ring1, ArmorKind::Ring)
                | (Self::Ring2, ArmorKind::Ring)
                | (Self::Back, ArmorKind::Back)
                | (Self::Belt, ArmorKind::Belt)
                | (Self::Legs, ArmorKind::Pants)
                | (Self::Feet, ArmorKind::Foot)
                | (Self::Tabard, ArmorKind::Tabard)
                | (Self::Bag1, ArmorKind::Bag)
                | (Self::Bag2, ArmorKind::Bag)
                | (Self::Bag3, ArmorKind::Bag)
                | (Self::Bag4, ArmorKind::Bag)
        )
    }
}
