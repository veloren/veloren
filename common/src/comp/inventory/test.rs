use super::*;
use crate::comp::{
    inventory::{slot::ArmorSlot, test_helpers::get_test_bag},
    item::ItemDefinitionId,
    Item,
};
use lazy_static::lazy_static;
lazy_static! {
    static ref TEST_ITEMS: Vec<Item> = vec![Item::new_from_asset_expect(
        "common.items.debug.admin_stick"
    ),];
}

/// Attempting to push into a full inventory should return the same item.
#[test]
fn push_full() {
    let msm = &MaterialStatManifest::load().read();
    let ability_map = &AbilityMap::load().read();
    let mut inv = Inventory {
        next_sort_order: InventorySortOrder::Name,
        slots: TEST_ITEMS
            .iter()
            .map(|a| Some(a.duplicate(ability_map, msm)))
            .collect(),
        loadout: LoadoutBuilder::empty().build(),
    };
    assert_eq!(
        inv.push(TEST_ITEMS[0].duplicate(ability_map, msm))
            .unwrap_err(),
        TEST_ITEMS[0].duplicate(ability_map, msm)
    )
}

/// Attempting to push a series into a full inventory should return them all.
#[test]
fn push_all_full() {
    let msm = &MaterialStatManifest::load().read();
    let ability_map = &AbilityMap::load().read();
    let mut inv = Inventory {
        next_sort_order: InventorySortOrder::Name,
        slots: TEST_ITEMS
            .iter()
            .map(|a| Some(a.duplicate(ability_map, msm)))
            .collect(),
        loadout: LoadoutBuilder::empty().build(),
    };
    let Error::Full(leftovers) = inv
        .push_all(
            TEST_ITEMS
                .iter()
                .map(|item| item.duplicate(ability_map, msm)),
        )
        .expect_err("Pushing into a full inventory somehow worked!");
    assert_eq!(
        leftovers,
        TEST_ITEMS
            .iter()
            .map(|item| item.duplicate(ability_map, msm))
            .collect::<Vec<_>>()
    )
}

/// Attempting to push uniquely into an inventory containing all the items
/// should work fine.
#[test]
fn push_unique_all_full() {
    let msm = &MaterialStatManifest::load().read();
    let ability_map = &AbilityMap::load().read();
    let mut inv = Inventory {
        next_sort_order: InventorySortOrder::Name,
        slots: TEST_ITEMS
            .iter()
            .map(|a| Some(a.duplicate(ability_map, msm)))
            .collect(),
        loadout: LoadoutBuilder::empty().build(),
    };
    inv.push_all_unique(
        TEST_ITEMS
            .iter()
            .map(|item| item.duplicate(ability_map, msm)),
    )
    .expect("Pushing unique items into an inventory that already contains them didn't work!");
}

/// Attempting to push uniquely into an inventory containing all the items
/// should work fine.
#[test]
fn push_all_empty() {
    let msm = &MaterialStatManifest::load().read();
    let ability_map = &AbilityMap::load().read();
    let mut inv = Inventory {
        next_sort_order: InventorySortOrder::Name,
        slots: vec![None, None],
        loadout: LoadoutBuilder::empty().build(),
    };
    inv.push_all(
        TEST_ITEMS
            .iter()
            .map(|item| item.duplicate(ability_map, msm)),
    )
    .expect("Pushing items into an empty inventory didn't work!");
}

/// Attempting to push uniquely into an inventory containing all the items
/// should work fine.
#[test]
fn push_all_unique_empty() {
    let msm = &MaterialStatManifest::load().read();
    let ability_map = &AbilityMap::load().read();
    let mut inv = Inventory {
        next_sort_order: InventorySortOrder::Name,
        slots: vec![None, None],
        loadout: LoadoutBuilder::empty().build(),
    };
    inv.push_all_unique(
        TEST_ITEMS
            .iter()
            .map(|item| item.duplicate(ability_map, msm)),
    )
    .expect("Pushing unique items into an empty inventory that didn't contain them didn't work!");
}

#[test]
fn free_slots_minus_equipped_item_items_only_present_in_equipped_bag_slots() {
    let msm = &MaterialStatManifest::load().read();
    let ability_map = &AbilityMap::load().read();
    let mut inv = Inventory::with_empty();

    let bag = get_test_bag(18);
    let bag1_slot = EquipSlot::Armor(ArmorSlot::Bag1);
    inv.loadout
        .swap(bag1_slot, Some(bag.duplicate(ability_map, msm)));

    assert!(inv.insert_at(InvSlotId::new(15, 0), bag).unwrap().is_none());

    let result = inv.free_slots_minus_equipped_item(bag1_slot);

    // All of the base inventory slots are empty and the equipped bag slots are
    // ignored
    assert_eq!(18, result);
}

#[test]
fn free_slots_minus_equipped_item() {
    let msm = &MaterialStatManifest::load().read();
    let ability_map = &AbilityMap::load().read();
    let mut inv = Inventory::with_empty();

    let bag = get_test_bag(18);
    let bag1_slot = EquipSlot::Armor(ArmorSlot::Bag1);
    inv.loadout
        .swap(bag1_slot, Some(bag.duplicate(ability_map, msm)));
    inv.loadout.swap(
        EquipSlot::Armor(ArmorSlot::Bag2),
        Some(bag.duplicate(ability_map, msm)),
    );

    assert!(inv.insert_at(InvSlotId::new(16, 0), bag).unwrap().is_none());

    let result = inv.free_slots_minus_equipped_item(bag1_slot);

    // All of the base 18 inventory slots are empty, the first equipped bag is
    // ignored, and the second equipped bag has 17 free slots
    assert_eq!(35, result);
}

#[test]
fn get_slot_range_for_equip_slot() {
    let mut inv = Inventory::with_empty();
    let bag = get_test_bag(18);
    let bag1_slot = EquipSlot::Armor(ArmorSlot::Bag1);
    inv.loadout.swap(bag1_slot, Some(bag));

    let result = inv.get_slot_range_for_equip_slot(bag1_slot).unwrap();

    assert_eq!(18..36, result);
}

#[test]
fn can_swap_equipped_bag_into_empty_inv_slot_1_free_slot() {
    can_swap_equipped_bag_into_empty_inv_slot(1, InvSlotId::new(0, 17), true);
}

#[test]
fn can_swap_equipped_bag_into_empty_inv_slot_0_free_slots() {
    can_swap_equipped_bag_into_empty_inv_slot(0, InvSlotId::new(0, 17), false);
}

#[test]
fn can_swap_equipped_bag_into_empty_inv_slot_provided_by_equipped_bag() {
    can_swap_equipped_bag_into_empty_inv_slot(1, InvSlotId::new(15, 0), true);
}

fn can_swap_equipped_bag_into_empty_inv_slot(
    free_slots: u16,
    inv_slot_id: InvSlotId,
    expected_result: bool,
) {
    let mut inv = Inventory::with_empty();

    inv.replace_loadout_item(EquipSlot::Armor(ArmorSlot::Bag1), Some(get_test_bag(18)));

    fill_inv_slots(&mut inv, 18 - free_slots);

    let result = inv.can_swap(inv_slot_id, EquipSlot::Armor(ArmorSlot::Bag1));

    assert_eq!(expected_result, result);
}

#[test]
fn can_swap_equipped_bag_into_only_empty_slot_provided_by_itself_should_return_true() {
    let mut inv = Inventory::with_empty();

    inv.replace_loadout_item(EquipSlot::Armor(ArmorSlot::Bag1), Some(get_test_bag(18)));

    fill_inv_slots(&mut inv, 35);

    let result = inv.can_swap(InvSlotId::new(15, 17), EquipSlot::Armor(ArmorSlot::Bag1));

    assert!(result);
}

#[test]
fn unequip_items_both_hands() {
    let msm = &MaterialStatManifest::load().read();
    let ability_map = &AbilityMap::load().read();
    let mut inv = Inventory::with_empty();

    let sword = Item::new_from_asset_expect("common.items.weapons.sword.starter");

    inv.replace_loadout_item(
        EquipSlot::ActiveMainhand,
        Some(sword.duplicate(ability_map, msm)),
    );
    inv.replace_loadout_item(
        EquipSlot::InactiveMainhand,
        Some(sword.duplicate(ability_map, msm)),
    );

    // Fill all inventory slots except one
    fill_inv_slots(&mut inv, 17);

    let result = inv.unequip(EquipSlot::ActiveMainhand);
    // We have space in the inventory, so this should have unequipped
    assert_eq!(None, inv.equipped(EquipSlot::ActiveMainhand));
    assert_eq!(18, inv.populated_slots());
    assert!(result.is_ok());

    let result = inv.unequip(EquipSlot::InactiveMainhand).unwrap_err();
    assert_eq!(SlotError::InventoryFull, result);

    // There is no more space in the inventory, so this should still be equipped
    assert_eq!(&sword, inv.equipped(EquipSlot::InactiveMainhand).unwrap());

    // Verify inventory
    assert_eq!(inv.slots[17], Some(sword));
    assert_eq!(inv.free_slots(), 0);
}

#[test]
fn equip_replace_already_equipped_item() {
    let msm = &MaterialStatManifest::load().read();
    let ability_map = &AbilityMap::load().read();
    let boots = Item::new_from_asset_expect("common.items.testing.test_boots");

    let starting_sandles = Some(Item::new_from_asset_expect(
        "common.items.armor.misc.foot.sandals",
    ));

    let mut inv = Inventory::with_empty();
    inv.push(boots.duplicate(ability_map, msm)).unwrap();
    inv.replace_loadout_item(
        EquipSlot::Armor(ArmorSlot::Feet),
        starting_sandles
            .as_ref()
            .map(|item| item.duplicate(ability_map, msm)),
    );

    let _ = inv.equip(InvSlotId::new(0, 0));

    // We should now have the testing boots equipped
    assert_eq!(
        &boots,
        inv.equipped(EquipSlot::Armor(ArmorSlot::Feet)).unwrap()
    );

    // Verify inventory
    assert_eq!(&inv.slots[0], &starting_sandles,);
    assert_eq!(inv.populated_slots(), 1);
}

/// Regression test for a panic that occurred when swapping an equipped bag
/// for a bag that exists in an inventory slot that will no longer exist
/// after equipping it (because the equipped bag is larger)
#[test]
fn equip_equipping_smaller_bag_from_last_slot_of_big_bag() {
    let mut inv = Inventory::with_empty();

    const LARGE_BAG_ID: &str = "common.items.testing.test_bag_18_slot";
    let small_bag = get_test_bag(9);
    let large_bag = Item::new_from_asset_expect(LARGE_BAG_ID);

    assert!(
        inv.loadout
            .swap(EquipSlot::Armor(ArmorSlot::Bag1), Some(large_bag))
            .is_none()
    );

    inv.insert_at(InvSlotId::new(15, 15), small_bag).unwrap();

    let result = inv.swap(
        Slot::Equip(EquipSlot::Armor(ArmorSlot::Bag1)),
        Slot::Inventory(InvSlotId::new(15, 15)),
    );

    assert_eq!(
        inv.get(InvSlotId::new(0, 0)).unwrap().item_definition_id(),
        ItemDefinitionId::Simple(LARGE_BAG_ID)
    );
    assert!(result.is_empty());
}

#[test]
fn unequip_unequipping_bag_into_its_own_slot_with_no_other_free_slots_returns_one_item() {
    let mut inv = Inventory::with_empty();
    let bag = get_test_bag(9);

    assert!(
        inv.loadout
            .swap(EquipSlot::Armor(ArmorSlot::Bag1), Some(bag))
            .is_none()
    );

    // Fill all inventory built-in slots
    fill_inv_slots(&mut inv, 18);

    let result =
        inv.swap_inventory_loadout(InvSlotId::new(15, 0), EquipSlot::Armor(ArmorSlot::Bag1));

    assert_eq!(result.len(), 1);
    // Because the slot the bag was swapped with no longer exists as it was provided
    // by itself, the bag is returned to the caller
    assert_eq!(
        result[0].item_definition_id(),
        ItemDefinitionId::Simple("common.items.testing.test_bag")
    );
}

#[test]
fn equip_one_bag_equipped_equip_second_bag() {
    let msm = &MaterialStatManifest::load().read();
    let ability_map = &AbilityMap::load().read();
    let mut inv = Inventory::with_empty();

    let bag = get_test_bag(9);
    assert!(
        inv.loadout
            .swap(
                EquipSlot::Armor(ArmorSlot::Bag1),
                Some(bag.duplicate(ability_map, msm)),
            )
            .is_none()
    );

    inv.push(bag).unwrap();

    let _ = inv.equip(InvSlotId::new(0, 0));

    assert!(inv.equipped(EquipSlot::Armor(ArmorSlot::Bag2)).is_some());
}

#[test]
fn free_after_swap_equipped_item_has_more_slots() {
    let mut inv = Inventory::with_empty();

    let bag = get_test_bag(18);
    assert!(
        inv.loadout
            .swap(EquipSlot::Armor(ArmorSlot::Bag1), Some(bag))
            .is_none()
    );

    let small_bag = get_test_bag(9);
    inv.push(small_bag).unwrap();

    // Fill all remaining slots
    fill_inv_slots(&mut inv, 35);

    let result = inv.free_after_swap(EquipSlot::Armor(ArmorSlot::Bag1), InvSlotId::new(0, 0));

    // 18 inv slots + 9 bag slots - 36 used slots -
    assert_eq!(-9, result);
}

#[test]
fn free_after_swap_equipped_item_has_less_slots() {
    let mut inv = Inventory::with_empty();

    let bag = get_test_bag(9);
    assert!(
        inv.loadout
            .swap(EquipSlot::Armor(ArmorSlot::Bag1), Some(bag))
            .is_none()
    );

    let small_bag = get_test_bag(18);
    inv.push(small_bag).unwrap();

    // Fill all slots except the last one
    fill_inv_slots(&mut inv, 26);

    let result = inv.free_after_swap(EquipSlot::Armor(ArmorSlot::Bag1), InvSlotId::new(0, 0));

    // 18 inv slots + 18 bag slots - 27 used slots
    assert_eq!(9, result);
}

#[test]
fn free_after_swap_equipped_item_with_slots_swapped_with_empty_inv_slot() {
    let mut inv = Inventory::with_empty();

    let bag = get_test_bag(9);
    assert!(
        inv.loadout
            .swap(EquipSlot::Armor(ArmorSlot::Bag1), Some(bag))
            .is_none()
    );

    // Add 5 items to the inventory
    fill_inv_slots(&mut inv, 5);

    let result = inv.free_after_swap(EquipSlot::Armor(ArmorSlot::Bag1), InvSlotId::new(0, 10));

    // 18 inv slots - 5 used slots - 1 slot for unequipped item
    assert_eq!(12, result);
}

#[test]
fn free_after_swap_inv_item_with_slots_swapped_with_empty_equip_slot() {
    let mut inv = Inventory::with_empty();

    inv.push(get_test_bag(9)).unwrap();

    // Add 5 items to the inventory
    fill_inv_slots(&mut inv, 5);

    let result = inv.free_after_swap(EquipSlot::Armor(ArmorSlot::Bag1), InvSlotId::new(0, 0));

    // 18 inv slots + 9 bag slots - 5 used slots
    assert_eq!(22, result);
}

#[test]
fn free_after_swap_inv_item_without_slots_swapped_with_empty_equip_slot() {
    let mut inv = Inventory::with_empty();

    let boots = Item::new_from_asset_expect("common.items.testing.test_boots");
    inv.push(boots).unwrap();

    // Add 5 items to the inventory
    fill_inv_slots(&mut inv, 5);

    let result = inv.free_after_swap(EquipSlot::Armor(ArmorSlot::Feet), InvSlotId::new(0, 0));

    // 18 inv slots - 5 used slots
    assert_eq!(13, result);
}

// This test is a regression test for a bug that crashed the server when
// swapping an equipped item providing slots with an item that does not
// provide slots.
#[test]
fn backpack_crash() {
    let mut inv = Inventory::with_empty();

    let backpack = Item::new_from_asset_expect("common.items.armor.misc.back.backpack");
    inv.loadout
        .swap(EquipSlot::Armor(ArmorSlot::Back), Some(backpack));

    fill_inv_slots(&mut inv, 35);

    let cape = Item::new_from_asset_expect("common.items.armor.misc.back.admin");
    assert!(inv.push(cape).is_ok());

    let returned_items =
        inv.swap_inventory_loadout(InvSlotId::new(9, 17), EquipSlot::Armor(ArmorSlot::Back));
    assert_eq!(18, returned_items.len());
    assert_eq!(
        ItemDefinitionId::Simple("common.items.armor.misc.back.backpack"),
        returned_items[0].item_definition_id()
    );
}

fn fill_inv_slots(inv: &mut Inventory, items: u16) {
    let msm = &MaterialStatManifest::load().read();
    let ability_map = &AbilityMap::load().read();
    let boots = Item::new_from_asset_expect("common.items.testing.test_boots");
    for _ in 0..items {
        inv.push(boots.duplicate(ability_map, msm)).unwrap();
    }
}
