use crate::{comp, comp::item};
use comp::{Inventory, Loadout};

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

// TODO: shouldn't need this
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

#[must_use]
fn loadout_insert(
    equip_slot: EquipSlot,
    item: item::Item,
    loadout: &mut Loadout,
) -> Option<item::Item> {
    loadout_replace(equip_slot, Some(item), loadout)
}

pub fn loadout_remove(equip_slot: EquipSlot, loadout: &mut Loadout) -> Option<item::Item> {
    loadout_replace(equip_slot, None, loadout)
}

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
            inventory
                .insert(inventory_slot, item)
                .unwrap_or_else(|i| Some(i))
        } else {
            inventory.remove(inventory_slot)
        };
        // Put item from the inventory in loadout
        if let Some(item) = from_inv {
            loadout_insert(equip_slot, item, loadout).unwrap_none(); // Can never fail
        }
    }
}

fn swap_loadout(slot_a: EquipSlot, slot_b: EquipSlot, loadout: &mut Loadout) {
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

// Should this report if a change actually occurred? (might be useful when
// minimizing network use)
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

pub fn unequip(slot: EquipSlot, inventory: &mut Inventory, loadout: &mut Loadout) {
    loadout_remove(slot, loadout) // Remove item from loadout
        .and_then(|i| inventory.push(i)) // Insert into inventory
        .and_then(|i| loadout_insert(slot, i, loadout)) // If that fails put back in loadout
        .unwrap_none(); // Never fails
}
