use core::ops::Not;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::{cmp::Ordering, convert::TryFrom, mem, ops::Range};
use tracing::{debug, trace, warn};
use vek::Vec3;

use crate::{
    comp::{
        body::Body,
        inventory::{
            item::{
                item_key::ItemKey, tool::AbilityMap, ItemDef, ItemDefinitionIdOwned, ItemKind,
                MaterialStatManifest, TagExampleInfo,
            },
            loadout::Loadout,
            slot::{EquipSlot, Slot, SlotError},
        },
        loot_owner::LootOwnerKind,
        slot::{InvSlotId, SlotId},
        Item,
    },
    resources::Time,
    uid::Uid,
    LoadoutBuilder,
};

pub mod item;
pub mod loadout;
pub mod loadout_builder;
pub mod slot;
#[cfg(test)] mod test;
#[cfg(test)] mod test_helpers;
pub mod trade_pricing;

pub type InvSlot = Option<Item>;
const DEFAULT_INVENTORY_SLOTS: usize = 18;

/// NOTE: Do not add a PartialEq instance for Inventory; that's broken!
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inventory {
    next_sort_order: InventorySortOrder,
    loadout: Loadout,
    /// The "built-in" slots belonging to the inventory itself, all other slots
    /// are provided by equipped items
    slots: Vec<InvSlot>,
    /// For when slot amounts are rebalanced or the inventory otherwise does not
    /// have enough space to hold all the items after loading from database.
    /// These slots are "remove-only" meaning that during normal gameplay items
    /// can only be removed from these slots and never entered.
    overflow_items: Vec<Item>,
}

/// Errors which the methods on `Inventory` produce
#[derive(Debug)]
pub enum Error {
    /// The inventory is full and items could not be added. The extra items have
    /// been returned.
    Full(Vec<Item>),
}

impl Error {
    pub fn returned_items(self) -> impl Iterator<Item = Item> {
        match self {
            Error::Full(items) => items.into_iter(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum InventorySortOrder {
    Name,
    Quality,
    Category,
    Tag,
    Amount,
}

impl InventorySortOrder {
    fn next(&self) -> InventorySortOrder {
        match self {
            InventorySortOrder::Name => InventorySortOrder::Quality,
            InventorySortOrder::Quality => InventorySortOrder::Tag,
            InventorySortOrder::Tag => InventorySortOrder::Category,
            InventorySortOrder::Category => InventorySortOrder::Amount,
            InventorySortOrder::Amount => InventorySortOrder::Name,
        }
    }
}

pub enum CustomOrder {
    Name,
    Quality,
    KindPartial,
    KindFull,
    Tag,
}

/// Represents the Inventory of an entity. The inventory has 18 "built-in"
/// slots, with further slots being provided by items equipped in the Loadout
/// sub-struct. Inventory slots are indexed by `InvSlotId` which is
/// comprised of `loadout_idx` - the index of the loadout item that provides the
/// slot, 0 being the built-in inventory slots, and `slot_idx` - the index of
/// the slot within that loadout item.
///
/// Currently, it is not supported for inventories to contain items that have
/// items inside them. This is due to both game balance purposes, and the lack
/// of a UI to show such items. Because of this, any action that would result in
/// such an item being put into the inventory (item pickup, unequipping an item
/// that contains items etc) must first ensure items are unloaded from the item.
/// This is handled in `inventory\slot.rs`
impl Inventory {
    pub fn with_empty() -> Inventory {
        Self::with_loadout_humanoid(LoadoutBuilder::empty().build())
    }

    pub fn with_loadout(loadout: Loadout, body: Body) -> Inventory {
        if let Body::Humanoid(_) = body {
            Self::with_loadout_humanoid(loadout)
        } else {
            Self::with_loadout_animal(loadout)
        }
    }

    pub fn with_loadout_humanoid(loadout: Loadout) -> Inventory {
        Inventory {
            next_sort_order: InventorySortOrder::Name,
            loadout,
            slots: vec![None; DEFAULT_INVENTORY_SLOTS],
            overflow_items: Vec::new(),
        }
    }

    pub fn with_loadout_animal(loadout: Loadout) -> Inventory {
        Inventory {
            next_sort_order: InventorySortOrder::Name,
            loadout,
            slots: vec![None; 1],
            overflow_items: Vec::new(),
        }
    }

    /// Total number of slots in the inventory.
    pub fn capacity(&self) -> usize { self.slots().count() }

    /// An iterator of all inventory slots
    pub fn slots(&self) -> impl Iterator<Item = &InvSlot> {
        self.slots
            .iter()
            .chain(self.loadout.inv_slots_with_id().map(|(_, slot)| slot))
    }

    /// An iterator of all overflow slots in the inventory
    pub fn overflow_items(&self) -> impl Iterator<Item = &Item> { self.overflow_items.iter() }

    /// A mutable iterator of all inventory slots
    fn slots_mut(&mut self) -> impl Iterator<Item = &mut InvSlot> {
        self.slots.iter_mut().chain(self.loadout.inv_slots_mut())
    }

    fn slots_mut_with_mutable_recently_unequipped_items(
        &mut self,
    ) -> (
        impl Iterator<Item = &mut InvSlot>,
        &mut HashMap<ItemDefinitionIdOwned, (Time, u8)>,
    ) {
        let (slots_mut, recently_unequipped) = self
            .loadout
            .inv_slots_mut_with_mutable_recently_unequipped_items();
        (self.slots.iter_mut().chain(slots_mut), recently_unequipped)
    }

    /// An iterator of all inventory slots and their position
    pub fn slots_with_id(&self) -> impl Iterator<Item = (InvSlotId, &InvSlot)> {
        self.slots
            .iter()
            .enumerate()
            .map(|(i, slot)| ((InvSlotId::new(0, u16::try_from(i).unwrap())), slot))
            .chain(
                self.loadout
                    .inv_slots_with_id()
                    .map(|(loadout_slot_id, inv_slot)| (loadout_slot_id.into(), inv_slot)),
            )
    }

    /// If custom_order is empty, it will always return Ordering::Equal
    pub fn order_by_custom(custom_order: &[CustomOrder], a: &Item, b: &Item) -> Ordering {
        let mut order = custom_order.iter();
        let a_quality = a.quality();
        let b_quality = b.quality();
        let a_kind = a.kind().get_itemkind_string();
        let b_kind = b.kind().get_itemkind_string();
        let mut cmp = Ordering::Equal;
        while cmp == Ordering::Equal {
            match order.next() {
                Some(CustomOrder::KindFull) => cmp = Ord::cmp(&a_kind, &b_kind),
                Some(CustomOrder::KindPartial) => {
                    cmp = Ord::cmp(
                        &a_kind.split_once(':').unwrap().0,
                        &b_kind.split_once(':').unwrap().0,
                    )
                },
                Some(CustomOrder::Quality) => cmp = Ord::cmp(&b_quality, &a_quality),
                #[allow(deprecated)]
                Some(CustomOrder::Name) => cmp = Ord::cmp(&a.name(), &b.name()),
                Some(CustomOrder::Tag) => {
                    cmp = Ord::cmp(
                        &a.tags().first().map_or("", |tag| tag.name()),
                        &b.tags().first().map_or("", |tag| tag.name()),
                    )
                },
                _ => break,
            }
        }
        cmp
    }

    /// Sorts the inventory using the next sort order
    pub fn sort(&mut self) {
        let sort_order = self.next_sort_order;
        let mut items: Vec<Item> = self.slots_mut().filter_map(mem::take).collect();

        items.sort_by(|a, b| match sort_order {
            #[allow(deprecated)]
            InventorySortOrder::Name => Ord::cmp(&a.name(), &b.name()),
            // Quality is sorted in reverse since we want high quality items first
            InventorySortOrder::Quality => Ord::cmp(&b.quality(), &a.quality()),
            InventorySortOrder::Category => {
                let order = [
                    CustomOrder::KindPartial,
                    CustomOrder::Quality,
                    CustomOrder::KindFull,
                    CustomOrder::Name,
                ];
                Self::order_by_custom(&order, a, b)
            },
            InventorySortOrder::Tag => Ord::cmp(
                &a.tags().first().map_or("", |tag| tag.name()),
                &b.tags().first().map_or("", |tag| tag.name()),
            ),
            // Amount is sorted in reverse since we want high amounts items first
            InventorySortOrder::Amount => Ord::cmp(&b.amount(), &a.amount()),
        });

        self.push_all(items.into_iter()).expect(
            "It is impossible for there to be insufficient inventory space when sorting the \
             inventory",
        );

        self.next_sort_order = self.next_sort_order.next();
    }

    /// Returns the sort order that will be used when Inventory::sort() is next
    /// called
    pub fn next_sort_order(&self) -> InventorySortOrder { self.next_sort_order }

    /// Adds a new item to the first fitting group of the inventory or starts a
    /// new group. Returns the item in an error if no space was found, otherwise
    /// returns the found slot.
    pub fn push(&mut self, item: Item) -> Result<(), Item> {
        // First, check to make sure there's enough room for all instances of the
        // item (note that if we find any empty slots, we can guarantee this by
        // just filling up the whole slot, but to be nice we won't use it if we
        // can find enough space in any combination of existing slots, and
        // that's what we check in the `is_stackable` case).

        if item.is_stackable()
            && self
                .slots()
                .filter_map(Option::as_ref)
                .filter(|s| *s == &item)
                .try_fold(item.amount(), |remaining, current| {
                    remaining
                        .checked_sub(current.max_amount() - current.amount())
                        .filter(|&remaining| remaining > 0)
                })
                .is_none()
        {
            // We either exactly matched or had more than enough space for inserting the
            // item into existing slots, so go do that!
            assert!(
                self.slots_mut()
                    .filter_map(Option::as_mut)
                    .filter(|s| *s == &item)
                    .try_fold(item.amount(), |remaining, current| {
                        // NOTE: Invariant that current.amount <= current.max_amount(), so the
                        // subtraction is safe.
                        let new_remaining = remaining
                            .checked_sub(current.max_amount() - current.amount())
                            .filter(|&remaining| remaining > 0);
                        if new_remaining.is_some() {
                            // Not enough capacity left to hold all the remaining items, so we fill
                            // it as much as we can.
                            current
                                .set_amount(current.max_amount())
                                .expect("max_amount() is always a valid amount.");
                        } else {
                            // Enough capacity to hold all the remaining items.
                            current
                                .increase_amount(remaining)
                                .expect("Already checked that there is enough room.");
                        }
                        new_remaining
                    })
                    .is_none()
            );
            Ok(())
        } else {
            // No existing item to stack with or item not stackable, put the item in a new
            // slot
            self.insert(item)
        }
    }

    /// Add a series of items to inventory, returning any which do not fit as an
    /// error.
    pub fn push_all<I: Iterator<Item = Item>>(&mut self, items: I) -> Result<(), Error> {
        // Vec doesn't allocate for zero elements so this should be cheap
        let mut leftovers = Vec::new();
        for item in items {
            if let Err(item) = self.push(item) {
                leftovers.push(item);
            }
        }
        if !leftovers.is_empty() {
            Err(Error::Full(leftovers))
        } else {
            Ok(())
        }
    }

    /// Add a series of items to an inventory without giving duplicates.
    /// (n * m complexity)
    ///
    /// Error if inventory cannot contain the items (is full), returning the
    /// un-added items. This is a lazy inefficient implementation, as it
    /// iterates over the inventory more times than necessary (n^2) and with
    /// the proper structure wouldn't need to iterate at all, but because
    /// this should be fairly cold code, clarity has been favored over
    /// efficiency.
    pub fn push_all_unique<I: Iterator<Item = Item>>(&mut self, mut items: I) -> Result<(), Error> {
        let mut leftovers = Vec::new();
        for item in &mut items {
            if self.contains(&item).not() {
                if let Err(overflow) = self.push(item) {
                    leftovers.push(overflow);
                }
            } // else drop item if it was already in
        }
        if !leftovers.is_empty() {
            Err(Error::Full(leftovers))
        } else {
            Ok(())
        }
    }

    /// Replaces an item in a specific slot of the inventory. Returns the old
    /// item or the same item again if that slot was not found.
    pub fn insert_at(&mut self, inv_slot_id: InvSlotId, item: Item) -> Result<Option<Item>, Item> {
        match self.slot_mut(inv_slot_id) {
            Some(slot) => Ok(mem::replace(slot, Some(item))),
            None => Err(item),
        }
    }

    /// Merge the stack of items at src into the stack at dst if the items are
    /// compatible and stackable, and return whether anything was changed
    pub fn merge_stack_into(&mut self, src: InvSlotId, dst: InvSlotId) -> bool {
        let mut amount = None;
        if let (Some(srcitem), Some(dstitem)) = (self.get(src), self.get(dst)) {
            // The equality check ensures the items have the same definition, to avoid e.g.
            // transmuting coins to diamonds, and the stackable check avoids creating a
            // stack of swords
            if srcitem == dstitem && srcitem.is_stackable() {
                amount = Some(srcitem.amount());
            }
        }
        if let Some(amount) = amount {
            let dstitem = self
                .get_mut(dst)
                .expect("self.get(dst) was Some right above this");
            dstitem
                .increase_amount(amount)
                .map(|_| {
                    // Suceeded in adding the item, so remove it from `src`.
                    self.remove(src).expect("Already verified that src was populated.");
                })
                // Can fail if we exceed `max_amount`
                .is_ok()
        } else {
            false
        }
    }

    /// Checks if inserting item exists in given cell. Inserts an item if it
    /// exists.
    pub fn insert_or_stack_at(
        &mut self,
        inv_slot_id: InvSlotId,
        item: Item,
    ) -> Result<Option<Item>, Item> {
        if item.is_stackable() {
            match self.slot_mut(inv_slot_id) {
                Some(Some(slot_item)) => {
                    Ok(if slot_item == &item {
                        slot_item
                            .increase_amount(item.amount())
                            .err()
                            .and(Some(item))
                    } else {
                        let old_item = mem::replace(slot_item, item);
                        // No need to recount--we know the count is the same.
                        Some(old_item)
                    })
                },
                Some(None) => self.insert_at(inv_slot_id, item),
                None => Err(item),
            }
        } else {
            self.insert_at(inv_slot_id, item)
        }
    }

    /// Attempts to equip the item into a compatible, unpopulated loadout slot.
    /// If no slot is available the item is returned.
    #[must_use = "Returned item will be lost if not used"]
    pub fn try_equip(&mut self, item: Item) -> Result<(), Item> { self.loadout.try_equip(item) }

    pub fn populated_slots(&self) -> usize { self.slots().filter_map(|slot| slot.as_ref()).count() }

    pub fn free_slots(&self) -> usize { self.slots().filter(|slot| slot.is_none()).count() }

    /// Check if an item is in this inventory.
    pub fn contains(&self, item: &Item) -> bool {
        self.slots().any(|slot| slot.as_ref() == Some(item))
    }

    /// Return the first slot id containing the item
    pub fn get_slot_of_item(&self, item: &Item) -> Option<InvSlotId> {
        self.slots_with_id()
            .find(|&(_, it)| {
                if let Some(it) = it {
                    it.item_definition_id() == item.item_definition_id()
                } else {
                    false
                }
            })
            .map(|(slot, _)| slot)
    }

    pub fn get_slot_of_item_by_def_id(
        &self,
        item_def_id: &item::ItemDefinitionIdOwned,
    ) -> Option<InvSlotId> {
        self.slots_with_id()
            .find(|&(_, it)| {
                if let Some(it) = it {
                    it.item_definition_id() == *item_def_id
                } else {
                    false
                }
            })
            .map(|(slot, _)| slot)
    }

    /// Get content of a slot
    pub fn get(&self, inv_slot_id: InvSlotId) -> Option<&Item> {
        self.slot(inv_slot_id).and_then(Option::as_ref)
    }

    /// Get item from inventory
    pub fn get_by_hash(&self, item_hash: u64) -> Option<&Item> {
        self.slots().flatten().find(|i| i.item_hash() == item_hash)
    }

    /// Get slot from hash
    pub fn get_slot_from_hash(&self, item_hash: u64) -> Option<InvSlotId> {
        let slot_with_id = self.slots_with_id().find(|slot| match slot.1 {
            None => false,
            Some(item) => item.item_hash() == item_hash,
        });
        slot_with_id.map(|s| s.0)
    }

    /// Mutably get content of a slot
    fn get_mut(&mut self, inv_slot_id: InvSlotId) -> Option<&mut Item> {
        self.slot_mut(inv_slot_id).and_then(Option::as_mut)
    }

    /// Returns a reference to the item (if any) equipped in the given EquipSlot
    pub fn equipped(&self, equip_slot: EquipSlot) -> Option<&Item> {
        self.loadout.equipped(equip_slot)
    }

    pub fn loadout_items_with_persistence_key(
        &self,
    ) -> impl Iterator<Item = (&str, Option<&Item>)> {
        self.loadout.items_with_persistence_key()
    }

    /// Returns the range of inventory slot indexes that a particular equipped
    /// item provides (used for UI highlighting of inventory slots when hovering
    /// over a loadout item)
    pub fn get_slot_range_for_equip_slot(&self, equip_slot: EquipSlot) -> Option<Range<usize>> {
        // The slot range returned from `Loadout` must be offset by the number of slots
        // that the inventory itself provides.
        let offset = self.slots.len();
        self.loadout
            .slot_range_for_equip_slot(equip_slot)
            .map(|loadout_range| (loadout_range.start + offset)..(loadout_range.end + offset))
    }

    /// Swap the items inside of two slots
    pub fn swap_slots(&mut self, a: InvSlotId, b: InvSlotId) {
        if self.slot(a).is_none() || self.slot(b).is_none() {
            warn!("swap_slots called with non-existent inventory slot(s)");
            return;
        }

        let slot_a = mem::take(self.slot_mut(a).unwrap());
        let slot_b = mem::take(self.slot_mut(b).unwrap());
        *self.slot_mut(a).unwrap() = slot_b;
        *self.slot_mut(b).unwrap() = slot_a;
    }

    /// Remove an item from the slot
    pub fn remove(&mut self, inv_slot_id: InvSlotId) -> Option<Item> {
        self.slot_mut(inv_slot_id).and_then(|item| item.take())
    }

    /// Remove just one item from the slot
    pub fn take(
        &mut self,
        inv_slot_id: InvSlotId,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Option<Item> {
        if let Some(Some(item)) = self.slot_mut(inv_slot_id) {
            let mut return_item = item.duplicate(ability_map, msm);

            if item.is_stackable() && item.amount() > 1 {
                item.decrease_amount(1).ok()?;
                return_item
                    .set_amount(1)
                    .expect("Items duplicated from a stackable item must be stackable.");
                Some(return_item)
            } else {
                self.remove(inv_slot_id)
            }
        } else {
            None
        }
    }

    /// Takes half of the items from a slot in the inventory
    pub fn take_half(
        &mut self,
        inv_slot_id: InvSlotId,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) -> Option<Item> {
        if let Some(Some(item)) = self.slot_mut(inv_slot_id) {
            if item.is_stackable() && item.amount() > 1 {
                let mut return_item = item.duplicate(ability_map, msm);
                let returning_amount = item.amount() / 2;
                item.decrease_amount(returning_amount).ok()?;
                return_item.set_amount(returning_amount).expect(
                    "return_item.amount() = item.amount() / 2 < item.amount() (since \
                     item.amount() ≥ 1) ≤ item.max_amount() = return_item.max_amount(), since \
                     return_item is a duplicate of item",
                );
                Some(return_item)
            } else {
                self.remove(inv_slot_id)
            }
        } else {
            None
        }
    }

    /// Takes all items from the inventory
    pub fn drain(&mut self) -> impl Iterator<Item = Item> + '_ {
        self.slots_mut()
            .filter(|x| x.is_some())
            .filter_map(mem::take)
    }

    /// Determine how many of a particular item there is in the inventory.
    pub fn item_count(&self, item_def: &ItemDef) -> u64 {
        self.slots()
            .flatten()
            .filter(|it| it.is_same_item_def(item_def))
            .map(|it| u64::from(it.amount()))
            .sum()
    }

    /// Adds a new item to the first empty slot of the inventory. Returns the
    /// item again in an Err if no free slot was found, otherwise returns a
    /// reference to the item.
    fn insert(&mut self, item: Item) -> Result<(), Item> {
        match self.slots_mut().find(|slot| slot.is_none()) {
            Some(slot) => {
                *slot = Some(item);
                Ok(())
            },
            None => Err(item),
        }
    }

    pub fn slot(&self, inv_slot_id: InvSlotId) -> Option<&InvSlot> {
        match SlotId::from(inv_slot_id) {
            SlotId::Inventory(slot_idx) => self.slots.get(slot_idx),
            SlotId::Loadout(loadout_slot_id) => self.loadout.inv_slot(loadout_slot_id),
        }
    }

    pub fn slot_mut(&mut self, inv_slot_id: InvSlotId) -> Option<&mut InvSlot> {
        match SlotId::from(inv_slot_id) {
            SlotId::Inventory(slot_idx) => self.slots.get_mut(slot_idx),
            SlotId::Loadout(loadout_slot_id) => self.loadout.inv_slot_mut(loadout_slot_id),
        }
    }

    /// Returns the number of free slots in the inventory ignoring any slots
    /// granted by the item (if any) equipped in the provided EquipSlot.
    pub fn free_slots_minus_equipped_item(&self, equip_slot: EquipSlot) -> usize {
        if let Some(mut equip_slot_idx) = self.loadout.loadout_idx_for_equip_slot(equip_slot) {
            // Offset due to index 0 representing built-in inventory slots
            equip_slot_idx += 1;

            self.slots_with_id()
                .filter(|(inv_slot_id, slot)| {
                    inv_slot_id.loadout_idx() != equip_slot_idx && slot.is_none()
                })
                .count()
        } else {
            // TODO: return Option<usize> and evaluate to None here
            warn!(
                "Attempted to fetch loadout index for non-existent EquipSlot: {:?}",
                equip_slot
            );
            0
        }
    }

    pub fn equipped_items(&self) -> impl Iterator<Item = &Item> { self.loadout.items() }

    pub fn equipped_items_with_slot(&self) -> impl Iterator<Item = (EquipSlot, &Item)> {
        self.loadout.items_with_slot()
    }

    /// Replaces the loadout item (if any) in the given EquipSlot with the
    /// provided item, returning the item that was previously in the slot.
    pub fn replace_loadout_item(
        &mut self,
        equip_slot: EquipSlot,
        replacement_item: Option<Item>,
        time: Time,
    ) -> Option<Item> {
        self.loadout.swap(equip_slot, replacement_item, time)
    }

    /// Equip an item from a slot in inventory. The currently equipped item will
    /// go into inventory. If the item is going to mainhand, put mainhand in
    /// offhand and place offhand into inventory.
    #[must_use = "Returned items will be lost if not used"]
    pub fn equip(&mut self, inv_slot: InvSlotId, time: Time) -> Vec<Item> {
        self.get(inv_slot)
            .and_then(|item| self.loadout.get_slot_to_equip_into(&item.kind()))
            .map(|equip_slot| self.swap_inventory_loadout(inv_slot, equip_slot, time))
            .unwrap_or_default()
    }

    /// Determines how many free inventory slots will be left after equipping an
    /// item (because it could be swapped with an already equipped item that
    /// provides more inventory slots than the item being equipped)
    pub fn free_after_equip(&self, inv_slot: InvSlotId) -> i32 {
        let (inv_slot_for_equipped, slots_from_equipped) = self
            .get(inv_slot)
            .and_then(|item| self.loadout.get_slot_to_equip_into(&item.kind()))
            .and_then(|equip_slot| self.equipped(equip_slot))
            .map_or((1, 0), |item| (0, item.slots().len()));

        let slots_from_inv = self
            .get(inv_slot)
            .map(|item| item.slots().len())
            .unwrap_or(0);

        i32::try_from(self.capacity()).expect("Inventory with more than i32::MAX slots")
            - i32::try_from(slots_from_equipped)
                .expect("Equipped item with more than i32::MAX slots")
            + i32::try_from(slots_from_inv).expect("Inventory item with more than i32::MAX slots")
            - i32::try_from(self.populated_slots())
                .expect("Inventory item with more than i32::MAX used slots")
            + inv_slot_for_equipped // If there is no item already in the equip slot we gain 1 slot
    }

    /// Handles picking up an item, unloading any items inside the item being
    /// picked up and pushing them to the inventory to ensure that items
    /// containing items aren't inserted into the inventory as this is not
    /// currently supported.
    pub fn pickup_item(&mut self, mut item: Item) -> Result<(), Item> {
        if item.is_stackable() {
            return self.push(item);
        }

        if self.free_slots() < item.populated_slots() + 1 {
            return Err(item);
        }

        // Unload any items contained within the item, and push those items and the item
        // itself into the inventory. We already know that there are enough free slots
        // so push will never give us an item back.
        item.drain().for_each(|item| {
            self.push(item).unwrap();
        });
        self.push(item)
    }

    /// Unequip an item from slot and place into inventory. Will leave the item
    /// equipped if inventory has no slots available.
    #[must_use = "Returned items will be lost if not used"]
    #[allow(clippy::needless_collect)] // This is a false positive, the collect is needed
    pub fn unequip(
        &mut self,
        equip_slot: EquipSlot,
        time: Time,
    ) -> Result<Option<Vec<Item>>, SlotError> {
        // Ensure there is enough space in the inventory to place the unequipped item
        if self.free_slots_minus_equipped_item(equip_slot) == 0 {
            return Err(SlotError::InventoryFull);
        }

        Ok(self
            .loadout
            .swap(equip_slot, None, time)
            .and_then(|mut unequipped_item| {
                let unloaded_items: Vec<Item> = unequipped_item.drain().collect();
                self.push(unequipped_item)
                    .expect("Failed to push item to inventory, precondition failed?");

                // Unload any items that were inside the equipped item into the inventory, with
                // any that don't fit to be to be dropped on the floor by the caller
                match self.push_all(unloaded_items.into_iter()) {
                    Err(Error::Full(leftovers)) => Some(leftovers),
                    Ok(()) => None,
                }
            }))
    }

    /// Determines how many free inventory slots will be left after unequipping
    /// an item
    pub fn free_after_unequip(&self, equip_slot: EquipSlot) -> i32 {
        let (inv_slot_for_unequipped, slots_from_equipped) = self
            .equipped(equip_slot)
            .map_or((0, 0), |item| (1, item.slots().len()));

        i32::try_from(self.capacity()).expect("Inventory with more than i32::MAX slots")
            - i32::try_from(slots_from_equipped)
                .expect("Equipped item with more than i32::MAX slots")
            - i32::try_from(self.populated_slots())
                .expect("Inventory item with more than i32::MAX used slots")
            - inv_slot_for_unequipped // If there is an item being unequipped we lose 1 slot
    }

    /// Swaps items from two slots, regardless of if either is inventory or
    /// loadout.
    #[must_use = "Returned items will be lost if not used"]
    pub fn swap(&mut self, slot_a: Slot, slot_b: Slot, time: Time) -> Vec<Item> {
        match (slot_a, slot_b) {
            (Slot::Inventory(slot_a), Slot::Inventory(slot_b)) => {
                self.swap_slots(slot_a, slot_b);
                Vec::new()
            },
            (Slot::Inventory(inv_slot), Slot::Equip(equip_slot))
            | (Slot::Equip(equip_slot), Slot::Inventory(inv_slot)) => {
                self.swap_inventory_loadout(inv_slot, equip_slot, time)
            },
            (Slot::Equip(slot_a), Slot::Equip(slot_b)) => {
                self.loadout.swap_slots(slot_a, slot_b, time);
                Vec::new()
            },
        }
    }

    /// Determines how many free inventory slots will be left after swapping two
    /// item slots
    pub fn free_after_swap(&self, equip_slot: EquipSlot, inv_slot: InvSlotId) -> i32 {
        let (inv_slot_for_equipped, slots_from_equipped) = self
            .equipped(equip_slot)
            .map_or((0, 0), |item| (1, item.slots().len()));
        let (inv_slot_for_inv_item, slots_from_inv_item) = self
            .get(inv_slot)
            .map_or((0, 0), |item| (1, item.slots().len()));

        // Return the number of inventory slots that will be free once this slot swap is
        // performed
        i32::try_from(self.capacity())
            .expect("inventory with more than i32::MAX slots")
            - i32::try_from(slots_from_equipped)
            .expect("equipped item with more than i32::MAX slots")
            + i32::try_from(slots_from_inv_item)
            .expect("inventory item with more than i32::MAX slots")
            - i32::try_from(self.populated_slots())
            .expect("inventory with more than i32::MAX used slots")
            - inv_slot_for_equipped // +1 inventory slot required if an item was unequipped
            + inv_slot_for_inv_item // -1 inventory slot required if an item was equipped
    }

    /// Swap item in an inventory slot with one in a loadout slot.
    #[must_use = "Returned items will be lost if not used"]
    pub fn swap_inventory_loadout(
        &mut self,
        inv_slot_id: InvSlotId,
        equip_slot: EquipSlot,
        time: Time,
    ) -> Vec<Item> {
        if !self.can_swap(inv_slot_id, equip_slot) {
            return Vec::new();
        }

        // Take the item from the inventory
        let from_inv = self.remove(inv_slot_id);

        // Swap the equipped item for the item from the inventory
        let from_equip = self.loadout.swap(equip_slot, from_inv, time);

        let unloaded_items = from_equip
            .map(|mut from_equip| {
                // Unload any items held inside the previously equipped item
                let mut items: Vec<Item> = from_equip.drain().collect();

                // Attempt to put the unequipped item in the same slot that the inventory item
                // was in - if that slot no longer exists (because a large container was
                // swapped for a smaller one) then we will attempt to push it to the inventory
                // with the rest of the unloaded items.
                if let Err(returned) = self.insert_at(inv_slot_id, from_equip) {
                    items.insert(0, returned);
                }

                items
            })
            .unwrap_or_default();

        // If 2 1h weapons are equipped, and mainhand weapon removed, move offhand into
        // mainhand
        match equip_slot {
            EquipSlot::ActiveMainhand => {
                if self.loadout.equipped(EquipSlot::ActiveMainhand).is_none()
                    && self.loadout.equipped(EquipSlot::ActiveOffhand).is_some()
                {
                    let offhand = self.loadout.swap(EquipSlot::ActiveOffhand, None, time);
                    assert!(
                        self.loadout
                            .swap(EquipSlot::ActiveMainhand, offhand, time)
                            .is_none()
                    );
                }
            },
            EquipSlot::InactiveMainhand => {
                if self.loadout.equipped(EquipSlot::InactiveMainhand).is_none()
                    && self.loadout.equipped(EquipSlot::InactiveOffhand).is_some()
                {
                    let offhand = self.loadout.swap(EquipSlot::InactiveOffhand, None, time);
                    assert!(
                        self.loadout
                            .swap(EquipSlot::InactiveMainhand, offhand, time)
                            .is_none()
                    );
                }
            },
            _ => {},
        }

        // Attempt to put any items unloaded from the unequipped item into empty
        // inventory slots and return any that don't fit to the caller where they
        // will be dropped on the ground
        match self.push_all(unloaded_items.into_iter()) {
            Err(Error::Full(leftovers)) => leftovers,
            Ok(()) => Vec::new(),
        }
    }

    /// Determines if an inventory and loadout slot can be swapped, taking into
    /// account whether there will be free space in the inventory for the
    /// loadout item once any slots that were provided by it have been
    /// removed.
    #[allow(clippy::blocks_in_if_conditions)]
    pub fn can_swap(&self, inv_slot_id: InvSlotId, equip_slot: EquipSlot) -> bool {
        // Check if loadout slot can hold item
        if !self.get(inv_slot_id).map_or(true, |item| {
            self.loadout.slot_can_hold(equip_slot, Some(&*item.kind()))
        }) {
            trace!("can_swap = false, equip slot can't hold item");
            return false;
        }

        if self.slot(inv_slot_id).is_none() {
            debug!(
                "can_swap = false, tried to swap into non-existent inventory slot: {:?}",
                inv_slot_id
            );
            return false;
        }

        true
    }

    pub fn equipped_items_replaceable_by<'a>(
        &'a self,
        item_kind: &'a ItemKind,
    ) -> impl Iterator<Item = &'a Item> {
        self.loadout.equipped_items_replaceable_by(item_kind)
    }

    pub fn swap_equipped_weapons(&mut self, time: Time) { self.loadout.swap_equipped_weapons(time) }

    /// Update internal computed state of all top level items in this loadout.
    /// Used only when loading in persistence code.
    pub fn persistence_update_all_item_states(
        &mut self,
        ability_map: &AbilityMap,
        msm: &MaterialStatManifest,
    ) {
        self.slots_mut().for_each(|slot| {
            if let Some(item) = slot {
                item.update_item_state(ability_map, msm);
            }
        });
        self.overflow_items
            .iter_mut()
            .for_each(|item| item.update_item_state(ability_map, msm));
    }

    /// Increments durability lost for all valid items equipped in loadout and
    /// recently unequipped from loadout by 1
    pub fn damage_items(
        &mut self,
        ability_map: &item::tool::AbilityMap,
        msm: &item::MaterialStatManifest,
        time: Time,
    ) {
        self.loadout.damage_items(ability_map, msm);
        self.loadout.cull_recently_unequipped_items(time);

        let (slots_mut, recently_unequipped_items) =
            self.slots_mut_with_mutable_recently_unequipped_items();
        slots_mut.filter_map(|slot| slot.as_mut()).for_each(|item| {
            if item.durability_lost()
                    .map_or(false, |dur| dur < Item::MAX_DURABILITY)
                && let Some((_unequip_time, count)) =
                   recently_unequipped_items.get_mut(&item.item_definition_id())
                && *count > 0
            {
                *count -= 1;
                item.increment_damage(ability_map, msm);
            }
        });
    }

    /// Resets durability of item in specified slot
    pub fn repair_item_at_slot(
        &mut self,
        slot: Slot,
        ability_map: &item::tool::AbilityMap,
        msm: &item::MaterialStatManifest,
    ) {
        match slot {
            Slot::Inventory(invslot) => {
                if let Some(Some(item)) = self.slot_mut(invslot) {
                    item.reset_durability(ability_map, msm);
                }
            },
            Slot::Equip(equip_slot) => {
                self.loadout
                    .repair_item_at_slot(equip_slot, ability_map, msm);
            },
        }
    }

    /// When loading a character from the persistence system, pushes any items
    /// to overflow_items that were not able to be loaded into or pushed to the
    /// inventory
    pub fn persistence_push_overflow_items<I: Iterator<Item = Item>>(&mut self, overflow_items: I) {
        self.overflow_items.extend(overflow_items);
    }
}

impl Component for Inventory {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum CollectFailedReason {
    InventoryFull,
    LootOwned {
        owner: LootOwnerKind,
        expiry_secs: u64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InventoryUpdateEvent {
    Init,
    Used,
    Consumed(ItemKey),
    Gave,
    Given,
    Swapped,
    Dropped,
    Collected(Item),
    BlockCollectFailed {
        pos: Vec3<i32>,
        reason: CollectFailedReason,
    },
    EntityCollectFailed {
        entity: Uid,
        reason: CollectFailedReason,
    },
    Possession,
    Debug,
    Craft,
}

impl Default for InventoryUpdateEvent {
    fn default() -> Self { Self::Init }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InventoryUpdate {
    events: Vec<InventoryUpdateEvent>,
}

impl InventoryUpdate {
    pub fn new(event: InventoryUpdateEvent) -> Self {
        Self {
            events: vec![event],
        }
    }

    pub fn push(&mut self, event: InventoryUpdateEvent) { self.events.push(event); }

    pub fn take_events(&mut self) -> Vec<InventoryUpdateEvent> { std::mem::take(&mut self.events) }
}

impl Component for InventoryUpdate {
    // TODO: This could probabably be `DenseVecStorage` (except we call clear on
    // this and that essentially leaks for `DenseVecStorage` atm afaict).
    type Storage = specs::VecStorage<Self>;
}
