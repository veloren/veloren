use crate::{comp, comp::item};
use comp::{Inventory, Loadout};
use tracing::warn;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Slot {
    Inventory(usize),
    Equip(EquipSlot),
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum EquipSlot {
    Armor(ArmorSlot),
    Mainhand,
    Offhand,
    Lantern,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum ArmorSlot {
    Head,
    Neck,
    Shoulders,
    Chest,
    Hands,
    Ring,
    Back,
    Belt,
    Legs,
    Feet,
    Tabard,
}

//const ALL_ARMOR_SLOTS: [ArmorSlot; 11] = [
//    Head, Neck, Shoulders, Chest, Hands, Ring, Back, Belt, Legs, Feet, Tabard,
//];

impl Slot {
    pub fn can_hold(self, item_kind: &item::ItemKind) -> bool {
        match (self, item_kind) {
            (Self::Inventory(_), _) => true,
            (Self::Equip(slot), item_kind) => slot.can_hold(item_kind),
        }
    }
}

impl EquipSlot {
    fn can_hold(self, item_kind: &item::ItemKind) -> bool {
        use item::ItemKind;
        match (self, item_kind) {
            (Self::Armor(slot), ItemKind::Armor { kind, .. }) => slot.can_hold(kind),
            (Self::Mainhand, ItemKind::Tool(_)) => true,
            (Self::Offhand, ItemKind::Tool(_)) => true,
            (Self::Lantern, ItemKind::Lantern(_)) => true,
            _ => false,
        }
    }
}

impl ArmorSlot {
    fn can_hold(self, armor: &item::armor::Armor) -> bool {
        use item::armor::Armor;
        match (self, armor) {
            (Self::Head, Armor::Head(_)) => true,
            (Self::Neck, Armor::Neck(_)) => true,
            (Self::Shoulders, Armor::Shoulder(_)) => true,
            (Self::Chest, Armor::Chest(_)) => true,
            (Self::Hands, Armor::Hand(_)) => true,
            (Self::Ring, Armor::Ring(_)) => true,
            (Self::Back, Armor::Back(_)) => true,
            (Self::Belt, Armor::Belt(_)) => true,
            (Self::Legs, Armor::Pants(_)) => true,
            (Self::Feet, Armor::Foot(_)) => true,
            (Self::Tabard, Armor::Tabard(_)) => true,
            _ => false,
        }
    }
}

// TODO: There are plans to save the selected abilities for each tool even
// when they are not equipped, when that is implemented this helper function
// should no longer be needed

/// Create an ItemConfig for an item. Apply abilties to item.
fn item_config(item: item::Item) -> comp::ItemConfig {
    let mut abilities = if let item::ItemKind::Tool(tool) = &item.kind {
        tool.get_abilities()
    } else {
        Vec::new()
    }
    .into_iter();

    comp::ItemConfig {
        item,
        ability1: abilities.next(),
        ability2: abilities.next(),
        ability3: abilities.next(),
        block_ability: Some(comp::CharacterAbility::BasicBlock),
        dodge_ability: Some(comp::CharacterAbility::Roll),
    }
}

/// Replace an equiptment slot with an item. Return the item that was in the
/// slot, if any. Doesn't update the inventory.
fn loadout_replace(
    equip_slot: EquipSlot,
    item: Option<item::Item>,
    loadout: &mut Loadout,
) -> Option<item::Item> {
    use std::mem::replace;
    match equip_slot {
        EquipSlot::Armor(ArmorSlot::Head) => replace(&mut loadout.head, item),
        EquipSlot::Armor(ArmorSlot::Neck) => replace(&mut loadout.neck, item),
        EquipSlot::Armor(ArmorSlot::Shoulders) => replace(&mut loadout.shoulder, item),
        EquipSlot::Armor(ArmorSlot::Chest) => replace(&mut loadout.chest, item),
        EquipSlot::Armor(ArmorSlot::Hands) => replace(&mut loadout.hand, item),
        EquipSlot::Armor(ArmorSlot::Ring) => replace(&mut loadout.ring, item),
        EquipSlot::Armor(ArmorSlot::Back) => replace(&mut loadout.back, item),
        EquipSlot::Armor(ArmorSlot::Belt) => replace(&mut loadout.belt, item),
        EquipSlot::Armor(ArmorSlot::Legs) => replace(&mut loadout.pants, item),
        EquipSlot::Armor(ArmorSlot::Feet) => replace(&mut loadout.foot, item),
        EquipSlot::Armor(ArmorSlot::Tabard) => replace(&mut loadout.tabard, item),
        EquipSlot::Lantern => replace(&mut loadout.lantern, item),
        EquipSlot::Mainhand => {
            replace(&mut loadout.active_item, item.map(item_config)).map(|i| i.item)
        },
        EquipSlot::Offhand => {
            replace(&mut loadout.second_item, item.map(item_config)).map(|i| i.item)
        },
    }
}

/// Insert an item into a loadout. If the specified slot is already occupied
/// the old item is returned.
#[must_use]
fn loadout_insert(
    equip_slot: EquipSlot,
    item: item::Item,
    loadout: &mut Loadout,
) -> Option<item::Item> {
    loadout_replace(equip_slot, Some(item), loadout)
}

/// Remove an item from a loadout.
///
/// ```
/// use veloren_common::{
///     comp::{
///         slot::{loadout_remove, EquipSlot},
///         Inventory,
///     },
///     LoadoutBuilder,
/// };
///
/// let mut inv = Inventory {
///     slots: vec![None],
///     amount: 0,
/// };
///
/// let mut loadout = LoadoutBuilder::new()
///     .defaults()
///     .active_item(LoadoutBuilder::default_item_config_from_str(Some(
///         "common.items.weapons.sword.zweihander_sword_0",
///     )))
///     .build();
///
/// let slot = EquipSlot::Mainhand;
///
/// loadout_remove(slot, &mut loadout);
/// assert_eq!(None, loadout.active_item);
/// ```
pub fn loadout_remove(equip_slot: EquipSlot, loadout: &mut Loadout) -> Option<item::Item> {
    loadout_replace(equip_slot, None, loadout)
}

/// Swap item in an inventory slot with one in a loadout slot.
fn swap_inventory_loadout(
    inventory_slot: usize,
    equip_slot: EquipSlot,
    inventory: &mut Inventory,
    loadout: &mut Loadout,
) {
    // Check if loadout slot can hold item
    if inventory
        .get(inventory_slot)
        .map_or(true, |item| equip_slot.can_hold(&item.kind))
    {
        // Take item from loadout
        let from_equip = loadout_remove(equip_slot, loadout);
        // Swap with item in the inventory
        let from_inv = if let Some(item) = from_equip {
            // If this fails and we get item back as an err it will just be put back in the
            // loadout
            inventory.insert(inventory_slot, item).unwrap_or_else(Some)
        } else {
            inventory.remove(inventory_slot)
        };
        // Put item from the inventory in loadout
        if let Some(item) = from_inv {
            loadout_insert(equip_slot, item, loadout).unwrap_none(); // Can never fail
        }
    }
}

/// Swap items in loadout. Does nothing if items are not compatible with their
/// new slots.
fn swap_loadout(slot_a: EquipSlot, slot_b: EquipSlot, loadout: &mut Loadout) {
    // Ensure that the slots are not the same
    if slot_a == slot_b {
        warn!("Tried to swap equip slot with itself");
        return;
    }

    // Get items from the slots
    let item_a = loadout_remove(slot_a, loadout);
    let item_b = loadout_remove(slot_b, loadout);
    // Check if items can go in the other slots
    if item_a.as_ref().map_or(true, |i| slot_b.can_hold(&i.kind))
        && item_b.as_ref().map_or(true, |i| slot_a.can_hold(&i.kind))
    {
        // Swap
        loadout_replace(slot_b, item_a, loadout).unwrap_none();
        loadout_replace(slot_a, item_b, loadout).unwrap_none();
    } else {
        // Otherwise put the items back
        loadout_replace(slot_a, item_a, loadout).unwrap_none();
        loadout_replace(slot_b, item_b, loadout).unwrap_none();
    }
}

// TODO: Should this report if a change actually occurred? (might be useful when
// minimizing network use)

/// Swap items from two slots, regardless of if either is inventory or loadout.
pub fn swap(
    slot_a: Slot,
    slot_b: Slot,
    inventory: Option<&mut Inventory>,
    loadout: Option<&mut Loadout>,
) {
    match (slot_a, slot_b) {
        (Slot::Inventory(slot_a), Slot::Inventory(slot_b)) => {
            inventory.map(|i| i.swap_slots(slot_a, slot_b));
        },
        (Slot::Inventory(inv_slot), Slot::Equip(equip_slot))
        | (Slot::Equip(equip_slot), Slot::Inventory(inv_slot)) => {
            if let Some((inventory, loadout)) = loadout.and_then(|l| inventory.map(|i| (i, l))) {
                swap_inventory_loadout(inv_slot, equip_slot, inventory, loadout);
            }
        },

        (Slot::Equip(slot_a), Slot::Equip(slot_b)) => {
            loadout.map(|l| swap_loadout(slot_a, slot_b, l));
        },
    }
}

/// Equip an item from a slot in inventory. The currently equipped item will go
/// into inventory. If the item is going to mainhand, put mainhand in
/// offhand and place offhand into inventory.
///
/// ```
/// use veloren_common::{
///     assets,
///     comp::{
///         slot::{equip, EquipSlot},
///         Inventory, Item,
///     },
///     LoadoutBuilder,
/// };
///
/// let boots: Option<Item> = Some(assets::load_expect_cloned(
///     "common.items.testing.test_boots",
/// ));
///
/// let mut inv = Inventory {
///     slots: vec![boots.clone()],
///     amount: 1,
/// };
///
/// let mut loadout = LoadoutBuilder::new().defaults().build();
///
/// equip(0, &mut inv, &mut loadout);
/// assert_eq!(boots, loadout.foot);
/// ```
pub fn equip(slot: usize, inventory: &mut Inventory, loadout: &mut Loadout) {
    use item::{armor::Armor, ItemKind};

    let equip_slot = inventory.get(slot).and_then(|i| match &i.kind {
        ItemKind::Tool(_) => Some(EquipSlot::Mainhand),
        ItemKind::Armor { kind, .. } => Some(EquipSlot::Armor(match kind {
            Armor::Head(_) => ArmorSlot::Head,
            Armor::Neck(_) => ArmorSlot::Neck,
            Armor::Shoulder(_) => ArmorSlot::Shoulders,
            Armor::Chest(_) => ArmorSlot::Chest,
            Armor::Hand(_) => ArmorSlot::Hands,
            Armor::Ring(_) => ArmorSlot::Ring,
            Armor::Back(_) => ArmorSlot::Back,
            Armor::Belt(_) => ArmorSlot::Belt,
            Armor::Pants(_) => ArmorSlot::Legs,
            Armor::Foot(_) => ArmorSlot::Feet,
            Armor::Tabard(_) => ArmorSlot::Tabard,
        })),
        ItemKind::Lantern(_) => Some(EquipSlot::Lantern),
        _ => None,
    });

    if let Some(equip_slot) = equip_slot {
        // If item is going to mainhand, put mainhand in offhand and place offhand in
        // inventory
        if let EquipSlot::Mainhand = equip_slot {
            swap_loadout(EquipSlot::Mainhand, EquipSlot::Offhand, loadout);
        }

        swap_inventory_loadout(slot, equip_slot, inventory, loadout);
    }
}

/// Unequip an item from slot and place into inventory. Will leave the item
/// equipped if inventory has no slots available.
///
/// ```
/// use veloren_common::{
///     comp::{
///         slot::{unequip, EquipSlot},
///         Inventory,
///     },
///     LoadoutBuilder,
/// };
///
/// let mut inv = Inventory {
///     slots: vec![None],
///     amount: 0,
/// };
///
/// let mut loadout = LoadoutBuilder::new()
///     .defaults()
///     .active_item(LoadoutBuilder::default_item_config_from_str(Some(
///         "common.items.weapons.sword.zweihander_sword_0",
///     )))
///     .build();
///
/// let slot = EquipSlot::Mainhand;
///
/// unequip(slot, &mut inv, &mut loadout);
/// assert_eq!(None, loadout.active_item);
/// ```
pub fn unequip(slot: EquipSlot, inventory: &mut Inventory, loadout: &mut Loadout) {
    loadout_remove(slot, loadout) // Remove item from loadout
        .and_then(|i| inventory.push(i)) // Insert into inventory
        .and_then(|i| loadout_insert(slot, i, loadout)) // If that fails put back in loadout
        .unwrap_none(); // Never fails
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assets, LoadoutBuilder};

    #[test]
    fn test_unequip_items_both_hands() {
        let mut inv = Inventory {
            slots: vec![None],
            amount: 0,
        };

        let sword = LoadoutBuilder::default_item_config_from_str(Some(
            "common.items.weapons.sword.zweihander_sword_0",
        ));

        let mut loadout = LoadoutBuilder::new()
            .defaults()
            .active_item(sword.clone())
            .second_item(sword.clone())
            .build();

        assert_eq!(sword, loadout.active_item);
        unequip(EquipSlot::Mainhand, &mut inv, &mut loadout);
        // We have space in the inventory, so this should have unequipped
        assert_eq!(None, loadout.active_item);

        unequip(EquipSlot::Offhand, &mut inv, &mut loadout);
        // There is no more space in the inventory, so this should still be equipped
        assert_eq!(sword, loadout.second_item);

        // Verify inventory
        assert_eq!(inv.slots[0], Some(sword.unwrap().item));
        assert_eq!(inv.slots.len(), 1);
    }

    #[test]
    fn test_equip_item() {
        let boots: Option<comp::Item> = Some(assets::load_expect_cloned(
            "common.items.testing.test_boots",
        ));

        let starting_sandles: Option<comp::Item> = Some(assets::load_expect_cloned(
            "common.items.armor.starter.sandals_0",
        ));

        let mut inv = Inventory {
            slots: vec![boots.clone()],
            amount: 1,
        };

        let mut loadout = LoadoutBuilder::new().defaults().build();

        // We should start with the starting sandles
        assert_eq!(starting_sandles, loadout.foot);
        equip(0, &mut inv, &mut loadout);

        // We should now have the testing boots equiped
        assert_eq!(boots, loadout.foot);

        // Verify inventory
        assert_eq!(inv.slots[0], starting_sandles);
        assert_eq!(inv.slots.len(), 1);
    }

    #[test]
    fn test_loadout_replace() {
        let boots: Option<comp::Item> = Some(assets::load_expect_cloned(
            "common.items.testing.test_boots",
        ));

        let starting_sandles: Option<comp::Item> = Some(assets::load_expect_cloned(
            "common.items.armor.starter.sandals_0",
        ));

        let mut loadout = LoadoutBuilder::new().defaults().build();

        // We should start with the starting sandles
        assert_eq!(starting_sandles, loadout.foot);

        // The swap should return the sandles
        assert_eq!(
            starting_sandles,
            loadout_replace(
                EquipSlot::Armor(ArmorSlot::Feet),
                boots.clone(),
                &mut loadout,
            )
        );

        // We should now have the testing boots equiped
        assert_eq!(boots, loadout.foot);
    }

    #[test]
    fn test_loadout_remove() {
        let sword = LoadoutBuilder::default_item_config_from_str(Some(
            "common.items.weapons.sword.zweihander_sword_0",
        ));

        let mut loadout = LoadoutBuilder::new()
            .defaults()
            .active_item(sword.clone())
            .build();

        // The swap should return the sword
        assert_eq!(
            Some(sword.unwrap().item),
            loadout_remove(EquipSlot::Mainhand, &mut loadout,)
        );

        // We should now have nothing equiped
        assert_eq!(None, loadout.active_item);
    }
}
