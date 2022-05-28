use crate::comp::{
    inventory::{
        item::{self, tool::Tool, Hands, ItemKind},
        slot::{ArmorSlot, EquipSlot},
        InvSlot,
    },
    Item,
};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use tracing::warn;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Loadout {
    slots: Vec<LoadoutSlot>,
}

/// NOTE: Please don't derive a PartialEq Instance for this; that's broken!
#[derive(Clone, Debug, Serialize, Deserialize)]
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
                (EquipSlot::ActiveMainhand, "active_mainhand".to_string()),
                (EquipSlot::ActiveOffhand, "active_offhand".to_string()),
                (EquipSlot::InactiveMainhand, "inactive_mainhand".to_string()),
                (EquipSlot::InactiveOffhand, "inactive_offhand".to_string()),
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

    pub fn get_mut_item_at_slot_using_persistence_key(
        &mut self,
        persistence_key: &str,
    ) -> Result<&mut Item, LoadoutError> {
        self.slots
            .iter_mut()
            .find(|loadout_slot| loadout_slot.persistence_key == persistence_key)
            .map_or(Err(LoadoutError::InvalidPersistenceKey), |loadout_slot| {
                loadout_slot
                    .slot
                    .as_mut()
                    .ok_or(LoadoutError::NoParentAtSlot)
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
        let item_b = self.swap(equip_slot_b, item_a);
        assert_eq!(self.swap(equip_slot_a, item_b), None);

        // Check if items are valid in their new positions
        if !self.slot_can_hold(
            equip_slot_a,
            self.equipped(equip_slot_a).map(|x| x.kind()).as_deref(),
        ) || !self.slot_can_hold(
            equip_slot_b,
            self.equipped(equip_slot_b).map(|x| x.kind()).as_deref(),
        ) {
            // If not, revert the swap
            let item_a = self.swap(equip_slot_a, None);
            let item_b = self.swap(equip_slot_b, item_a);
            assert_eq!(self.swap(equip_slot_a, item_b), None);
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
            .filter(|s| self.slot_can_hold(s.equip_slot, Some(item_kind)));

        let first = suitable_slots.next();

        first
            .into_iter()
            .chain(suitable_slots)
            .find(|loadout_slot| loadout_slot.slot.is_none())
            .map(|x| x.equip_slot)
            .or_else(|| first.map(|x| x.equip_slot))
    }

    /// Returns all items currently equipped that an item of the given ItemKind
    /// could replace
    pub(super) fn equipped_items_replaceable_by<'a>(
        &'a self,
        item_kind: &'a ItemKind,
    ) -> impl Iterator<Item = &'a Item> {
        self.slots
            .iter()
            .filter(move |s| self.slot_can_hold(s.equip_slot, Some(item_kind)))
            .filter_map(|s| s.slot.as_ref())
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
        let loadout_slot = self
            .slots
            .iter()
            .find(|s| s.slot.is_none() && self.slot_can_hold(s.equip_slot, Some(&*item.kind())))
            .map(|s| s.equip_slot);
        if let Some(slot) = self
            .slots
            .iter_mut()
            .find(|s| Some(s.equip_slot) == loadout_slot)
        {
            slot.slot = Some(item);
            Ok(())
        } else {
            Err(item)
        }
    }

    pub(super) fn items(&self) -> impl Iterator<Item = &Item> {
        self.slots.iter().filter_map(|x| x.slot.as_ref())
    }

    /// Checks that a slot can hold a given item
    pub(super) fn slot_can_hold(
        &self,
        equip_slot: EquipSlot,
        item_kind: Option<&ItemKind>,
    ) -> bool {
        // Disallow equipping incompatible weapon pairs (i.e a two-handed weapon and a
        // one-handed weapon)
        if !(match equip_slot {
            EquipSlot::ActiveMainhand => Loadout::is_valid_weapon_pair(
                item_kind,
                self.equipped(EquipSlot::ActiveOffhand)
                    .map(|x| x.kind())
                    .as_deref(),
            ),
            EquipSlot::ActiveOffhand => Loadout::is_valid_weapon_pair(
                self.equipped(EquipSlot::ActiveMainhand)
                    .map(|x| x.kind())
                    .as_deref(),
                item_kind,
            ),
            EquipSlot::InactiveMainhand => Loadout::is_valid_weapon_pair(
                item_kind,
                self.equipped(EquipSlot::InactiveOffhand)
                    .map(|x| x.kind())
                    .as_deref(),
            ),
            EquipSlot::InactiveOffhand => Loadout::is_valid_weapon_pair(
                self.equipped(EquipSlot::InactiveMainhand)
                    .map(|x| x.kind())
                    .as_deref(),
                item_kind,
            ),
            _ => true,
        }) {
            return false;
        }

        item_kind.map_or(true, |item| equip_slot.can_hold(item))
    }

    #[rustfmt::skip]
    fn is_valid_weapon_pair(main_hand: Option<&ItemKind>, off_hand: Option<&ItemKind>) -> bool {
        matches!((main_hand, off_hand),
            (Some(ItemKind::Tool(Tool { hands: Hands::One, .. })), None) |
            (Some(ItemKind::Tool(Tool { hands: Hands::Two, .. })), None) |
            (Some(ItemKind::Tool(Tool { hands: Hands::One, .. })), Some(ItemKind::Tool(Tool { hands: Hands::One, .. }))) |
            (None, None))
    }

    pub(super) fn swap_equipped_weapons(&mut self) {
        // Checks if a given slot can hold an item right now, defaults to true if
        // nothing is equipped in slot
        let valid_slot = |equip_slot| {
            self.equipped(equip_slot)
                .map_or(true, |i| self.slot_can_hold(equip_slot, Some(&*i.kind())))
        };

        // If every weapon is currently in a valid slot, after this change they will
        // still be in a valid slot. This is because active mainhand and
        // inactive mainhand, and active offhand and inactive offhand have the same
        // requirements on what can be equipped.
        if valid_slot(EquipSlot::ActiveMainhand)
            && valid_slot(EquipSlot::ActiveOffhand)
            && valid_slot(EquipSlot::InactiveMainhand)
            && valid_slot(EquipSlot::InactiveOffhand)
        {
            // Get weapons from each slot
            let active_mainhand = self.swap(EquipSlot::ActiveMainhand, None);
            let active_offhand = self.swap(EquipSlot::ActiveOffhand, None);
            let inactive_mainhand = self.swap(EquipSlot::InactiveMainhand, None);
            let inactive_offhand = self.swap(EquipSlot::InactiveOffhand, None);
            // Equip weapons into new slots
            assert!(
                self.swap(EquipSlot::ActiveMainhand, inactive_mainhand)
                    .is_none()
            );
            assert!(
                self.swap(EquipSlot::ActiveOffhand, inactive_offhand)
                    .is_none()
            );
            assert!(
                self.swap(EquipSlot::InactiveMainhand, active_mainhand)
                    .is_none()
            );
            assert!(
                self.swap(EquipSlot::InactiveOffhand, active_offhand)
                    .is_none()
            );
        }
    }

    /// Update internal computed state of all top level items in this loadout.
    /// Used only when loading in persistence code.
    pub fn persistence_update_all_item_states(
        &mut self,
        ability_map: &item::tool::AbilityMap,
        msm: &item::MaterialStatManifest,
    ) {
        self.slots.iter_mut().for_each(|slot| {
            if let Some(item) = &mut slot.slot {
                item.update_item_state(ability_map, msm);
            }
        });
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
                ArmorKind::Bag,
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
                ArmorKind::Bag,
                Protection::Normal(0.0),
                Protection::Normal(0.0),
            )))
            .unwrap();

        assert_eq!(EquipSlot::Armor(ArmorSlot::Bag1), result);
    }
}
