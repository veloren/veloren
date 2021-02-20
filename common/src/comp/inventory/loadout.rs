use crate::comp::{
    inventory::{
        item::ItemKind,
        slot::{ArmorSlot, EquipSlot},
        InvSlot,
    },
    Item,
};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use tracing::warn;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Loadout {
    slots: Vec<LoadoutSlot>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct LoadoutSlot {
    /// The EquipSlot that this slot represents
    pub(super) equip_slot: EquipSlot,
    /// The contents of the slot
    slot: InvSlot,
    /// The unique string that represents this loadout slot in the database (not
    /// synced to clients)
    #[serde(skip)]
    persistence_key: String,
}

impl LoadoutSlot {
    fn new(equip_slot: EquipSlot, persistence_key: String) -> LoadoutSlot {
        LoadoutSlot {
            equip_slot,
            slot: None,
            persistence_key,
        }
    }
}

pub(super) struct LoadoutSlotId {
    // The index of the loadout item that provides this inventory slot.
    pub loadout_idx: usize,
    // The index of the slot within its container
    pub slot_idx: usize,
}

pub enum LoadoutError {
    InvalidPersistenceKey,
    NoParentAtSlot,
}

impl Loadout {
    pub(super) fn new_empty() -> Self {
        Self {
            slots: vec![
                (EquipSlot::Lantern, "lantern".to_string()),
                (EquipSlot::Glider, "glider".to_string()),
                (
                    EquipSlot::Armor(ArmorSlot::Shoulders),
                    "shoulder".to_string(),
                ),
                (EquipSlot::Armor(ArmorSlot::Chest), "chest".to_string()),
                (EquipSlot::Armor(ArmorSlot::Belt), "belt".to_string()),
                (EquipSlot::Armor(ArmorSlot::Hands), "hand".to_string()),
                (EquipSlot::Armor(ArmorSlot::Legs), "pants".to_string()),
                (EquipSlot::Armor(ArmorSlot::Feet), "foot".to_string()),
                (EquipSlot::Armor(ArmorSlot::Back), "back".to_string()),
                (EquipSlot::Armor(ArmorSlot::Ring1), "ring1".to_string()),
                (EquipSlot::Armor(ArmorSlot::Ring2), "ring2".to_string()),
                (EquipSlot::Armor(ArmorSlot::Neck), "neck".to_string()),
                (EquipSlot::Armor(ArmorSlot::Head), "head".to_string()),
                (EquipSlot::Armor(ArmorSlot::Tabard), "tabard".to_string()),
                (EquipSlot::Armor(ArmorSlot::Bag1), "bag1".to_string()),
                (EquipSlot::Armor(ArmorSlot::Bag2), "bag2".to_string()),
                (EquipSlot::Armor(ArmorSlot::Bag3), "bag3".to_string()),
                (EquipSlot::Armor(ArmorSlot::Bag4), "bag4".to_string()),
                (EquipSlot::Mainhand, "active_item".to_string()),
                (EquipSlot::Offhand, "second_item".to_string()),
            ]
            .into_iter()
            .map(|(equip_slot, persistence_key)| LoadoutSlot::new(equip_slot, persistence_key))
            .collect(),
        }
    }

    /// Replaces the item in the Loadout slot that corresponds to the given
    /// EquipSlot and returns the previous item if any
    pub(super) fn swap(&mut self, equip_slot: EquipSlot, item: Option<Item>) -> Option<Item> {
        self.slots
            .iter_mut()
            .find(|x| x.equip_slot == equip_slot)
            .and_then(|x| core::mem::replace(&mut x.slot, item))
    }

    /// Returns a reference to the item (if any) equipped in the given EquipSlot
    pub(super) fn equipped(&self, equip_slot: EquipSlot) -> Option<&Item> {
        self.slot(equip_slot).and_then(|x| x.slot.as_ref())
    }

    fn slot(&self, equip_slot: EquipSlot) -> Option<&LoadoutSlot> {
        self.slots
            .iter()
            .find(|loadout_slot| loadout_slot.equip_slot == equip_slot)
    }

    pub(super) fn loadout_idx_for_equip_slot(&self, equip_slot: EquipSlot) -> Option<usize> {
        self.slots
            .iter()
            .position(|loadout_slot| loadout_slot.equip_slot == equip_slot)
    }

    /// Returns all loadout items paired with their persistence key
    pub(super) fn items_with_persistence_key(&self) -> impl Iterator<Item = (&str, Option<&Item>)> {
        self.slots
            .iter()
            .map(|x| (x.persistence_key.as_str(), x.slot.as_ref()))
    }

    /// Sets a loadout item in the correct slot using its persistence key. Any
    /// item that already exists in the slot is lost.
    pub fn set_item_at_slot_using_persistence_key(
        &mut self,
        persistence_key: &str,
        item: Item,
    ) -> Result<(), LoadoutError> {
        if let Some(slot) = self
            .slots
            .iter_mut()
            .find(|x| x.persistence_key == persistence_key)
        {
            slot.slot = Some(item);
            Ok(())
        } else {
            Err(LoadoutError::InvalidPersistenceKey)
        }
    }

    pub fn update_item_at_slot_using_persistence_key<F: FnOnce(&mut Item)>(
        &mut self,
        persistence_key: &str,
        f: F,
    ) -> Result<(), LoadoutError> {
        self.slots
            .iter_mut()
            .find(|loadout_slot| loadout_slot.persistence_key == persistence_key)
            .map_or(Err(LoadoutError::InvalidPersistenceKey), |loadout_slot| {
                loadout_slot
                    .slot
                    .as_mut()
                    .map_or(Err(LoadoutError::NoParentAtSlot), |item| {
                        f(item);
                        Ok(())
                    })
            })
    }

    /// Swaps the contents of two loadout slots
    pub(super) fn swap_slots(&mut self, equip_slot_a: EquipSlot, equip_slot_b: EquipSlot) {
        if self.slot(equip_slot_b).is_none() || self.slot(equip_slot_b).is_none() {
            // Currently all loadouts contain slots for all EquipSlots so this can never
            // happen, but if loadouts with alternate slot combinations are
            // introduced then it could.
            warn!("Cannot swap slots for non-existent equip slot");
            return;
        }

        let item_a = self.swap(equip_slot_a, None);
        let item_b = self.swap(equip_slot_b, None);

        // Check if items can go in the other slots
        if item_a
            .as_ref()
            .map_or(true, |i| equip_slot_b.can_hold(&i.kind()))
            && item_b
                .as_ref()
                .map_or(true, |i| equip_slot_a.can_hold(&i.kind()))
        {
            // Swap
            self.swap(equip_slot_b, item_a).unwrap_none();
            self.swap(equip_slot_a, item_b).unwrap_none();
        } else {
            // Otherwise put the items back
            self.swap(equip_slot_a, item_a).unwrap_none();
            self.swap(equip_slot_b, item_b).unwrap_none();
        }
    }

    /// Gets a slot that an item of a particular `ItemKind` can be equipped
    /// into. The first empty slot compatible with the item will be
    /// returned, or if there are no free slots then the first occupied slot
    /// will be returned. The bool part of the tuple indicates whether an item
    /// is already equipped in the slot.
    pub(super) fn get_slot_to_equip_into(&self, item_kind: &ItemKind) -> Option<EquipSlot> {
        let mut suitable_slots = self
            .slots
            .iter()
            .filter(|s| s.equip_slot.can_hold(item_kind));

        let first = suitable_slots.next();

        first
            .into_iter()
            .chain(suitable_slots)
            .find(|loadout_slot| loadout_slot.slot.is_none())
            .map(|x| x.equip_slot)
            .or_else(|| first.map(|x| x.equip_slot))
    }

    /// Returns the `InvSlot` for a given `LoadoutSlotId`
    pub(super) fn inv_slot(&self, loadout_slot_id: LoadoutSlotId) -> Option<&InvSlot> {
        self.slots
            .get(loadout_slot_id.loadout_idx)
            .and_then(|loadout_slot| loadout_slot.slot.as_ref())
            .and_then(|item| item.slot(loadout_slot_id.slot_idx))
    }

    /// Returns the `InvSlot` for a given `LoadoutSlotId`
    pub(super) fn inv_slot_mut(&mut self, loadout_slot_id: LoadoutSlotId) -> Option<&mut InvSlot> {
        self.slots
            .get_mut(loadout_slot_id.loadout_idx)
            .and_then(|loadout_slot| loadout_slot.slot.as_mut())
            .and_then(|item| item.slot_mut(loadout_slot_id.slot_idx))
    }

    /// Returns all inventory slots provided by equipped loadout items, along
    /// with their `LoadoutSlotId`
    pub(super) fn inv_slots_with_id(&self) -> impl Iterator<Item = (LoadoutSlotId, &InvSlot)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, loadout_slot)| {
                loadout_slot.slot.as_ref().map(|item| (i, item.slots()))
            })
            .flat_map(|(loadout_slot_index, loadout_slots)| {
                loadout_slots
                    .iter()
                    .enumerate()
                    .map(move |(item_slot_index, inv_slot)| {
                        (
                            LoadoutSlotId {
                                loadout_idx: loadout_slot_index,
                                slot_idx: item_slot_index,
                            },
                            inv_slot,
                        )
                    })
            })
    }

    /// Returns all inventory slots provided by equipped loadout items
    pub(super) fn inv_slots_mut(&mut self) -> impl Iterator<Item = &mut InvSlot> {
        self.slots.iter_mut()
            .filter_map(|x| x.slot.as_mut().map(|item| item.slots_mut()))  // Discard loadout items that have no slots of their own
            .flat_map(|loadout_slots| loadout_slots.iter_mut()) //Collapse iter of Vec<InvSlot> to iter of InvSlot 
    }

    /// Gets the range of loadout-provided inventory slot indexes that are
    /// provided by the item in the given `EquipSlot`
    pub(super) fn slot_range_for_equip_slot(&self, equip_slot: EquipSlot) -> Option<Range<usize>> {
        self.slots
            .iter()
            .map(|loadout_slot| {
                (
                    loadout_slot.equip_slot,
                    loadout_slot
                        .slot
                        .as_ref()
                        .map_or(0, |item| item.slots().len()),
                )
            })
            .scan(0, |acc_len, (equip_slot, len)| {
                let res = Some((equip_slot, len, *acc_len));
                *acc_len += len;
                res
            })
            .find(|(e, len, _)| *e == equip_slot && len > &0)
            .map(|(_, slot_len, start)| start..start + slot_len)
    }

    /// Attempts to equip the item into a compatible, unpopulated loadout slot.
    /// If no slot is available the item is returned.
    #[must_use = "Returned item will be lost if not used"]
    pub(super) fn try_equip(&mut self, item: Item) -> Result<(), Item> {
        if let Some(loadout_slot) = self
            .slots
            .iter_mut()
            .find(|s| s.slot.is_none() && s.equip_slot.can_hold(item.kind()))
        {
            loadout_slot.slot = Some(item);
            Ok(())
        } else {
            Err(item)
        }
    }

    pub(super) fn items(&self) -> impl Iterator<Item = &Item> {
        self.slots.iter().filter_map(|x| x.slot.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use crate::comp::{
        inventory::{
            item::{
                armor::{Armor, ArmorKind, Protection},
                ItemKind,
            },
            loadout::Loadout,
            slot::{ArmorSlot, EquipSlot},
            test_helpers::get_test_bag,
        },
        Item,
    };

    #[test]
    fn test_slot_range_for_equip_slot() {
        let mut loadout = Loadout::new_empty();

        let bag1_slot = EquipSlot::Armor(ArmorSlot::Bag1);
        let bag = get_test_bag(18);
        loadout.swap(bag1_slot, Some(bag));

        let result = loadout.slot_range_for_equip_slot(bag1_slot).unwrap();

        assert_eq!(0..18, result);
    }

    #[test]
    fn test_slot_range_for_equip_slot_no_item() {
        let loadout = Loadout::new_empty();
        let result = loadout.slot_range_for_equip_slot(EquipSlot::Armor(ArmorSlot::Bag1));

        assert_eq!(None, result);
    }

    #[test]
    fn test_slot_range_for_equip_slot_item_without_slots() {
        let mut loadout = Loadout::new_empty();

        let feet_slot = EquipSlot::Armor(ArmorSlot::Feet);
        let boots = Item::new_from_asset_expect("common.items.testing.test_boots");
        loadout.swap(feet_slot, Some(boots));
        let result = loadout.slot_range_for_equip_slot(feet_slot);

        assert_eq!(None, result);
    }

    #[test]
    fn test_get_slot_to_equip_into_second_bag_slot_free() {
        let mut loadout = Loadout::new_empty();

        loadout.swap(EquipSlot::Armor(ArmorSlot::Bag1), Some(get_test_bag(1)));

        let result = loadout
            .get_slot_to_equip_into(&ItemKind::Armor(Armor::test_armor(
                ArmorKind::Bag("test".to_string()),
                Protection::Normal(0.0),
                Protection::Normal(0.0),
            )))
            .unwrap();

        assert_eq!(EquipSlot::Armor(ArmorSlot::Bag2), result);
    }

    #[test]
    fn test_get_slot_to_equip_into_no_bag_slots_free() {
        let mut loadout = Loadout::new_empty();

        loadout.swap(EquipSlot::Armor(ArmorSlot::Bag1), Some(get_test_bag(1)));
        loadout.swap(EquipSlot::Armor(ArmorSlot::Bag2), Some(get_test_bag(1)));
        loadout.swap(EquipSlot::Armor(ArmorSlot::Bag3), Some(get_test_bag(1)));
        loadout.swap(EquipSlot::Armor(ArmorSlot::Bag4), Some(get_test_bag(1)));

        let result = loadout
            .get_slot_to_equip_into(&ItemKind::Armor(Armor::test_armor(
                ArmorKind::Bag("test".to_string()),
                Protection::Normal(0.0),
                Protection::Normal(0.0),
            )))
            .unwrap();

        assert_eq!(EquipSlot::Armor(ArmorSlot::Bag1), result);
    }
}
